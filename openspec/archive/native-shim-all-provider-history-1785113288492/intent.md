# Intent: native-shim-all-provider-history

## Raw Request

"no anh huong toan bo provider trong shim, khong chi mimo. Em sua luon va commit"

## Problem

The native shim's Responses-to-Anthropic adapter still accepts only string
input, while the Chat Completions adapter now supports structured turn history.
This creates provider-dependent failures after Codex tool execution.

## Desired Outcome

Every native shim route accepts the shared text and tool-result history shape:
OpenAI Responses routes retain it natively, while Chat Completions and
Anthropic routes translate it into their textual message forms.

## Users / Actors

- Thoor selecting any configured native-shim custom model
- Chat Completions, Anthropic Messages, and OpenAI Responses providers

## Current Context

`responses_to_chat_completion` translates structured history. OpenAI Responses
routes forward the original Responses body. `responses_to_anthropic_message`
still calls `Value::as_str` and rejects arrays.

## Proposed Direction

Reuse the normalized Chat message history for Anthropic conversion: move system
messages into `system`, retain the ordered non-system history as messages, and
keep the same explicit rejection for unrepresentable item types.

## Scope

- Structured Responses input for Anthropic native-shim routes.
- Cross-provider translator regression coverage.

## Non-Goals

- Provider-native tool execution and tool-call response generation.
- Python `codex-shim` changes, credentials, model catalog, and bridge work.

## Constraints

- Preserve existing string-input output exactly.
- Preserve provider credentials outside all artifacts and tests.
- Do not silently drop unknown structured items.

## Success Criteria

- Focused native-shim tests cover structured tool history for Chat Completions
  and Anthropic routes, and pass.
- The archive verifies complete traceability.

## Risks

- Flattened text is less expressive than native tool blocks.
  - Mitigation: retain call ids, names, arguments, and outputs in sequence.

## Ambiguities

### Blocking

- None.

### Non-Blocking

- Anthropic accepts a sequence of textual user and assistant messages after
  system text has been removed.

## Assumptions

- The shared normalized history is the canonical adapter boundary because the
  OpenAI Chat path already uses it successfully.

## Spec Seeds

These are candidate requirements for the spec phase. Not binding. Derived from Desired Outcome, Constraints, Risks, and Success Criteria.

- Anthropic routes SHALL accept structured Responses input without the
  string-only adapter error.

## Intent Approval

Status: APPROVED FOR IMPLEMENTATION

Approved by:
Date:
