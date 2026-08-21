# Intent: telegram-plugin-user-input

## Raw Request

fix MCP telegram cho xong

## Problem

The active standalone Telegram MCP plugin polls the configured bot and receives
allowlisted messages, but Codex drops its `codex/user-input` notifications.

## Desired Outcome

An allowlisted Telegram message becomes a visible user turn in the active
Codex CLI that hosts the Telegram MCP, and that same CLI can reply through the
plugin's `reply` tool.

## Users / Actors

- Thoor
- The standalone `codex-telegram-plugin` MCP server

## Current Context

`codex-mcp` only attaches the user-input handler to `colony-mesh`. The new
plugin starts as MCP server `telegram`, so its notification is ignored.

## Proposed Direction

Explicitly authorize the `telegram` MCP server alongside the legacy
`colony-mesh` bridge at the existing handler boundary.

## Scope

- Trust `telegram` for `codex/user-input` notifications.
- Preserve the current `colony-mesh` behavior.
- Add focused regression coverage and perform a real bot-to-visible-CLI check.

## Non-Goals

- Reintroducing the colony-mesh Telegram bridge or webhook path.

## Constraints

- Do not give arbitrary MCP servers the ability to submit user turns.

## Success Criteria

- `telegram` receives the handler; unrelated server names do not.
- A fresh real Telegram message appears in this active CLI and a reply is delivered.

## Risks

- Granting user-input authority too broadly.
  - Mitigation: exact server-name allowlist at the sole handler boundary.

## Ambiguities

### Blocking

- None.

### Non-Blocking

- The installed standalone plugin remains registered as MCP server `telegram`.

## Assumptions

- The plugin's local installation and explicit name are user-controlled and
  verified in the active CLI process.

## Spec Seeds

These are candidate requirements for the spec phase. Not binding. Derived from Desired Outcome, Constraints, Risks, and Success Criteria.

- A server named `telegram` SHALL be able to deliver non-empty
  `codex/user-input` notifications to its hosting Codex session.

## Intent Approval

Status: IMPLEMENTING

Approved by:
Date:
