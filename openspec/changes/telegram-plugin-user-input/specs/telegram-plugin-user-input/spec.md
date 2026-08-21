## ADDED Requirements

### Requirement: Standalone Telegram user-input delivery
Codex SHALL attach its realtime user-input handler to the MCP server named
`telegram` so the installed standalone Telegram plugin can deliver an
allowlisted incoming message to the CLI session that hosts that MCP server.

#### Scenario: Telegram plugin notification
- **WHEN** the MCP server name is `telegram`
- **THEN** the server is permitted to use the realtime user-input handler

#### Scenario: Other MCP notification
- **WHEN** an unrelated MCP server emits `codex/user-input`
- **THEN** it is not permitted to use the realtime user-input handler

#### Scenario: Legacy compatibility
- **WHEN** the MCP server name is `colony-mesh`
- **THEN** it remains permitted to use the realtime user-input handler
