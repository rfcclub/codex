## Why

Composed sessions must keep GPT turns off the local shim even when machine state still selects a legacy shim provider. This avoids accidental protocol translation and keeps subscription-native behavior intact.

## What Changes

- Force the native OpenAI provider when launching a composed child.
- Cover a stale `codex_shim` global provider configuration in the compose CLI integration test.

## Capabilities

### New Capabilities
<!-- Each becomes specs/<name>/spec.md. Use kebab-case. -->
- `compose-native-gpt-provider`: Native provider startup policy for composed sessions.

### Modified Capabilities
None.
- `<existing-name>`: <what requirement is changing>

## Impact

`codex-rs/cli/src/main.rs` and `codex-rs/cli/tests/shim.rs`; no public API or dependency changes.
