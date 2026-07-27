# Intent: native-shim-custom-tool-history

## Raw Request

Fix `native shim only supports string Responses input for this provider adapter` so custom models can continue a turn after Codex custom tool calls.

## Problem

The native shim rejects Codex `custom_tool_call` and `custom_tool_call_output` history while translating a Responses request for Chat Completions or Anthropic providers.

## Desired Outcome

A custom-provider Responses turn with prior custom tool calls is translated and forwarded instead of returning HTTP 400.

## Scope

- Shared translation for Chat Completions and Anthropic Messages routes.
- Regression tests for custom tool input and content-item output.

## Non-Goals

- Provider API, models.json, and tool-execution changes.

## Constraints

- Preserve function-call translation and OpenAI Responses pass-through.
- Do not expose provider credentials.

## Success Criteria

- Native shim tests prove custom tool history translates for both adapter families.

## Risks

- Content items without text cannot be faithfully represented for text-only providers.
  - Mitigation: preserve text items and reject an output with no usable text.

## Assumptions

- Freeform tool input is textual because the protocol type requires a string.

## Intent Approval

Status: APPROVED
