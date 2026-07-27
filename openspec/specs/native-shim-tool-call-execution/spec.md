# native-shim-tool-call-execution Specification

## Requirements

### Requirement: Chat adapter preserves the Codex tool contract

The native shim SHALL translate Responses function-tool declarations to the OpenAI Chat Completions function-tool shape, SHALL preserve Responses custom-tool declarations, and SHALL forward `tool_choice` and `parallel_tool_calls` when present.

#### Scenario: Function and custom tools target a Chat Completions route

- **WHEN** a Responses request contains function and custom tool declarations
- **THEN** the translated Chat request contains both declarations, with the function wrapped in its `function` object

### Requirement: Chat tool calls become Codex Responses output items

The native shim SHALL emit a `function_call` or `custom_tool_call` Responses output item for a corresponding upstream Chat Completions tool call. It SHALL apply the same conversion after accumulating streamed tool-call deltas.

#### Scenario: Non-streaming function call

- **WHEN** an upstream Chat completion contains a function `tool_call`
- **THEN** the shim response contains a `function_call` output item with the same `call_id`, name, and arguments

#### Scenario: Streamed custom tool call

- **WHEN** an upstream Chat stream contains custom tool-call deltas followed by `[DONE]`
- **THEN** the shim emits a completed `custom_tool_call` output item whose input is the concatenated deltas
