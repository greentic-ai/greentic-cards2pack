# A-PR-03 — Flow graph builder + flow emitter (generated blocks) + idempotent regeneration

Date: 2026-01-20

## Goal

Turn the extracted IR (A-PR-02) into **actual Greentic flow files** while preserving developer edits.

This PR introduces:
- graph construction (nodes/edges) from card actions,
- a flow emitter that writes `flows/<flow>.flow.yaml`,
- generated-block markers so re-running `generate` updates only the generated section,
- optional stub-node creation for unresolved targets (warn, or error in `--strict`).

## Design: routing strategy

For each card node (named by `card_id`):
- The node renders the card asset (Adaptive Card component).
- On submit/execute, routing is decided by the action’s `data.step` (preferred) or `data.cardId`.
- The route key is the action identifier:
  - Prefer stable key: `data.step` if present, else `data.cardId`, else `title`, else action index.

We do **not** interpret UI inputs; we only route.

## Graph building rules

### Nodes
- One node per `card_id` within a flow group
- Node name = `card_id` (safe and explicit)

### Edges
For each action with a target:
- if target is `Step(name)`:
  - if a node exists with name == `name`: edge to it
  - else:
    - if `--strict`: error “missing step target”
    - else: create stub node named `name` with placeholder “TODO” component
- if target is `CardId(id)`:
  - if node exists with card_id == id: edge to it
  - else strict/warn similarly

### Terminal cards
- If a card has no actions with targets, treat as EndFlow.
- Emit explicit `EndFlow` route if your flow format supports it; otherwise omit outgoing edges.

## Flow emitter

### Generated-block markers

Each emitted flow file contains:

```yaml
# BEGIN GENERATED (cards2pack)
... generated content ...
# END GENERATED (cards2pack)

# Developer space below (preserved on regen)
```

On regeneration:
- If markers exist, replace content between them.
- If file absent, create it with markers.

### Emitted content (example structure)

Because exact Greentic flow schema may vary, this PR should:
- Use the existing greentic-flow YAML schema currently used in your repos.
- Keep nodes simple: render card by referencing asset path.

Illustrative pseudostructure:

```yaml
flow: hrAssist
nodes:
  HR-CARD-00:
    type: component
    component: oci://.../component_adaptive_card
    input:
      card_path: assets/cards/hrAssist/HR-CARD-00.json
    routes:
      submitIdentity: HR-CARD-01
      cancel: EndFlow
```

If Greentic requires explicit `steps:` etc, adapt accordingly.

## Idempotency beyond flows

- `pack.yaml`: only add/ensure references to `flows/*.flow.yaml` (if required); do not overwrite other pack fields.
- `assets/cards`: copy cards only if source changed (optional optimization); ok to always overwrite assets.

Optionally keep `.cards2pack/manifest.json` so regeneration knows what it produced last time.

## Implementation steps

1. Add `graph.rs`:
   - Convert IR to `FlowGraph` with `nodes` + `edges` + `warnings`
2. Add `emit_flow.rs`:
   - `emit_flow(flow_graph, workspace_root) -> PathBuf`
   - marker replace logic
3. Update `workspace.rs`:
   - For each flow group, write `flows/<flow>.flow.yaml`
   - Update README with list of flows and entry nodes
4. Update `generate` pipeline:
   - scan -> graph -> emit flow(s) -> build pack

## Tests

Fixtures:
- branching card with two actions
- unresolved step target

Golden tests:
- compare emitted flow YAML between markers to expected
- ensure developer content outside markers preserved

## Acceptance criteria

- Running `generate` twice does not erase manual edits outside generated blocks.
- Flow YAML is produced for each inferred flow.
- Unresolved targets:
  - warn + stub in non-strict mode
  - error in strict mode
- `dist/<name>.gtpack` still built successfully.
