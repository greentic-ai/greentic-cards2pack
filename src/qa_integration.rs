use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const PROMPT_SPEC_REL: &str = "qa/prompt2flow.form.json";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PromptLimits {
    pub max_intents: usize,
    pub max_examples_per_intent: usize,
    pub max_keywords: usize,
    pub max_anchors: usize,
}

impl Default for PromptLimits {
    fn default() -> Self {
        Self {
            max_intents: 50,
            max_examples_per_intent: 20,
            max_keywords: 30,
            max_anchors: 10,
        }
    }
}

pub fn prompt_limits_from_arg(raw: Option<&str>) -> Result<Option<PromptLimits>> {
    let raw = match raw {
        Some(value) => value.trim(),
        None => return Ok(None),
    };
    if raw.is_empty() {
        return Ok(None);
    }
    if let Ok(parsed) = serde_json::from_str::<PromptLimits>(raw) {
        return Ok(Some(parsed));
    }
    let path = Path::new(raw);
    let contents = fs::read_to_string(path)
        .with_context(|| format!("read prompt limits from {}", path.display()))?;
    let limits: PromptLimits =
        serde_json::from_str(&contents).context("parse prompt limits JSON")?;
    Ok(Some(limits))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Prompt2FlowConfig {
    pub version: u8,
    pub mode: PromptMode,
    pub intents: Vec<PromptIntent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptMode {
    pub require_prefix: bool,
    pub prefixes: Vec<String>,
    pub min_score: f64,
    pub min_gap: f64,
    pub top_k: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptIntent {
    pub id: String,
    pub title: String,
    pub route: PromptRoute,
    pub examples: Vec<String>,
    pub keywords: Vec<String>,
    pub anchors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptRoute {
    pub flow: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
}

pub enum Source<'a> {
    JsonFile(&'a Path),
    Interactive,
}

impl Prompt2FlowConfig {
    pub fn validate_limits(&self, limits: &PromptLimits) -> Result<()> {
        if self.intents.len() > limits.max_intents {
            bail!(
                "intents exceed max {} (use --prompt-limits to raise)",
                limits.max_intents
            );
        }
        for intent in &self.intents {
            if intent.examples.len() > limits.max_examples_per_intent {
                bail!(
                    "intent '{}' has more than {} examples (use --prompt-limits to raise)",
                    intent.id,
                    limits.max_examples_per_intent
                );
            }
            if intent.keywords.len() > limits.max_keywords {
                bail!(
                    "intent '{}' has more than {} keywords (use --prompt-limits to raise)",
                    intent.id,
                    limits.max_keywords
                );
            }
            if intent.anchors.len() > limits.max_anchors {
                bail!(
                    "intent '{}' has more than {} anchors (use --prompt-limits to raise)",
                    intent.id,
                    limits.max_anchors
                );
            }
        }
        Ok(())
    }
}

pub fn build_prompt2flow_config(source: Source, limits: PromptLimits) -> Result<Prompt2FlowConfig> {
    match source {
        Source::JsonFile(path) => {
            let contents =
                fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
            let config: Prompt2FlowConfig =
                serde_json::from_str(&contents).context("parse prompt2flow json")?;
            config.validate_limits(&limits)?;
            Ok(config)
        }
        Source::Interactive => interactive_config(&limits),
    }
}

pub fn persist_prompt2flow_config(config: &Prompt2FlowConfig, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create directory {}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(config).context("serialize prompt2flow")?;
    fs::write(target, serialized).with_context(|| format!("write {}", target.display()))?;
    println!("Saved prompt2flow config to {}", target.display());
    Ok(())
}

fn interactive_config(limits: &PromptLimits) -> Result<Prompt2FlowConfig> {
    run_prompt2flow_wizard(limits)
}

#[derive(Deserialize)]
struct QaFormSpec {
    questions: Vec<QaQuestion>,
}

#[derive(Deserialize)]
struct QaQuestion {
    id: String,
    #[serde(rename = "type")]
    _kind: String,
    title: Option<String>,
    description: Option<String>,
    #[serde(default)]
    _required: bool,
    default: Option<Value>,
}

fn run_prompt2flow_wizard(limits: &PromptLimits) -> Result<Prompt2FlowConfig> {
    if limits.max_intents == 0 {
        bail!("prompt2flow requires at least one intent (set --prompt-limits to raise the cap)");
    }

    let spec = load_prompt_spec()?;
    println!("Prompt2Flow configuration wizard (greentic-qa form).");

    let require_prefix =
        ask_bool_question(find_question(&spec.questions, "mode.require_prefix")?, true)?;
    let prefixes = ask_prefix_question(find_question(&spec.questions, "mode.prefixes")?)?;
    let min_score = ask_float_question(find_question(&spec.questions, "mode.min_score")?, 0.35)?;
    let min_gap = ask_float_question(find_question(&spec.questions, "mode.min_gap")?, 0.10)?;
    let top_k = ask_usize_question(find_question(&spec.questions, "mode.top_k")?, 3, 1, 10)?;
    let intents = ask_intents_question(find_question(&spec.questions, "intents")?, limits)?;

    let config = Prompt2FlowConfig {
        version: 1,
        mode: PromptMode {
            require_prefix,
            prefixes,
            min_score,
            min_gap,
            top_k,
        },
        intents,
    };
    config.validate_limits(limits)?;
    Ok(config)
}

fn load_prompt_spec() -> Result<QaFormSpec> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(PROMPT_SPEC_REL);
    let contents =
        fs::read_to_string(&path).with_context(|| format!("read form spec {}", path.display()))?;
    serde_json::from_str(&contents).context("parse prompt2flow form spec")
}

fn find_question<'a>(questions: &'a [QaQuestion], id: &str) -> Result<&'a QaQuestion> {
    questions
        .iter()
        .find(|question| question.id == id)
        .ok_or_else(|| anyhow::anyhow!("missing question '{}' in spec", id))
}

fn ask_bool_question(question: &QaQuestion, fallback: bool) -> Result<bool> {
    display_question(question);
    let default = question
        .default
        .as_ref()
        .and_then(Value::as_bool)
        .unwrap_or(fallback);
    prompt_bool(&question_prompt(question), default)
}

fn ask_float_question(question: &QaQuestion, fallback: f64) -> Result<f64> {
    display_question(question);
    let default = question
        .default
        .as_ref()
        .and_then(Value::as_f64)
        .unwrap_or(fallback);
    prompt_float(&question_prompt(question), default)
}

fn ask_usize_question(
    question: &QaQuestion,
    fallback: usize,
    min: usize,
    max: usize,
) -> Result<usize> {
    display_question(question);
    let default = question
        .default
        .as_ref()
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(fallback);
    prompt_usize(&question_prompt(question), default, min, max)
}

fn ask_prefix_question(question: &QaQuestion) -> Result<Vec<String>> {
    display_question(question);
    let default = question
        .default
        .as_ref()
        .and_then(Value::as_str)
        .unwrap_or("[]");
    loop {
        let raw = prompt_string_with_default(&question_prompt(question), default)?;
        match serde_json::from_str::<Vec<String>>(&raw) {
            Ok(list) if !list.is_empty() => return Ok(list),
            Ok(_) => {
                println!("Please provide at least one prefix.");
                continue;
            }
            Err(err) => {
                println!("Invalid JSON array: {}", err);
                continue;
            }
        }
    }
}

fn ask_intents_question(question: &QaQuestion, limits: &PromptLimits) -> Result<Vec<PromptIntent>> {
    display_question(question);
    println!("Enter a JSON array of intents. Prefix the input with '@' to read from a file.");
    loop {
        let raw = prompt_line("Intents JSON: ")?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            println!("Intent list cannot be empty.");
            continue;
        }
        let content = if let Some(path) = trimmed.strip_prefix('@') {
            fs::read_to_string(path).with_context(|| format!("read intents from {}", path))?
        } else {
            trimmed.to_string()
        };
        let intents: Vec<PromptIntent> =
            serde_json::from_str(&content).context("parse intents array as JSON")?;
        if intents.is_empty() {
            println!("Provide at least one intent.");
            continue;
        }
        if intents.len() > limits.max_intents {
            println!(
                "You may provide at most {} intents (use --prompt-limits to raise).",
                limits.max_intents
            );
            continue;
        }
        return Ok(intents);
    }
}

