## Why

Native shim models can switch successfully through 9router but cannot execute tools because the Chat Completions translation silently removes the tool contract. This makes models emit text/XML approximations instead of native tool calls.

## What Changes

- Translate Responses function tools into Chat Completions function tools and preserve custom tools.
- Preserve `tool_choice` and `parallel_tool_calls` request controls.
- Translate Chat function and custom tool calls into Responses output items for regular and SSE responses.

## Capabilities

### New Capabilities
- `native-shim-chat-tool-calls`: preserve and normalize tool calls for Chat Completions routes.

### Modified Capabilities
- None.

## Impact

Changes `codex-rs/native-shim/src/translation.rs`, its unit tests, and relay tests. No dependency, model-catalog, or provider-config changes.
