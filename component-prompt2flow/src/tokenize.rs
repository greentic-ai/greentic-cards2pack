use std::collections::HashSet;

pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter_map(|segment| {
            let trimmed = segment.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

pub fn unique_tokens<I>(values: I) -> HashSet<String>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut tokens = HashSet::new();
    for value in values {
        for token in tokenize(value.as_ref()) {
            tokens.insert(token);
        }
    }
    tokens
}
