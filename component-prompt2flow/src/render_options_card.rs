use serde_json::{json, Value};
use crate::IntentMatch;

pub fn build_options_card(query: &str, matches: &[IntentMatch]) -> Value {
    let actions: Vec<Value> = matches
        .iter()
        .map(|intent| {
            json!({
                "type": "Action.Submit",
                "title": intent.title,
                "data": {
                    "intent_id": intent.intent_id,
                    "route": intent.route_payload(),
                    "score": intent.score,
                    "matched_tokens": intent.matched_tokens,
                    "examples": intent.examples.clone(),
                }
            })
        })
        .collect();

    let metadata_matches: Vec<Value> = matches
        .iter()
        .map(|intent| {
            json!({
                "intent_id": intent.intent_id,
                "score": intent.score,
                "matched_tokens": intent.matched_tokens.clone(),
                "route": intent.route_payload(),
            })
        })
        .collect();

    json!({
        "type": "MessageCard",
        "tier": "advanced",
        "payload": {
            "adaptive_card": {
                "type": "AdaptiveCard",
                "version": "1.4",
                "body": [
                    {"type": "TextBlock", "text": "I found multiple possibilities.", "wrap": true, "weight": "bolder"},
                    {"type": "TextBlock", "text": format!("Query: {query}"), "wrap": true}
                ],
                "metadata": {
                    "query": query,
                    "matches": metadata_matches
                },
                "actions": actions
            }
        }
    })
}
