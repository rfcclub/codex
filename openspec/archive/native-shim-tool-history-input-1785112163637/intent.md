# Intent: native-shim-tool-history-input

## Raw Request

"Loi khi dung shim voi xiaomi/mimo-v2.5 em oi {\"error\":{\"message\":\"native shim only supports string Responses input for this provider adapter\"}}"

## Problem

The native API-key shim rejects a Responses request when its input contains a
tool-result history item without message content. Codex sends this shape after
tools execute, so Xiaomi Mimo cannot continue a normal agent turn.

## Desired Outcome

Mimo receives a valid Chat Completions history for supported text, message,
and function-call-output Responses items; unsupported item types remain a
clear client error.

## Users / Actors

- Thoor selecting `xiaomi/mimo-v2.5-pro` in patched Codex
- `codex-native-shim` translating Requests to the Xiaomi Chat Completions API

## Current Context

`responses_to_chat_completion` already accepts string and message-array
inputs. Its array converter errors when it encounters a valid
`function_call_output` item because that item uses `output`, not `content`.

## Proposed Direction

Translate a function-call output into a textual user message that preserves
the call identifier and output, allowing a text-only Chat Completions provider
to continue the turn without pretending it supports native tool roles.

## Scope

- Responses-to-Chat conversion for `function_call_output` entries.
- Regression coverage of mixed assistant tool call, tool output, and user text.

## Non-Goals

- Native tool-call execution or response tool-call conversion for Xiaomi.
- Changes to Python `codex-shim`, provider credentials, or picker catalog.

## Constraints

- Preserve existing string and structured-message behavior.
- Never put provider credentials in tests, logs, or OpenSpec artifacts.
- Keep all unrecognized Responses item types rejected rather than silently
  deleting context.

## Success Criteria

- Focused native-shim tests pass, including a mixed tool-history request.
- A live `codex-patched shim --compose` Mimo request no longer returns the
  string-only input error.

## Risks

- Flattened tool output loses native structured semantics.
  - Mitigation: retain the output verbatim with its call id and limit this
    adapter to text continuation rather than tool execution.

## Ambiguities

### Blocking

- None.

### Non-Blocking

- Xiaomi accepts standard user text messages, as its adapter is configured as
  `openai-chat-completions`.

## Assumptions

- A function-call-output can be represented as user-visible tool result text;
  this mirrors the current adapter's text-only contract and fixes the crash.

## Spec Seeds

These are candidate requirements for the spec phase. Not binding. Derived from Desired Outcome, Constraints, Risks, and Success Criteria.

- A `function_call_output` entry SHALL be translated without rejecting the
  enclosing Responses input array.

## Intent Approval

Status: APPROVED FOR IMPLEMENTATION

Approved by:
Date:
