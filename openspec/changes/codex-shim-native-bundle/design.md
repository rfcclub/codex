## Architecture

`codex-native-shim` is a workspace crate independent of `codex-core`. It owns
settings validation, catalog derivation, routing, protocol translation,
loopback serving, and admission. The CLI owns the process lifecycle.

`codex shim [--settings PATH] [--port PORT] [-- <normal-codex-args>]` is a
parent process. It validates settings before binding, creates a private runtime
directory/catalog, starts the loopback proxy, then spawns `current_exe` with the
normal Codex arguments and child-only overrides:

```
model=<default slug>
model_provider=codex_native_shim
model_catalog_json=<private runtime catalog>
model_providers.codex_native_shim.base_url=http://127.0.0.1:<port>/v1
model_providers.codex_native_shim.wire_api=responses
```

The parent forwards termination and exit status, stops the server on child exit,
and removes the runtime directory. It never resolves `codex` from `PATH`.
Direct `codex-patched` remains a no-shim path with no injected config.

## Settings Contract

The canonical path is `~/.codex-shim/models.json`; `--settings PATH` is an
explicit alternative. Native code only reads it: it does not write
`~/.codex/config.toml`, invoke Python, or alter the legacy shim.

Preferred schema separates an account identity from transport protocol:

```json
{
  "providers": [{
    "id": "cheapinference",
    "protocol": "openai-responses",
    "base_url": "https://provider.example/v1",
    "api_key_env": "CHEAPINFERENCE_API_KEY",
    "concurrency": { "max_active": 1, "max_queued": 8, "wait_timeout_ms": 30000 }
  }],
  "models": [{
    "slug": "cheap-sonnet",
    "upstream_model": "sonnet",
    "provider": "cheapinference",
    "display_name": "Cheap Sonnet"
  }]
}
```

`provider.id` is the concurrency key; models sharing it share the account
budget. A provider without `concurrency` is unlimited. All three concurrency
values are positive. A full queue returns HTTP 429; a timed-out waiter returns
HTTP 503. Tokio's fair semaphore is wrapped by a bounded FIFO queue so queue
length/timeout are observable without exposing secrets.

Legacy flat rows are accepted only for API-key `openai` and
`generic-chat-completion-api` entries. Each becomes a separate provider entry
unless it declares `provider_entry_id`; that field makes legacy models share a
budget. Anthropic flat rows normalize to `anthropic-messages`. ChatGPT/Cursor
or other subscription rows fail before the listener binds.

## Runtime Contract

The service always binds `127.0.0.1`; `--port` selects a port but never an
address and defaults to ephemeral. `GET /healthz` is ready only after settings
and catalog validate. `GET /diagnostics` exposes route slugs, provider ids,
active/queued count, and limits only—never API keys, Authorization headers,
request bodies, or credential-bearing URLs.

Each request resolves its catalog slug, acquires the provider lease immediately
before dispatch, and retains it until upstream stream completion, failure, or
downstream disconnect. An owned lease guarantees exactly one release via Drop.

| Ingress | OpenAI-compatible provider | Anthropic provider |
|---|---|---|
| `/v1/responses` | forward after model substitution | translate to Messages and SSE back to Responses |
| `/v1/chat/completions` | forward after model substitution | translate through Messages and emit chunks |
| `/v1/messages` | translate to Responses and emit Anthropic events | forward after model substitution |

Unsupported request shapes return typed 400 before admission. Credentials are
in-memory sensitive headers excluded from tracing.

## Components

- `settings`: preferred schema, legacy normalization, and fail-closed errors.
- `catalog`: a valid Codex `ModelsResponse` for every resolved model route.
- `router`: slug lookup/upstream-model substitution/provider selection.
- `admission`: bounded FIFO provider-entry queues and owned stream leases.
- `server`: Axum loopback endpoints, health, diagnostics, graceful shutdown.
- `translation`: OpenAI Responses, Chat Completions, Anthropic Messages/SSE adapters.
- `launcher`: CLI subcommand, private files, child overrides, signal lifecycle.

## Data Model

`ProviderEntry` has unique non-secret id, protocol, base URL, credential
reference, and optional queue policy. `ModelRoute` has unique catalog slug,
display metadata, upstream model, and provider id. Resolved routes retain keys
privately and never implement secret-revealing `Debug`.

## Test Strategy

| Scenario | Test | Type |
|---|---|---|
| settings/migration | `native-shim/src/settings_tests.rs` | unit |
| catalog compatibility | `native-shim/src/catalog_tests.rs` | unit |
| FIFO/timeout/release | `native-shim/src/admission_tests.rs` | async unit |
| Responses relay | `native-shim/tests/responses_relay.rs` | integration |
| Chat relay | `native-shim/tests/chat_relay.rs` | integration |
| Messages relay | `native-shim/tests/messages_relay.rs` | integration |
| protocol fixture parity | `native-shim/tests/translation_fixtures.rs` | integration |
| current-exe/cleanup | `cli/tests/shim.rs` | integration |
| secret-safe diagnostics | `native-shim/tests/diagnostics.rs` | integration |

## Dependencies

Use workspace `axum`, `tokio`, `reqwest`, `serde`, and protocol model types.
Any new dependencies require Cargo and Bazel lockfile updates.

## Migration

No automatic migration. Legacy path is a compatibility default; `codex shim
doctor` reports preferred-schema migration actions. Unsupported entries fail
closed; direct Codex behavior remains unchanged.
