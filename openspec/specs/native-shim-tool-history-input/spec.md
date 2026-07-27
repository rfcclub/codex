# native-shim-tool-history-input Specification

## Requirements

### Requirement: Preserve function-call result history for chat providers

When a native-shim route uses the Chat Completions adapter, the translator
SHALL accept a Responses input array containing `function_call_output` items
and preserve each result as text in the translated message history.

#### Scenario: Tool result follows a function call

- **WHEN** a Responses input array contains an assistant `function_call`, its
- **THEN** the translated Chat Completions `messages` contains the assistant

### Requirement: Reject unsupported structured items explicitly

The translator SHALL continue to reject an input item whose type cannot be
represented as text by the Chat Completions adapter.

#### Scenario: Unknown item type

- **WHEN** a Responses input array contains an item with no supported text or
- **THEN** translation returns `UnsupportedResponsesInput`
