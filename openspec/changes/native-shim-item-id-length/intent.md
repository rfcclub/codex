# Intent: native-shim-item-id-length

## Raw Request

"lai loi telegram va shim ... switch qua cheapinfe/deepseek-v4-flash xong sau do switch lai terra hay luna thi co loi ... Invalid 'input[15].id': string too long. Expected a string with maximum length 64, but got a string with length 69 instead."

## Problem

Response history preserves a prefixed item ID longer than the Responses API accepts. A later turn sent to a native Responses provider therefore fails before inference.

## Desired Outcome

Switching from a composed Chat Completions shim model back to Terra or Luna does not submit item IDs longer than 64 characters.

## Users / Actors

- Thoor, switching models in a visible Codex CLI
- Native Responses providers receiving resumed history

## Current Context

`ResponseItemId::is_prefixed` accepts any non-empty `prefix_suffix` value. `Client::prepare_response_items_for_request` retains those IDs even when they came from a shim and exceed the upstream maximum.

## Proposed Direction

Omit item IDs longer than the Responses API maximum while retaining valid, short prefixed IDs for cache and resume behavior.

## Scope

- Request-boundary normalization in `codex-rs/core/src/client.rs`
- Regression coverage against the serialized Responses request

## Non-Goals

- Telegram bridge routing or restart
- Changes to the external shim service or provider configuration

## Constraints

- Preserve the in-memory prompt and all short prefixed IDs.
- No provider-specific ID parsing or migration.

## Success Criteria

- A 65-character prefixed item ID is omitted from the outbound request while a short `msg_` ID is preserved.

## Risks

- Losing a useful ID could reduce cache reuse.
  - Mitigation: only omit IDs that upstream will reject.

## Ambiguities

### Blocking

- None.

### Non-Blocking

- The 64-character limit applies to every outbound Responses request using this shared request path.

## Assumptions

- The provider error is authoritative for the maximum; omitting an item ID is already supported by the Responses API.

## Spec Seeds

These are candidate requirements for the spec phase. Not binding. Derived from Desired Outcome, Constraints, Risks, and Success Criteria.

- Long prefixed IDs are omitted without changing the original prompt.

## Intent Approval

Status: IMPLEMENTING

Approved by:
Date:
