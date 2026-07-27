## Architecture

Extend `responses_input_to_chat_messages`, the existing shared input normalizer. A helper extracts usable text from a string or supported content-item array; both Chat Completions and Anthropic adapters already consume this normalized output.

## Components

- `responses_input_to_chat_messages`: recognize `custom_tool_call` and `custom_tool_call_output`.
- `response_content_to_text`: read `input_text`, `output_text`, or `text` content items.

## Data Model

No public schema change. The shim accepts an existing Codex Responses wire variant.

## Test Strategy

| Scenario ID | Test File | Type |
|-------------|-----------|------|
| CTH-1 | `codex-rs/native-shim/src/translation_tests.rs` | unit |
| CTH-2 | `codex-rs/native-shim/src/translation_tests.rs` | unit |

## Dependencies

None.

## Migration

None.
