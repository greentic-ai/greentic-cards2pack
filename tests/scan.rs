use std::fs;
use std::path::{Path, PathBuf};

use greentic_cards2pack::cli::GroupBy;
use greentic_cards2pack::ir::RouteTarget;
use greentic_cards2pack::scan::{ScanConfig, scan_cards};
use tempfile::TempDir;

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cards")
}

fn copy_fixture(rel: &str, dest_root: &Path) -> PathBuf {
    let source = fixtures_root().join(rel);
    let dest = dest_root.join(rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::copy(&source, &dest).unwrap();
    dest
}

#[test]
fn resolves_card_id_and_flow_from_actions() {
    let tmp = TempDir::new().unwrap();
    copy_fixture("simple_submit.json", tmp.path());

    let config = ScanConfig {
        cards_dir: tmp.path().to_path_buf(),
        group_by: None,
        default_flow: None,
        strict: true,
    };

    let manifest = scan_cards(&config).unwrap();
    let flow = manifest
        .flows
        .iter()
        .find(|flow| flow.flow_name == "hrAssist")
        .unwrap();
    let card = &flow.cards[0];

    assert_eq!(card.card_id, "HR-CARD-00");
    assert_eq!(card.flow_name, "hrAssist");
    assert_eq!(card.actions.len(), 1);

    let action = &card.actions[0];
    assert_eq!(action.action_type, "Action.Submit");
    assert_eq!(action.title.as_deref(), Some("Next"));
    match action.target.as_ref().unwrap() {
        RouteTarget::Step(value) => assert_eq!(value, "collect"),
        RouteTarget::CardId(_) => panic!("expected step target"),
    }
}

#[test]
fn uses_folder_grouping_for_flow() {
    let cards_dir = fixtures_root().join("folder_grouping/cards");

    let config = ScanConfig {
        cards_dir: cards_dir.clone(),
        group_by: Some(GroupBy::Folder),
        default_flow: None,
        strict: true,
    };

    let manifest = scan_cards(&config).unwrap();
    let flow = manifest
        .flows
        .iter()
        .find(|flow| flow.flow_name == "hrAssist")
        .unwrap();
    let card = &flow.cards[0];

    assert_eq!(card.card_id, "HR-CARD-00");
    assert_eq!(card.flow_name, "hrAssist");
}

#[test]
fn uses_filename_for_card_id() {
    let cards_dir = fixtures_root().join("filename_fallback");

    let config = ScanConfig {
        cards_dir: cards_dir.clone(),
        group_by: None,
        default_flow: Some("ops".to_string()),
        strict: true,
    };

    let manifest = scan_cards(&config).unwrap();
    let flow = manifest
        .flows
        .iter()
        .find(|flow| flow.flow_name == "ops")
        .unwrap();
    let card = &flow.cards[0];

    assert_eq!(card.card_id, "IT-CARD-99");
}

#[test]
fn inconsistent_cardid_strict_errors() {
    let tmp = TempDir::new().unwrap();
    copy_fixture("inconsistent_cardid.json", tmp.path());

    let config = ScanConfig {
        cards_dir: tmp.path().to_path_buf(),
        group_by: None,
        default_flow: None,
        strict: true,
    };

    let result = scan_cards(&config);
    assert!(result.is_err());
}

#[test]
fn ignores_non_card_json_with_warning() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("note.json"), "{ \"foo\": 1 }").unwrap();

    let config = ScanConfig {
        cards_dir: tmp.path().to_path_buf(),
        group_by: None,
        default_flow: Some("ops".to_string()),
        strict: false,
    };

    let manifest = scan_cards(&config).unwrap();
    assert!(
        manifest
            .warnings
            .iter()
            .any(|w| w.message.contains("non-card"))
    );
    assert!(manifest.flows.is_empty());
}
