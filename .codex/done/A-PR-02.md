# A-PR-02 — Adaptive Card scanner + metadata extraction (cardId/flow/actions) + flow grouping IR

Date: 2026-01-20

## Goal

Add a robust scanner that:
- finds `.json` Adaptive Card files under `--cards`,
- extracts `card_id`, `flow_name`, and route targets from actions,
- builds an intermediate representation (IR) grouped into flows,
- writes this IR into `.cards2pack/manifest.json` for debugging and later stages.

This PR does **not** emit real Greentic flows yet; it only provides **accurate, testable extraction** and grouping.

## Inputs and conventions supported

### Card identity resolution (card_id)

Resolution order:
1. Any `Action.*.data.cardId` found (must be consistent across actions in the same card)
2. Top-level `greentic.cardId` (optional custom metadata block)
3. Filename stem (e.g. `HR-CARD-00.json` -> `HR-CARD-00`)

If multiple different cardIds are detected in one file -> error in strict mode, warn otherwise (pick first).

### Flow name resolution (flow_name)

Resolution order:
1. Any `Action.*.data.flow` found (must be consistent within card)
2. Top-level `greentic.flow`
3. Folder grouping fallback:
   - if `--group-by folder`, use the immediate folder name under `--cards` (e.g. `cards/hrAssist/*.json` -> `hrAssist`)
4. `--default-flow` if still none

### Action route target extraction

For each action in `actions[]`:
- If `action.data.step` exists: route target kind = `step`, value = string
- Else if `action.data.cardId` exists: route target kind = `cardId`, value = string
- Else: no route target (terminal / stays on same step; later PR decides)

Capture:
- action `type` (Submit / Execute / other)
- `title` (for branching labels)
- route target (optional)
- raw `data` object (for later templating)

## IR model

Add `src/ir.rs`:

```rust
struct CardDoc {
  rel_path: String,
  abs_path: PathBuf,
  card_id: String,
  flow_name: String,
  actions: Vec<CardAction>,
}

struct CardAction {
  action_type: String,
  title: Option<String>,
  target: Option<RouteTarget>,
  data: serde_json::Value,
}

enum RouteTarget {
  Step(String),
  CardId(String),
}

struct FlowGroup {
  flow_name: String,
  cards: Vec<CardDoc>,
}
```

Write a top-level manifest:

```json
{
  "version": 1,
  "generated_at": "...",
  "input": {
    "cards_dir": "...",
    "group_by": "...",
    "default_flow": "..."
  },
  "flows": [
    { "flow_name": "hrAssist", "cards": [ ... ] }
  ],
  "warnings": [ ... ]
}
```

## Implementation steps

1. Add `scan.rs`:
   - Walk directory using `walkdir`
   - For each `.json`, parse with `serde_json`
   - Extract actions from `actions` array only (do not attempt to parse nested actions in body yet)
2. Implement extraction helpers:
   - `extract_action_data_fields(action_value) -> (flow?, cardId?, step?)`
3. Implement resolution logic (order above)
4. Group into flows
5. Update `generate` pipeline:
   - after copying cards, run scanner against **copied** cards in `assets/cards` so workspace is self-contained
   - write `.cards2pack/manifest.json` with IR

## Tests

Add fixtures in `tests/fixtures/cards/`:
- `simple_submit.json` with `data: { flow, cardId, step }`
- `folder_grouping/cards/hrAssist/HR-CARD-00.json` with no flow field
- `filename_fallback/IT-CARD-99.json` no cardId anywhere
- `inconsistent_cardid.json` with two actions disagreeing

Test cases:
- resolves cardId in correct order
- resolves flow in correct order
- extracts step vs cardId targets
- groups flows correctly

## Acceptance criteria

- `generate` writes `.cards2pack/manifest.json` including flows and actions.
- Scanner passes tests and is robust to non-card JSON files (ignore those with warning).
- No change required to greentic-pack or greentic-flow.
