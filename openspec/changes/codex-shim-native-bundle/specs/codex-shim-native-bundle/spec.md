## ADDED Requirements

### Requirement: Explicit native settings

The native shim SHALL read only an explicit settings file, defaulting to
`~/.codex-shim/models.json`. It SHALL validate preferred provider/model entries
and the documented safe legacy subset before binding a listener. It SHALL NOT
write `~/.codex/config.toml`, mutate legacy settings, invoke Python, or fall
back to a subscription route.

#### Scenario: Valid API-key settings load

- **WHEN** settings contain a valid API-key provider entry and one model route
- **THEN** the resolved configuration contains that model exactly once

#### Scenario: Unsupported route fails before serving

- **WHEN** settings contain a ChatGPT/Cursor subscription-passthrough route
- **THEN** startup fails with an actionable unsupported-route diagnostic and no listener

### Requirement: Provider-entry admission

The shim SHALL key admission by non-secret provider entry id, not model slug or
credential. An explicit queue policy SHALL enforce a bounded FIFO wait with
positive active, queued, and timeout values. A lease SHALL remain held through
the upstream stream and release exactly once on success, error, cancellation,
or client disconnect. Providers with no queue policy SHALL have no shared cap.

#### Scenario: One provider cannot consume another provider budget

- **WHEN** a limit-one provider has a live stream and a second provider has capacity
- **THEN** the second provider dispatches without waiting behind the saturated provider

#### Scenario: Saturated bounded queue is rejected

- **WHEN** active capacity and configured queue capacity are both occupied
- **THEN** the next request is rejected with retryable HTTP 429 without upstream dispatch

### Requirement: Catalog and child isolation

The `codex shim` command SHALL derive a non-empty catalog accepted by Codex's
existing model catalog parser, bind a loopback runtime, and spawn the current
`codexia` executable with catalog/base-URL overrides only for the child. It
SHALL NOT resolve a `codex` executable from PATH. Direct `codex-patched` use
SHALL remain unchanged.

#### Scenario: Launcher starts current executable

- **WHEN** `codexia shim` is invoked with valid settings
- **THEN** it starts a loopback runtime and launches the current executable with the generated catalog

#### Scenario: Child exit cleans runtime state

- **WHEN** the child Codex process exits
- **THEN** the server stops and private runtime files are removed

### Requirement: Loopback protocol relay

The shim SHALL bind only `127.0.0.1` and serve `POST /v1/responses`,
`POST /v1/chat/completions`, and `POST /v1/messages` for configured API-key
OpenAI-compatible and Anthropic-compatible routes. It SHALL substitute the
resolved upstream model, translate request/SSE shapes where the ingress and
provider protocol differ, and preserve streaming semantics.

#### Scenario: OpenAI Responses route is relayed

- **WHEN** Codex sends `/v1/responses` for a configured OpenAI-compatible model
- **THEN** the shim forwards the selected upstream model and streams the upstream response

#### Scenario: Anthropic route is translated

- **WHEN** Codex sends a supported ingress shape for a configured Anthropic model
- **THEN** the shim translates to Messages and returns semantically equivalent streaming events

### Requirement: Secret-safe diagnostics

The shim SHALL expose loopback health and non-secret diagnostics. Diagnostics,
logs, errors, and queue metrics SHALL exclude API keys, authorization headers,
request bodies, and credential-bearing URLs.

#### Scenario: Diagnostics redact credentials

- **WHEN** a configured provider has an API key and active request
- **THEN** health and diagnostics identify only route/provider metadata and counts
