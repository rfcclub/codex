## Architecture

`responses_input_to_chat_messages` is the single loss-boundary for converting Responses input to chat-completions messages. `reasoning` is opaque model-internal history and has no chat-completions representation, so the iterator will skip it before supported-item conversion.

## Components

- `codex-rs/native-shim/src/translation.rs`: omit `reasoning` input items.
- `codex-rs/native-shim/src/translation_tests.rs`: regression coverage with live-log-shaped input.

## Data Model

No public schema changes.

## Test Strategy

| Scenario ID | Test File | Type |
|-------------|-----------|------|
| NSRH-1 | `codex-rs/native-shim/src/translation_tests.rs` | unit |
| NSRH-2 | `codex-rs/native-shim/src/translation_tests.rs` | unit |

## Dependencies

No new dependencies.

## Migration

Existing clients require only a rebuilt patched binary.
