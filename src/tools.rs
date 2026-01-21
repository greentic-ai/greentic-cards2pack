use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

pub fn resolve_greentic_pack_bin(override_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }

    if let Some(path) = env::var_os("GREENTIC_PACK_BIN") {
        return Ok(PathBuf::from(path));
    }

    which::which("greentic-pack").context("greentic-pack not found on PATH")
}

pub struct BuildOutput {
    pub stdout: String,
    pub stderr: String,
}

pub fn run_greentic_pack_new(bin: &Path, out_dir: &Path, name: &str) -> Result<()> {
    let status = Command::new(bin)
        .arg("new")
        .arg("--dir")
        .arg(out_dir)
        .arg(name)
        .status()
        .with_context(|| format!("failed to run greentic-pack new for {}", out_dir.display()))?;

    if !status.success() {
        bail!("greentic-pack new failed for {}", out_dir.display());
    }

    Ok(())
}

pub fn run_greentic_pack_doctor(bin: &Path, workspace: &Path) -> Result<()> {
    let status = Command::new(bin)
        .arg("doctor")
        .arg("--in")
        .arg(workspace)
        .status()
        .with_context(|| {
            format!(
                "failed to run greentic-pack doctor for {}",
                workspace.display()
            )
        })?;

    if !status.success() {
        bail!("greentic-pack doctor failed for {}", workspace.display());
    }

    Ok(())
}

pub fn run_greentic_pack_update(bin: &Path, workspace: &Path) -> Result<()> {
    let status = Command::new(bin)
        .arg("update")
        .arg("--in")
        .arg(workspace)
        .status()
        .with_context(|| {
            format!(
                "failed to run greentic-pack update for {}",
                workspace.display()
            )
        })?;

    if !status.success() {
        bail!("greentic-pack update failed for {}", workspace.display());
    }

    Ok(())
}

pub fn run_greentic_pack_components(bin: &Path, workspace: &Path) -> Result<()> {
    let status = Command::new(bin)
        .arg("components")
        .arg("--in")
        .arg(workspace)
        .status()
        .with_context(|| {
            format!(
                "failed to run greentic-pack components for {}",
                workspace.display()
            )
        })?;

    if !status.success() {
        bail!(
            "greentic-pack components failed for {}",
            workspace.display()
        );
    }

    Ok(())
}

pub fn run_greentic_pack_resolve(bin: &Path, workspace: &Path) -> Result<()> {
    let status = Command::new(bin)
        .arg("resolve")
        .arg("--in")
        .arg(workspace)
        .status()
        .with_context(|| {
            format!(
                "failed to run greentic-pack resolve for {}",
                workspace.display()
            )
        })?;

    if !status.success() {
        bail!("greentic-pack resolve failed for {}", workspace.display());
    }

    Ok(())
}

pub fn run_greentic_pack_build(
    bin: &Path,
    workspace: &Path,
    gtpack_out: &Path,
    verbose: bool,
) -> Result<BuildOutput> {
    let mut command = Command::new(bin);
    command
        .arg("build")
        .arg("--in")
        .arg(workspace)
        .arg("--gtpack-out")
        .arg(gtpack_out);

    if verbose {
        eprintln!(
            "Running: {} build --in {} --gtpack-out {}",
            bin.display(),
            workspace.display(),
            gtpack_out.display()
        );
    }

    let output = command
        .output()
        .with_context(|| format!("failed to spawn greentic-pack at {}", bin.display()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if verbose {
        if !stdout.trim().is_empty() {
            eprintln!("greentic-pack stdout:\n{}", stdout.trim_end());
        }
        if !stderr.trim().is_empty() {
            eprintln!("greentic-pack stderr:\n{}", stderr.trim_end());
        }
    }

    if !output.status.success() {
        let tail = tail_lines(&stderr, 20);
        bail!("greentic-pack failed (status {}):\n{}", output.status, tail);
    }

    Ok(BuildOutput { stdout, stderr })
}

fn tail_lines(input: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = input.lines().collect();
    if lines.len() <= max_lines {
        return input.trim_end().to_string();
    }
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}