fn question_prompt(question: &QaQuestion) -> String {
    question
        .title
        .as_deref()
        .unwrap_or(&question.id)
        .to_string()
}

fn display_question(question: &QaQuestion) {
    println!("\n{}:", question_prompt(question));
    if let Some(description) = &question.description {
        println!("{}", description);
    }
}

fn prompt_string_with_default(prompt: &str, default: &str) -> Result<String> {
    let input = prompt_line(&format!("{prompt} [{default}]: "))?;
    if input.trim().is_empty() {
        Ok(default.to_string())
    } else {
        Ok(input)
    }
}

fn prompt_usize(prompt: &str, default: usize, min: usize, max: usize) -> Result<usize> {
    loop {
        let input = prompt_line(&format!("{}: ", prompt))?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(default.min(max).max(min));
        }
        match trimmed.parse::<usize>() {
            Ok(value) if value >= min && value <= max => return Ok(value),
            _ => println!("Enter a number between {} and {}.", min, max),
        }
    }
}

fn prompt_float(prompt: &str, default: f64) -> Result<f64> {
    loop {
        let input = prompt_line(&format!("{}: ", prompt))?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(default);
        }
        match trimmed.parse::<f64>() {
            Ok(value) if (0.0..=1.0).contains(&value) => return Ok(value),
            _ => println!("Enter a number between 0.0 and 1.0."),
        }
    }
}

fn prompt_bool(prompt: &str, default: bool) -> Result<bool> {
    loop {
        let default_hint = if default { "[Y/n]" } else { "[y/N]" };
        let input = prompt_line(&format!("{} {}: ", prompt, default_hint))?;
        let trimmed = input.trim().to_lowercase();
        match trimmed.as_str() {
            "" => return Ok(default),
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => println!("Answer 'y' or 'n'."),
        }
    }
}

fn prompt_line(prompt: &str) -> Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim_end().to_string())
}
