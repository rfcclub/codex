## Why

`codex shim` currently replaces the process catalog and provider, so API-key
models work only by giving up the native OpenAI/ChatGPT subscription route.
Operators need one explicit mode where the selector can safely choose either
route without proxying subscription credentials through the API-key shim.

## What Changes

- Add an opt-in composed launcher/mode that exposes a deterministic union of
  native subscription models and configured native-shim models.
- Bind every selected model to a typed route owner (`native` or `shim`) at turn
  creation instead of relying on one process-wide `model_provider`.
- Preserve native OAuth auth/transport for subscription routes; start and use
  the loopback shim only for API-key routes.
- Add collision diagnostics, lifecycle/error semantics, and regression tests
  proving credential and transport isolation.

## Capabilities

### New Capabilities
- `native-subscription-shim-composition`: Composed model catalog, route
  ownership, safe dispatch, and lifecycle for native subscription plus shim
  API-key models.

### Modified Capabilities
- `codex-shim-native-bundle`: The API-key launcher gains an opt-in composed
  mode while its existing shim-only mode remains compatible.

## Impact

- `codex-rs/cli/src/main.rs`: composed-launcher command and lifecycle.
- `codex-rs/core/src/config/mod.rs`, session configuration, and model manager:
  catalog union plus per-selected-model provider binding.
- `codex-rs/models-manager/src/manager.rs`: composed catalog/picker source.
- `codex-rs/native-shim`: route registry input and loopback lifecycle only;
  no subscription auth code.
- No credential migration, new runtime language, or external service.
