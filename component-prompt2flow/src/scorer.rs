use crate::jaccard::jaccard_score;
use serde::Deserialize;
use std::collections::HashSet;

pub struct Score {
    pub value: f64,
    pub matched_tokens: Vec<String>,
}

pub trait Scorer: Send + Sync {
    fn score(&self, query: &HashSet<String>, target: &HashSet<String>) -> Score;
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScorerKind {
    Jaccard,
    Hybrid,
}

impl Default for ScorerKind {
    fn default() -> Self {
        ScorerKind::Jaccard
    }
}

impl ScorerKind {
    pub fn instantiate(&self) -> Box<dyn Scorer> {
        match self {
            ScorerKind::Jaccard => Box::new(JaccardScorer),
            ScorerKind::Hybrid => {
                // TODO: implement a hybrid scorer that combines embeddings + rules
                Box::new(JaccardScorer)
            }
        }
    }
}

struct JaccardScorer;

impl Scorer for JaccardScorer {
    fn score(&self, query: &HashSet<String>, target: &HashSet<String>) -> Score {
        let result = jaccard_score(query, target);
        Score {
            value: result.score,
            matched_tokens: result.matched_tokens,
        }
    }
}
