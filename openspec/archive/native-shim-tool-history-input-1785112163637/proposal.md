## Why

Patched Codex sends structured Responses history after tool execution, but the
native Chat Completions adapter rejects a `function_call_output` item. This
prevents Xiaomi Mimo from completing ordinary turns after the first tool call.

## What Changes

- Translate Responses `function_call_output` entries into text-bearing user
  messages with the call id and result.
- Add a regression test covering mixed tool history passed to a Chat
  Completions provider.

## Capabilities

### New Capabilities
- `native-shim-tool-history-input`: Preserve tool-result context when routing
  a Responses request through a text-only Chat Completions provider.

### Modified Capabilities
<!-- Existing specs whose REQUIREMENTS change. Leave empty if none. -->
- None.

## Impact

- `codex-rs/native-shim/src/translation.rs`
- `codex-rs/native-shim/src/translation_tests.rs`
- No dependency, credential, or public-config changes.
