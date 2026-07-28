## ADDED Requirements

### Requirement: Compose mode starts GPT turns on the native provider
The compose launcher SHALL override a stale global shim provider selection with the native OpenAI provider for models that are not custom shim routes.

#### Scenario: GPT selected with a stale global shim provider
- **WHEN** a composed child starts with `model_provider = "codex_shim"` in its config and selects a GPT model
- **THEN** its request reaches the configured native OpenAI endpoint and makes zero requests to the shim endpoint
