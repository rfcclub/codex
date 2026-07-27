## Architecture

Use `responses_input_to_chat_messages` as the single structured Responses
normalization boundary. For Anthropic routes, pull normalized system entries
into `system` and pass remaining textual messages in order. OpenAI Responses
needs no conversion because it already speaks the source protocol.

## Components

- `responses_to_anthropic_message`: normalize and split system text.
- `translation_tests`: cover Anthropic structured tool history.

## Data Model

No external schema changes. The Anthropic payload's `messages` field accepts
the normalized text messages already used for Chat Completions.

## Test Strategy

| Scenario ID | Test File | Type |
|-------------|-----------|------|
| NATIVE-ALL-PROVIDERS-1 | `codex-rs/native-shim/src/translation_tests.rs` | unit |
| NATIVE-ALL-PROVIDERS-2 | `codex-rs/native-shim/src/relay_tests.rs` | integration |

## Dependencies

None.

## Migration

None.
