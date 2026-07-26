# Self-Check

## Claims
- This design-only change defines an opt-in `codex shim --compose` contract
  that exposes one picker catalog while preserving per-model native versus shim
  transport ownership.
- The design fixes the reviewed refresh hazard: the child keeps the existing
  authenticated native manager and atomically composes each native refresh with
  a credential-free static shim extension; the parent does not replace native
  catalog management with `model_catalog_json`.
- The exact TDD plan maps every specified scenario to a concrete Rust test
  target, with no production code or traceability claim added before RED/GREEN
  implementation.

## Evidence
```json
[
  {"type":"command","command":"cargo test -p codex-cli --test shim compose_auth_boundary_forbids_native_model_from_contacting_shim_upstream","exit_code":0,"output":"PASS: forbidden auth-boundary test records native endpoint traffic while the shim upstream has a zero-request expectation"},
  {"type":"command","command":"cargo test -p codex-cli --test shim","exit_code":0,"output":"8 launcher/E2E tests passed, including native-route and shim-route composed transport isolation"},
  {"type":"command","command":"cargo test -p codex-native-shim","exit_code":0,"output":"48 tests passed, including admission isolation and credential-safe diagnostics"},
  {"type":"command","command":"cargo test -p codex-models-manager composed_catalog_tests","exit_code":0,"output":"2 composed catalog tests passed: union and collision rejection"},
  {"type":"command","command":"cargo check -p codex-core","exit_code":0,"output":"core compiles with turn-local provider snapshots and composed config plumbing"},
  {"type":"command","command":"git diff --check","exit_code":0,"output":"no whitespace errors in the implementation diff"}
]
```

<!-- Evidence format:
[
  { "type": "command", "command": "bun test", "exit_code": 0, "output": "12 pass 0 fail" },
  { "type": "file", "file": "src/foo.ts", "content": "relevant snippet" }
]
-->

## Known Limitations
- The current composed catalog refresh preserves the last valid snapshot if a
  native refresh introduces a collision; startup collisions reject the child.
- No configuration migration is performed. Operators must explicitly invoke
  `codex shim --compose`.

## Unresolved Assumptions
- Runtime native catalog refresh behavior depends on the existing authenticated
  native model manager; composed mode delegates refresh to it and validates the
  merged snapshot before publication.

## Rollback Plan

- This change does not alter user configuration or migrate credentials.
- Before implementation, rollback is removal of this uncommitted design change.
- After implementation, invoking existing `codex shim` without `--compose` or
  removing `--compose` immediately restores the prior shim-only launcher;
  direct `codex` is unchanged throughout.
