use std::collections::HashSet;

pub struct JaccardScore {
    pub score: f64,
    pub matched_tokens: Vec<String>,
}

pub fn jaccard_score(query: &HashSet<String>, target: &HashSet<String>) -> JaccardScore {
    let mut matched: Vec<String> = query
        .intersection(target)
        .map(|value| value.clone())
        .collect();
    matched.sort();
    let intersection_size = matched.len() as f64;
    let union_size = (query.len() + target.len()) as f64 - intersection_size;
    let score = if union_size <= 0.0 { 0.0 } else { intersection_size / union_size };
    JaccardScore {
        score,
        matched_tokens: matched,
    }
}
