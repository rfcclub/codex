## ADDED Requirements

### Requirement: Custom tool history translates for text-only provider adapters

The native shim SHALL translate `custom_tool_call` and `custom_tool_call_output` Responses input items for Chat Completions and Anthropic Messages provider routes. It SHALL extract text from a string output or a content-item output array. OpenAI Responses routes SHALL remain pass-through.

#### Scenario: Custom tool call with content-item output targets Chat Completions

- **WHEN** a Responses request contains a `custom_tool_call` followed by a `custom_tool_call_output` with an `input_text` content item
- **THEN** the translated messages contain an assistant tool-call transcript and a user tool-output transcript with the item text

#### Scenario: Custom tool history targets Anthropic Messages

- **WHEN** a Responses request contains custom tool history and targets an Anthropic Messages provider
- **THEN** the translated Anthropic `messages` array contains the equivalent assistant and user transcripts
