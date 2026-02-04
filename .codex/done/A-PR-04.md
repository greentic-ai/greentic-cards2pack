# A-PR-04 — Integrated generate pipeline polish: strict mode, diagnostics summary, and gtpack naming

Date: 2026-01-20

## Goal

Polish the developer experience so `generate` feels “one-shot reliable”:

- `--strict` becomes meaningful and enforced.
- Clear diagnostics printed at end:
  - flows detected
  - cards processed
  - warnings (inconsistent cardId/flow, ignored files, missing targets)
  - generated paths
- More robust `.gtpack` artifact selection and naming:
  - if greentic-pack outputs a different filename, select newest `.gtpack` in dist and copy/rename to `dist/<name>.gtpack`
- Add `--verbose` for printing the exact greentic-pack command and stdout/stderr.

## Strict mode behavior

In strict mode, fail on:
- No cards found
- Duplicate card_id within same flow
- Card missing card_id after resolution attempts
- Card missing flow_name after all fallbacks
- Inconsistent action `data.flow` values within a card
- Inconsistent action `data.cardId` values within a card
- Any action target referencing missing node (no stub allowed)
- Any JSON parse error for a file that ends with `.json` (unless clearly not an adaptive card; see below)

Non-strict mode:
- Ignore JSON files that aren’t adaptive cards (missing `type: AdaptiveCard`) with warning
- Allow missing targets by creating stubs
- Allow missing flow by using default-flow or folder fallback; if still missing, put into `misc` flow with warning

## Diagnostics format

At end of `generate`, print:

- Workspace root
- Dist artifact path
- Flows list with card counts
- Warnings count + top warnings

Also write diagnostics into `.cards2pack/manifest.json` so it’s reproducible.

## Implementation steps

1. Add `diagnostics.rs`:
   - warning type enum + display
   - summary printer
2. Update scanner and graph builder to return structured warnings/errors
3. Add `--verbose` flag
4. Improve gtpack selection:
   - if expected `dist/<name>.gtpack` exists: ok
   - else:
     - list dist dir for `*.gtpack`, pick most recently modified
     - copy to expected name
     - warn that filename was normalized

## Tests

- strict mode fails on missing target
- non-strict creates stub and succeeds
- gtpack naming normalization works:
  - mock greentic-pack produces `something_else.gtpack`, ensure it becomes `<name>.gtpack`

## Acceptance criteria

- `generate --strict` provides actionable error messages
- `generate` always ends with `dist/<name>.gtpack` on success
- diagnostics are helpful and stored in manifest
