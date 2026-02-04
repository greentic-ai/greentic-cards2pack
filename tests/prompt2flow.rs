mod support;

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn prompt_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/prompt2flow")
}

fn copy_prompt_cards(target: &Path) {
    support::copy_fixture_cards(&prompt_fixture_root().join("cards"), target);
}

fn first_node_name(flow_contents: &str) -> Option<String> {
    let marker = "nodes:\n";
    let start = flow_contents.find(marker)? + marker.len();
    for line in flow_contents[start..].lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('-') {
            continue;
        }
        if let Some(stripped) = trimmed.strip_suffix(':') {
            return Some(stripped.to_string());
        }
        break;
    }
    None
}

#[test]
fn generate_without_prompt_does_not_add_prompt_node() {
    let tmp = TempDir::new().unwrap();
    let cards_dir = tmp.path().join("cards");
    let out_dir = tmp.path().join("workspace");
    fs::create_dir_all(&cards_dir).unwrap();
    copy_prompt_cards(&cards_dir);

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let greentic_pack = support::create_fake_greentic_pack(&bin_dir);

    cargo_bin_cmd!("greentic-cards2pack")
        .arg("generate")
        .arg("--cards")
        .arg(&cards_dir)
        .arg("--out")
        .arg(&out_dir)
        .arg("--name")
        .arg("prompt-demo")
        .arg("--greentic-pack-bin")
        .arg(greentic_pack)
        .assert()
        .success();

    assert!(!out_dir.join("assets/config/prompt2flow.json").exists());
    let flow = fs::read_to_string(out_dir.join("flows/main.ygtc")).unwrap();
    assert!(!flow.contains("prompt2flow"));
}

#[test]
fn generate_with_prompt_emits_prompt_config_and_node() {
    let tmp = TempDir::new().unwrap();
    let cards_dir = tmp.path().join("cards");
    let out_dir = tmp.path().join("workspace");
    fs::create_dir_all(&cards_dir).unwrap();
    copy_prompt_cards(&cards_dir);

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let greentic_pack = support::create_fake_greentic_pack(&bin_dir);

    let answers = prompt_fixture_root().join("prompt2flow_answers.json");

    cargo_bin_cmd!("greentic-cards2pack")
        .arg("generate")
        .arg("--cards")
        .arg(&cards_dir)
        .arg("--out")
        .arg(&out_dir)
        .arg("--name")
        .arg("prompt-demo")
        .arg("--prompt")
        .arg("--prompt-json")
        .arg(&answers)
        .arg("--greentic-pack-bin")
        .arg(greentic_pack)
        .assert()
        .success();

    let prompt_config = out_dir.join("assets/config/prompt2flow.json");
    assert!(prompt_config.exists());

    let pack_yaml = fs::read_to_string(out_dir.join("pack.yaml")).unwrap();
    assert!(pack_yaml.contains("ai.greentic.component-prompt2flow"));

    let flow = fs::read_to_string(out_dir.join("flows/main.ygtc")).unwrap();
    assert_eq!(first_node_name(&flow), Some("prompt2flow".to_string()));
}
