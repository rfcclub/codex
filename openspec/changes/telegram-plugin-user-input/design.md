## Architecture

Keep the existing handler assignment in `McpConnectionSet::new`. Replace the
single hard-coded bridge name with a small explicit authorization predicate for
the two trusted local channels. This retains the existing rmcp notification
parser and session-local input path.

## Components

- `McpConnectionSet`: assigns `SendUserInput` only to trusted server names.

## Data Model

No wire or configuration schema changes.

## Test Strategy

| Scenario ID | Test File | Type |
|-------------|-----------|------|
| T1 | `codex-rs/codex-mcp/src/connection_manager_tests.rs` | unit |
| T2 | active `codex-coda` CLI and `@gbb_ccodae_bot` | E2E |

## Dependencies

None.

## Migration

No migration. Existing `colony-mesh` behavior remains available.
