# Intent: native-shim-tool-call-execution

## Raw Request

Fix tool calls for custom models. Model switching through 9router works, but models emit XML-looking tool calls instead of causing Codex to execute the tool.

## Problem

The Chat Completions adapter drops Responses `tools` when forwarding a Codex turn. It also accepts only text assistant messages on the return path. Upstream models therefore lack the tool schema or their native tool calls are rejected rather than returned to Codex.

## Desired Outcome

A 9router Chat Completions model receives the requested function and custom tool declarations and a native function/custom tool call is returned as a Responses output item that Codex can execute.

## Users / Actors

- Codex CLI user selecting a native-shim model
- 9router Chat Completions upstream

## Current Context

`codex-rs/native-shim/src/translation.rs` converts Responses requests and Chat Completions responses. Current conversion preserves messages but omits tools, and only supports textual completions. The CLI uses streamed Responses turns in normal operation.

## Proposed Direction

Translate Responses function-tool declarations to Chat Completions declarations, preserve custom declarations, and normalize Chat function/custom tool calls into Responses output items for both non-streaming and SSE responses.

## Scope

- OpenAI Chat Completions ingress/egress translation.
- Function and custom tool-call result normalization.
- Streaming and non-streaming regression coverage.

## Non-Goals

- 9router configuration and provider credentials.
- Anthropic and OpenAI Responses adapters.
- Prompt-level XML parsing fallback.

## Constraints

- Preserve ordinary text responses and existing tool-history normalization.
- Do not expose or serialize credentials in tests or artifacts.
- Use the existing native-shim translation module and TDD workflow.

## Success Criteria

- Unit tests show the Chat request retains translated tool declarations.
- Unit tests show non-streaming and streamed upstream tool calls become executable Responses items.
- `just test -p codex-native-shim` and LoomKit verification pass.

## Risks

- Chat providers vary in their custom-tool wire shape.
  - Mitigation: preserve custom declarations and recognize the documented `custom` payload while retaining function-call handling.

## Ambiguities

### Blocking

- None.

### Non-Blocking

- 9router forwards OpenAI Chat Completions tool fields without rewriting them.

## Assumptions

- Codex receives `response.output_item.done` function/custom-tool items from streamed responses; this is already the canonical event boundary used by its response parser.

## Spec Seeds

These are candidate requirements for the spec phase. Not binding. Derived from Desired Outcome, Constraints, Risks, and Success Criteria.

- Chat tool declarations and calls round-trip through the native shim.

## Intent Approval

Status: APPROVED

Approved by:
Date:
