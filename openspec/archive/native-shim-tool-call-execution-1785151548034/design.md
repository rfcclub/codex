## Architecture

Keep the conversion in `translation.rs`, where the existing Responses-to-Chat and Chat-to-Responses adapters live. Use the upstream Chat Completions standard wrapper for Responses function tools, while forwarding custom tools unchanged. Return native tool calls as Responses output items rather than trying to parse model text/XML.

## Components

- `responses_to_chat_completion`: translate tool declarations and controls.
- `chat_completion_to_response`: normalize a non-streaming assistant message or its tool calls.
- `ChatSseToResponses`: accumulate streamed Chat tool-call deltas and emit completed Responses tool-call items.

## Data Model

No public configuration change. Internal translation accepts Chat `tool_calls` with either `function` (`arguments`) or `custom` (`input`) payloads.

## Test Strategy

| Scenario ID | Test File | Type |
|-------------|-----------|------|
| CTC-1 | `codex-rs/native-shim/src/translation_tests.rs` | unit |
| CTC-2 | `codex-rs/native-shim/src/translation_tests.rs` | unit |
| CTC-3 | `codex-rs/native-shim/src/relay_tests.rs` | integration |

## Dependencies

None.

## Migration

None.
