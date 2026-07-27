## Architecture

The adapter remains deliberately text-oriented. It already maps normal
Responses messages to OpenAI Chat Completions messages. Add one explicit arm
for `function_call_output`; render `Tool output for <call_id>:\n<output>` as
a user message. This retains causal context without declaring that Xiaomi
supports OpenAI native tool roles.

## Components

- `responses_input_to_chat_messages`: recognizes and translates tool results.
- `translation_tests`: asserts the complete translated message list.

## Data Model

No schema or public API changes. The generated Chat Completions body gains a
normal `{role: "user", content: string}` entry for each tool result.

## Test Strategy

| Scenario ID | Test File | Type |
|-------------|-----------|------|
| NATIVE-TOOL-HISTORY-1 | `codex-rs/native-shim/src/translation_tests.rs` | unit |
| NATIVE-TOOL-HISTORY-2 | `codex-rs/native-shim/src/translation_tests.rs` | unit |

## Dependencies

None.

## Migration

None.
