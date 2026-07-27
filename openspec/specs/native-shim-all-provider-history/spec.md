# native-shim-all-provider-history Specification

## Requirements

### Requirement: Structured Responses history works across native routes

The native shim SHALL accept the same supported structured Responses input for
Chat Completions and Anthropic Messages routes. OpenAI Responses routes SHALL
continue to receive the original request unchanged.

#### Scenario: Structured tool history targets an Anthropic provider

- **WHEN** a Responses request contains tool call, tool output, and user text
- **THEN** the Anthropic request has `system =` the developer instruction and

#### Scenario: Structured tool history targets an OpenAI Responses provider

- **WHEN** a native route uses the OpenAI Responses protocol
- **THEN** the relay forwards the original structured `input` array unchanged
