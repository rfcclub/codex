## Why

Codex emits `custom_tool_call` history for freeform tools, but the native shim only recognizes function-call history. This makes every non-Responses custom provider fail after a tool call.

## What Changes

- Normalize custom tool calls and outputs alongside function tool history.
- Convert text-bearing content-item outputs to the text form required by Chat Completions and Anthropic adapters.

## Capabilities

### New Capabilities

- `native-shim-custom-tool-history`: translate Codex custom tool history for text-only upstream adapters.

### Modified Capabilities

- `native-shim-all-provider-history`: extend structured-history support to Codex custom tools.

## Impact

Changes `codex-rs/native-shim/src/translation.rs` and its unit tests. There are no dependency or external API changes.
