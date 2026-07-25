## Why

`codexia` currently vendors a Python `codex-shim`, but the patched Rust binary
does not contain or activate a shim runtime. Consequently a model catalog and
provider routes cannot be selected from Codex without a separately installed
Python sidecar.

## What Changes

- Add a native Rust shim crate to the Codex workspace.
- Read an explicit, user-owned JSON settings file and derive a Codex model
  catalog without mutating `~/.codex/config.toml`.
- Start a loopback-only Responses-compatible proxy for configured
  OpenAI-compatible API-key routes.
- Add an explicit `codex shim` launcher surface which starts the native proxy,
  selects its generated catalog, and launches the same `codexia` executable.
- Enforce configured per-provider concurrency limits, including foreground
  requests, with cancellation-safe lease release.

Implementation is staged by protocol: OpenAI Responses first, then OpenAI Chat
Completions, then Anthropic Messages. The completed change supports all three
for API-key routes. Legacy subscription passthrough remains out of scope.

## Capabilities

### New Capabilities
- `native-shim-runtime`: Native loopback proxy, catalog generation, launcher,
  and provider admission control for `codexia`.

### Modified Capabilities
None.

## Impact

- New `codex-rs/native-shim` workspace crate and focused CLI wiring.
- No automatic edits to existing Codex or legacy shim configuration.
- `~/.codex-shim/models.json` remains the default settings path for
  compatibility, but only its documented API-key subset is accepted.
