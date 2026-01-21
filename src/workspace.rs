use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use walkdir::WalkDir;

use crate::cli::GenerateArgs;
use crate::diagnostics::{build_diagnostics, summarize, warning};
use crate::emit_flow::emit_flow;
use crate::graph::build_flow_graph;
use crate::ir::{FlowSummary, Manifest, Warning, WarningKind};
use crate::scan::{ScanConfig, scan_cards};
use crate::tools::{
    resolve_greentic_pack_bin, run_greentic_pack_build, run_greentic_pack_doctor,
    run_greentic_pack_new, run_greentic_pack_resolve, run_greentic_pack_update,
};

pub fn generate(args: &GenerateArgs) -> Result<()> {
    if !args.cards.is_dir() {
        bail!("cards directory does not exist: {}", args.cards.display());
    }

    let greentic_pack_bin = resolve_greentic_pack_bin(args.greentic_pack_bin.as_deref())?;
    let pack_yaml = args.out.join("pack.yaml");
    if !pack_yaml.exists() {
        run_greentic_pack_new(&greentic_pack_bin, &args.out, &args.name)?;
    }

    fs::create_dir_all(&args.out)
        .with_context(|| format!("failed to create workspace {}", args.out.display()))?;

    let assets_cards = args.out.join("assets").join("cards");
    let flows_dir = args.out.join("flows");
    let dist_dir = args.out.join("dist");
    let state_dir = args.out.join(".cards2pack");

    fs::create_dir_all(&assets_cards)
        .with_context(|| format!("failed to create {}", assets_cards.display()))?;
    fs::create_dir_all(&flows_dir)
        .with_context(|| format!("failed to create {}", flows_dir.display()))?;
    fs::create_dir_all(&dist_dir)
        .with_context(|| format!("failed to create {}", dist_dir.display()))?;
    fs::create_dir_all(&state_dir)
        .with_context(|| format!("failed to create {}", state_dir.display()))?;

    copy_cards(&args.cards, &assets_cards)?;
    ensure_readme(&args.out, &args.name)?;

    let scan_config = ScanConfig {
        cards_dir: assets_cards.clone(),
        group_by: args.group_by,
        default_flow: args.default_flow.clone(),
        strict: args.strict,
    };
    let mut manifest = scan_cards(&scan_config)?;

    let mut flow_paths = Vec::new();
    let mut readme_entries = Vec::new();
    for flow in &manifest.flows {
        let graph = build_flow_graph(flow, args.strict)?;
        if !graph.warnings.is_empty() {
            manifest.warnings.extend(graph.warnings.iter().cloned());
        }
        let (path, flow_warnings) = emit_flow(&graph, &args.out, args.strict)?;
        if !flow_warnings.is_empty() {
            manifest.warnings.extend(flow_warnings);
        }
        write_flow_resolve_sidecar(&path, &graph)?;
        let flow_path = path
            .strip_prefix(&args.out)
            .unwrap_or(&path)
            .display()
            .to_string();
        if !flow_paths.contains(&flow_path) {
            flow_paths.push(flow_path);
        }
        let entry = graph
            .nodes
            .values()
            .find(|node| !node.stub)
            .map(|node| node.name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        readme_entries.push((flow.flow_name.clone(), entry));
    }

    run_greentic_pack_update(&greentic_pack_bin, &args.out)?;
    update_readme(&args.out, &args.name, &readme_entries)?;

    if let Err(err) = run_greentic_flow_doctor(&args.out.join("flows")) {
        if args.strict {
            return Err(err);
        }
        manifest.warnings.push(warning(
            WarningKind::Validation,
            format!("greentic-flow doctor failed: {err}"),
        ));
    }

    if let Err(err) = run_greentic_pack_resolve(&greentic_pack_bin, &args.out) {
        if args.strict {
            return Err(err);
        }
        manifest.warnings.push(warning(
            WarningKind::Validation,
            format!("greentic-pack resolve failed: {err}"),
        ));
    }

    if let Err(err) = run_greentic_pack_doctor(&greentic_pack_bin, &args.out) {
        if args.strict {
            return Err(err);
        }
        manifest.warnings.push(warning(
            WarningKind::Validation,
            format!("greentic-pack doctor failed: {err}"),
        ));
    }

    let gtpack_out = dist_dir.join(format!("{}.gtpack", args.name));
    let build_output =
        run_greentic_pack_build(&greentic_pack_bin, &args.out, &gtpack_out, args.verbose)?;
    if !gtpack_out.exists()
        && let Some(path) = extract_gtpack_path(&build_output)
        && path.exists()
    {
        fs::copy(&path, &gtpack_out).with_context(|| {
            format!(
                "failed to copy greentic-pack output {} to {}",
                path.display(),
                gtpack_out.display()
            )
        })?;
    }

    let (gtpack_path, gtpack_warning) = ensure_named_gtpack(&dist_dir, &args.name)?;
    if let Some(warning) = gtpack_warning {
        manifest.warnings.push(warning);
    }

    let flow_summaries: Vec<FlowSummary> = manifest
        .flows
        .iter()
        .map(|flow| FlowSummary {
            flow_name: flow.flow_name.clone(),
            card_count: flow.cards.len(),
        })
        .collect();
    manifest.diagnostics = build_diagnostics(
        args.out.clone(),
        Some(gtpack_path.clone()),
        flow_paths.clone(),
        flow_summaries,
        manifest.flows.iter().map(|flow| flow.cards.len()).sum(),
        manifest.warnings.len(),
    );
    write_manifest(&state_dir, &manifest)?;

    println!("{}", summarize(&manifest.diagnostics, &manifest.warnings));

    Ok(())
}

fn copy_cards(cards_dir: &Path, dest_root: &Path) -> Result<()> {
    for entry in WalkDir::new(cards_dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let extension = path.extension().and_then(|ext| ext.to_str());
        if extension.is_none_or(|ext| !ext.eq_ignore_ascii_case("json")) {
            continue;
        }

        let rel = path
            .strip_prefix(cards_dir)
            .with_context(|| format!("failed to strip prefix for {}", path.display()))?;
        let dest_path = dest_root.join(rel);
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(path, &dest_path).with_context(|| format!("failed to copy {}", path.display()))?;
    }

    Ok(())
}

fn ensure_readme(workspace: &Path, name: &str) -> Result<()> {
    let readme_path = workspace.join("README.md");
    if readme_path.exists() {
        return Ok(());
    }

    let contents = format!(
        "# {name}\n\nGenerated by greentic-cards2pack.\n",
        name = name
    );

    fs::write(&readme_path, contents)
        .with_context(|| format!("failed to write {}", readme_path.display()))?;

    Ok(())
}

fn write_manifest(state_dir: &Path, manifest: &Manifest) -> Result<()> {
    let path = state_dir.join("manifest.json");
    let json = serde_json::to_vec_pretty(&manifest)?;
    let mut file =
        fs::File::create(&path).with_context(|| format!("failed to write {}", path.display()))?;
    file.write_all(&json)?;
    file.write_all(b"\n")?;

    Ok(())
}

fn update_readme(workspace: &Path, name: &str, entries: &[(String, String)]) -> Result<()> {
    let readme_path = workspace.join("README.md");
    let existing = if readme_path.exists() {
        fs::read_to_string(&readme_path)
            .with_context(|| format!("failed to read {}", readme_path.display()))?
    } else {
        format!("# {name}\n\nGenerated by greentic-cards2pack.\n")
    };

    let mut section = String::new();
    section.push_str("<!-- BEGIN GENERATED FLOWS (cards2pack) -->\n");
    section.push_str("## Generated Flows\n");
    if entries.is_empty() {
        section.push_str("- (none)\n");
    } else {
        for (flow, entry) in entries {
            section.push_str(&format!("- `{flow}` entry: `{entry}`\n"));
        }
    }
    section.push_str("<!-- END GENERATED FLOWS (cards2pack) -->\n");

    let updated = replace_marked_section(
        &existing,
        "<!-- BEGIN GENERATED FLOWS (cards2pack) -->",
        "<!-- END GENERATED FLOWS (cards2pack) -->",
        &section,
    );

    fs::write(&readme_path, updated)
        .with_context(|| format!("failed to write {}", readme_path.display()))?;

    Ok(())
}

fn run_greentic_flow_doctor(flows_dir: &Path) -> Result<()> {
    if !flows_dir.is_dir() {
        return Ok(());
    }

    let status = std::process::Command::new("greentic-flow")
        .arg("doctor")
        .arg(flows_dir)
        .status()
        .with_context(|| {
            format!(
                "failed to run greentic-flow doctor for {}",
                flows_dir.display()
            )
        })?;

    if !status.success() {
        bail!("greentic-flow doctor failed for {}", flows_dir.display());
    }

    Ok(())
}

fn replace_marked_section(existing: &str, start: &str, end: &str, section: &str) -> String {
    let start_pos = existing.find(start);
    let end_pos = existing.find(end);

    match (start_pos, end_pos) {
        (Some(start_pos), Some(end_pos)) if end_pos > start_pos => {
            let after_end = existing[end_pos..].find('\n').map(|idx| end_pos + idx + 1);
            let before = &existing[..start_pos];
            let after = after_end.map_or("", |idx| &existing[idx..]);
            format!("{before}{section}{after}")
        }
        _ => {
            if existing.trim().is_empty() {
                section.to_string()
            } else {
                format!("{existing}\n{section}")
            }
        }
    }
}

fn extract_gtpack_path(build_output: &crate::tools::BuildOutput) -> Option<PathBuf> {
    for line in build_output
        .stdout
        .lines()
        .chain(build_output.stderr.lines())
    {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("wrote ") {
            let candidate = rest.trim();
            if candidate.ends_with(".gtpack") {
                return Some(PathBuf::from(candidate));
            }
        }
    }
    None
}

fn write_flow_resolve_sidecar(flow_path: &Path, graph: &crate::graph::FlowGraph) -> Result<()> {
    let mut nodes = serde_json::Map::new();
    for node in graph.nodes.keys() {
        nodes.insert(
            node.clone(),
            serde_json::json!({
                "source": {
                    "kind": "oci",
                    "ref": "oci://ghcr.io/greentic-ai/components/component-adaptive-card:latest"
                }
            }),
        );
    }

    let payload = serde_json::json!({
        "schema_version": 1,
        "flow": flow_path.file_name().and_then(|name| name.to_str()).unwrap_or("main.ygtc"),
        "nodes": nodes
    });

    let sidecar_path = flow_path.with_extension("ygtc.resolve.json");
    fs::write(&sidecar_path, serde_json::to_string_pretty(&payload)?)
        .with_context(|| format!("failed to write {}", sidecar_path.display()))?;

    let summary_path = flow_path.with_extension("ygtc.resolve.summary.json");
    if summary_path.exists() {
        let _ = fs::remove_file(&summary_path);
    }

    Ok(())
}

fn ensure_named_gtpack(dist_dir: &Path, name: &str) -> Result<(PathBuf, Option<Warning>)> {
    let target_name = format!("{name}.gtpack");
    let target_path = dist_dir.join(&target_name);
    if target_path.exists() {
        return Ok((target_path, None));
    }

    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in fs::read_dir(dist_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("gtpack") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let replace = newest
            .as_ref()
            .map(|(_, time)| modified > *time)
            .unwrap_or(true);
        if replace {
            newest = Some((path, modified));
        }
    }

    let (source, _) =
        newest.ok_or_else(|| anyhow!("no .gtpack file found in {}", dist_dir.display()))?;
    let normalized_warning = warning(
        WarningKind::PackOutput,
        format!(
            "normalized gtpack output from {} to {}",
            source.display(),
            target_path.display()
        ),
    );

    if source != target_path && fs::rename(&source, &target_path).is_err() {
        fs::copy(&source, &target_path)?;
        fs::remove_file(&source)?;
    }

    Ok((target_path, Some(normalized_warning)))
}
