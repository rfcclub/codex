## Why

The standalone Telegram plugin is already polling `@gbb_ccodae_bot` and
persisting inbound messages, but Codex only trusts the retired bridge name for
realtime user-input notifications.

## What Changes

- Permit the `telegram` MCP server to receive the existing user-input handler.
- Keep `colony-mesh` support unchanged for compatibility.
- Add regression coverage for the explicit trust boundary.

## Capabilities

### New Capabilities
- `telegram-plugin-user-input`: accept standalone Telegram MCP notifications.

### Modified Capabilities
- None.

## Impact

- `codex-rs/codex-mcp/src/connection_manager.rs`
- Active `codex-telegram-plugin` MCP sessions
