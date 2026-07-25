# Self-Check

## Claims
- Native loopback shim validates explicit API-key settings, creates a private
  catalog, launches the current executable, and removes its runtime state when
  the child exits.
- The Responses, Chat Completions, and Messages ingress endpoints relay all
  supported OpenAI and Anthropic provider protocols, including SSE adapters.
- `codex shim doctor` validates settings and reports schema migration status
  without starting a listener or exposing credentials.
- Migration and rollback: no automatic data/config migration occurs; rollback
  is a code revert and direct Codex remains unchanged.

## Evidence
```json
[
  {"type":"command","command":"just test -p codex-native-shim","exit_code":0,"output":"46 tests passed"},
  {"type":"command","command":"cargo test -p codex-cli --test shim","exit_code":0,"output":"2 tests passed"},
  {"type":"command","command":"cargo build -p codex-cli","exit_code":0,"output":"development binary built"}
]
```

<!-- Evidence format:
[
  { "type": "command", "command": "bun test", "exit_code": 0, "output": "12 pass 0 fail" },
  { "type": "file", "file": "src/foo.ts", "content": "relevant snippet" }
]
-->

## Known Limitations
- Bazel lockfile update could not be run in this environment because the
  `bazel` executable is not installed.
- The repository-wide `just fmt` gate remains unavailable because `dotslash`
  and its buildifier wrapper are not installed; Rust formatting was run with
  `cargo fmt --all`.

## Unresolved Assumptions
- The compiled Cargo development binary is representative of the release
  Bazel artifact; release/Bazel packaging still requires a machine with Bazel.

## Rollback Plan

- This change performs no automatic data or configuration migration. Direct
  Codex continues to use its existing configuration unchanged.
- Rollback is a code revert of the native-shim crate and `codex shim` command;
  any user-created `~/.codex-shim/models.json` is inert after that revert and
  can be retained or removed by the user without affecting direct Codex.
