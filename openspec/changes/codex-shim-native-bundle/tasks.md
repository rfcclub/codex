# Implementation Plan: codex-shim-native-bundle

## Preparation

- [x] Approve intent, scope, and non-goals.
- [x] Specify parent/child launcher, settings, protocol, and queue contracts.

## Tasks

### Task 1: Settings, migration, and typed catalog

- [x] Add initial RED tests for API-key settings, duplicate slugs, and subscription rejection.
- [x] Add initial read-only parser and provider-isolation foundation.
- [x] Add RED tests for provider/model schema, shared legacy `provider_entry_id`, invalid queue policy, and Anthropic rows.
- [x] Implement normalization and generate a catalog accepted by the existing `ModelsResponse` parser.
- [x] Run `just test -p codex-native-shim`.

### Task 2: Bounded admission manager

- [x] Add RED provider-isolation test and owned lease implementation.
- [x] Add RED FIFO order, queue-full, timeout, cancellation, stream-error, and exactly-once-release tests.
- [x] Implement bounded queue, timed acquisition, and secret-safe diagnostics.
- [x] Run `just test -p codex-native-shim`.

### Task 3: OpenAI Responses relay

- [x] Add mock-upstream tests for streaming, model substitution, and client disconnect.
- [x] Implement loopback `/v1/responses`, health, sensitive headers, and lease lifetime.
- [x] Run focused integration tests.

### Task 4: Chat Completions and Anthropic Messages adapters

- [x] Add checked-in request/SSE fixture pairs for every cross-protocol SSE translation direction.
- [x] Implement `/v1/chat/completions`, `/v1/messages`, and cross-protocol SSE adapters.
- [x] Run fixture and relay integration tests.

### Task 5: CLI launcher and release wiring

- [x] Add launcher tests proving `codex shim` uses `current_exe`, child-only overrides, exit forwarding, runtime cleanup, and an end-to-end child-to-upstream Responses request.
- [x] Implement the CLI subcommand, runtime files, health wait, lifecycle, and `shim doctor` diagnostics.
- [x] Update Bazel metadata; build `codexia` and prove it retains trusted Mesh patch plus shim command.
- [x] Run focused tests and `just fix -p codex-native-shim`.

## Verification

- [x] All requirement scenarios, including protocol fixture parity, pass.
- [x] `loomkit verify codex-shim-native-bundle` passes.
- [x] Fresh-artifact smoke proves no Python/Go runtime dependency.
