## Why

Composed model catalog merging currently aborts the remote TUI when a model is
already present in the app-server catalog. A shared model should appear once,
not prevent the model picker from opening.

## What Changes

- Preserve the remote model entry and ignore a composed entry with the same slug.
- Replace the collision-error test with duplicate-deduplication coverage.

## Capabilities

### New Capabilities
<!-- Each becomes specs/<name>/spec.md. Use kebab-case. -->
- `composed-catalog-collision`: Deduplicate remote and composed model slugs.

### Modified Capabilities
<!-- Existing specs whose REQUIREMENTS change. Leave empty if none. -->
- `remote-composed-shim-catalog`: Duplicate slug handling becomes non-failing.

## Impact

- `codex-rs/tui/src/app_server_session.rs`
- No API or dependency changes
