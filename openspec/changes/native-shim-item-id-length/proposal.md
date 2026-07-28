## Why

Model switching can replay an ID emitted by a Chat Completions shim that is too long for a native Responses provider. The provider rejects the whole turn before it reaches the model.

## What Changes

- Omit response-item IDs longer than the Responses API's 64-character maximum at the existing outbound request boundary.
- Add a request-level regression test that preserves short IDs and does not mutate prompt history.

## Capabilities

### New Capabilities
<!-- Each becomes specs/<name>/spec.md. Use kebab-case. -->
- `responses-item-id-length`: Keep provider-incompatible history IDs out of Responses requests.

### Modified Capabilities
None.
- `<existing-name>`: <what requirement is changing>

## Impact

`codex-rs/core/src/client.rs` and the existing websocket client integration suite. No external API shape, dependency, or migration changes.
