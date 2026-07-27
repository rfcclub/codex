# native-shim-reasoning-history Specification

## Requirements

### Requirement: Preserve actionable history around opaque reasoning

When a native-shim chat-completions route receives a structured Responses `input` array containing `reasoning` items interleaved with supported message, function-call, or tool-output items, the shim SHALL omit the reasoning items and translate the supported items in their original order.

#### Scenario: Interleaved reasoning does not reject a follow-up turn

- **WHEN** a request includes a reasoning item before a tool call, a tool output, and a user message
- **THEN** the request translates successfully

### Requirement: Unsupported item types remain fail-closed

When the input array contains an unsupported item type other than `reasoning`, the shim SHALL return `UnsupportedResponsesInput`.
