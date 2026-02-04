use std::fs;
use std::io::Write;

use greentic_cards2pack::qa_integration::{
    Prompt2FlowConfig, PromptIntent, PromptLimits, PromptMode, PromptRoute, Source,
    build_prompt2flow_config, prompt_limits_from_arg,
};
use serde_json::json;
use tempfile::NamedTempFile;

#[test]
fn loads_prompt_json_with_default_limits() {
    let fixture = json!({
        "version": 1,
        "mode": {
            "require_prefix": true,
            "prefixes": ["go:", "/"],
            "min_score": 0.35,
            "min_gap": 0.1,
            "top_k": 3
        },
        "intents": [
            {
                "id": "setup",
                "title": "Setup Flow",
                "route": { "flow": "setup", "node": "start" },
                "examples": ["go: setup"],
                "keywords": ["setup"],
                "anchors": ["setup"]
            }
        ]
    });
    let mut tmp = NamedTempFile::new().expect("create fixture");
    write!(tmp, "{}", fixture).expect("write fixture");

    let config = build_prompt2flow_config(Source::JsonFile(tmp.path()), PromptLimits::default())
        .expect("load prompt config");
    assert_eq!(config.version, 1);
    assert_eq!(config.intents.len(), 1);
    assert_eq!(config.intents[0].id, "setup");
}

#[test]
fn rejects_intent_limit() {
    let limits = PromptLimits {
        max_intents: 1,
        ..PromptLimits::default()
    };
    let config = Prompt2FlowConfig {
        version: 1,
        mode: PromptMode {
            require_prefix: true,
            prefixes: vec!["go:".to_string()],
            min_score: 0.35,
            min_gap: 0.1,
            top_k: 3,
        },
        intents: vec![
            PromptIntent {
                id: "one".to_string(),
                title: "One".to_string(),
                route: PromptRoute {
                    flow: "flow".to_string(),
                    node: Some("start".to_string()),
                },
                examples: vec!["example".to_string()],
                keywords: vec![],
                anchors: vec![],
            },
            PromptIntent {
                id: "two".to_string(),
                title: "Two".to_string(),
                route: PromptRoute {
                    flow: "flow".to_string(),
                    node: None,
                },
                examples: vec!["example".to_string()],
                keywords: vec![],
                anchors: vec![],
            },
        ],
    };
    assert!(config.validate_limits(&limits).is_err());
}

#[test]
fn parses_prompt_limits_inline_json() {
    let raw = r#"{"max_intents": 60, "max_examples_per_intent": 25, "max_keywords": 35, "max_anchors": 12}"#;
    let limits = prompt_limits_from_arg(Some(raw))
        .expect("parse limits")
        .expect("limit present");
    assert_eq!(limits.max_intents, 60);
    assert_eq!(limits.max_examples_per_intent, 25);
    assert_eq!(limits.max_keywords, 35);
    assert_eq!(limits.max_anchors, 12);
}

#[test]
fn parses_prompt_limits_from_file() {
    let limits = PromptLimits {
        max_intents: 45,
        max_examples_per_intent: 15,
        max_keywords: 20,
        max_anchors: 8,
    };
    let contents = serde_json::to_string(&limits).expect("serialize limits");
    let temp = NamedTempFile::new().expect("create limits file");
    fs::write(temp.path(), contents).expect("write limits file");

    let parsed =
        prompt_limits_from_arg(Some(temp.path().to_string_lossy().as_ref())).expect("parse file");
    assert_eq!(parsed, Some(limits));
}
