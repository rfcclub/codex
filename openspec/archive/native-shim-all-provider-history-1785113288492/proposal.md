## Why

Structured Responses history works for the Chat Completions adapter but fails
for Anthropic routes. Custom model behavior must not depend on which upstream
provider Thoor selected.

## What Changes

- Translate structured Responses history to Anthropic textual messages.
- Add cross-provider regression coverage for the common tool-history shape.

## Capabilities

### New Capabilities
<!-- Each becomes specs/<name>/spec.md. Use kebab-case. -->
- `native-shim-all-provider-history`: Support one structured turn-history
  contract across every native-shim provider protocol.

### Modified Capabilities
<!-- Existing specs whose REQUIREMENTS change. Leave empty if none. -->
- None.

## Impact

- `codex-rs/native-shim/src/translation.rs`
- `codex-rs/native-shim/src/translation_tests.rs`
- No config, dependency, or credential changes.
