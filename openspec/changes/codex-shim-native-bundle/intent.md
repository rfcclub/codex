# Intent: codex-shim-native-bundle

## Raw Request

Vendored `codex-shim` must become part of the `codexia` distribution, and its
per-provider concurrency limit must include the main foreground turn. The
approved direction is a native Rust implementation, not a Python sidecar or a
Go port.

## Problem

The current vendored `codex-shim` is a Python/aiohttp local compatibility proxy.
Its 6.8k lines implement HTTP/SSE translation, routing, model catalog/selection,
and several provider paths. `codex-patched` is a Rust executable, so merely
including the source in the `codexia` branch does not make it runnable: Python,
aiohttp, and their runtime environment are still required.

Separately, requests reach a selected provider without a shared admission gate.
This can exceed an account's concurrency budget when a foreground/main turn and
another session or parallel task use that provider at once.

## Desired Outcome

The `codexia` release binary contains a native Rust shim runtime. When enabled,
it owns a loopback Responses-compatible proxy, routes configured external models,
and launches/uses the patched Codex client against that proxy. No Python, Go, or
editable installation is required at runtime.

Only an explicitly configured provider concurrency budget admits native upstream
dispatches through a bounded gate. That gate includes the main foreground turn.
For example, `umans=4` and `cheapinference=1` are independent budgets; a
saturated configured provider cannot delay another provider. Providers with no
configured limit are not gated.

## Users / Actors

- Operator running the `codexia` CLI with a configured external model.
- The patched Codex CLI, speaking the existing local Responses-compatible
  provider contract.
- The native shim runtime: route resolver, protocol translators, streaming
  relay, and configured-provider concurrency manager.
- External OpenAI-compatible and Anthropic-compatible provider accounts.

## Current Context

- `/Users/thoor/repo/codex` branch `codexia` carries `codex-shim/` as vendored
  Python reference source, plus the trusted Colony Mesh MCP notification patch.
- The built patched executable is `codex-rs/target/release/codex`; the local
  `codex-patched` alias points to it.
- Python shim ingress supports `/v1/responses`, `/v1/responses/compact`,
  `/v1/chat/completions`, and `/v1/messages`, then resolves a configured route
  before forwarding/streaming upstream.
- `umans-dash-go` is a reference for bounded FIFO queue accounting and release
  on completion; it is not a deployment/runtime candidate.
- The existing Python-only ChatGPT and Cursor subscription passthrough paths
  depend on distinct local authentication/CLI integrations and are not safe to
  claim as native parity before their contracts are specified.

## Proposed Direction

Create a Rust workspace crate for the native shim and link it into the `codexia`
CLI distribution. It will run a loopback-only HTTP/SSE proxy inside the same
release executable/process lifecycle, use native async HTTP clients for upstream
requests, and expose a deliberate launcher/service contract.

Native v1 is specifically API-key external models: configured OpenAI-compatible
and Anthropic-compatible routes; model catalog/selection; Responses and the two
compatibility endpoint shapes; streaming translation; and opt-in provider
concurrency admission. The Python shim remains a reference implementation during
the migration, not a runtime dependency. Subscription passthrough is explicitly
deferred until independently designed.

## Scope

- Add a native Rust shim crate to the Codex workspace and package it in the
  `codexia` release binary.
- Define `codexia` launcher/runtime ownership: startup, loopback port
  allocation, readiness, binary selection, shutdown, and diagnostics.
- Implement native API-key model configuration and catalog/selection compatible
  with the required subset of existing settings.
- Implement `/v1/responses`, `/v1/chat/completions`, and `/v1/messages`
  translation/streaming for OpenAI-compatible and Anthropic-compatible routes.
- Implement an opt-in configured-provider concurrency manager with bounded FIFO
  queue, timeout/cancellation semantics, and exactly-once lease release at
  stream end.
- Make provider entry ID the concurrency budget key: all model variants routed
  through one configured provider entry share its explicit limit.
- Ensure the main foreground request passes through the same manager as all
  other upstream work.
- Expose non-secret active/queued/rejected metrics and native diagnostics.
- Establish parity tests against selected Python-shim fixtures plus native
  integration tests for streams, cancellation, and provider isolation.

## Non-Goals

- Porting the shim to Go or shipping a Go sidecar.
- Bundling CPython, aiohttp, or a Python virtual environment in `codexia`.
- Reimplementing ChatGPT subscription passthrough, Cursor CLI passthrough,
  picker web UI, or macOS Desktop ASAR patching in native v1.
- Automatically discovering upstream provider concurrency limits.
- Distributed coordination across multiple native shim processes/machines in v1.
- Altering the Colony Mesh MCP notification/A2A carrier semantics.

## Constraints

- One `codexia` executable/distribution must be sufficient at runtime; it must
  not invoke a PATH `codex`, Python interpreter, or Go binary.
- The proxy binds loopback only and preserves no-proxy behavior for local calls.
- Credentials stay in user-controlled secure/local configuration and never
  appear in logs, diagnostics, queue keys, or release artifacts.
- There is no global concurrency cap. A provider without an explicit positive
  limit retains unlimited concurrency, including future ChatGPT/Cursor paths.
- The concurrency budget key is the non-secret configured provider entry ID;
  provider credentials are never used as keys or exposed in metrics.
- Existing direct `codex-patched` use remains an explicit no-shim path.

## Success Criteria

- A fresh machine with only the codexia release artifact can configure an
  API-key external model and run it through the native shim; no Python install
  is involved.
- The native shim routes and streams a configured OpenAI-compatible model and a
  configured Anthropic-compatible model through the documented endpoints.
- With `cheapinference` limited to one, a streaming main request through that
  provider prevents a second CheapInference request from dispatching; a request
  through `umans` still dispatches until its own limit of four is reached.
- Queue cancellation, upstream failure, client disconnect, and normal stream
  completion each release the lease exactly once.
- Missing/corrupt native shim configuration fails before a stock Codex binary
  could be launched, with an actionable diagnostic.
- The codexia binary selected by the launcher is the same branch build carrying
  the Colony Mesh notification patch.

## Risks

- Protocol translation and SSE edge cases can drift from Python behavior.
  - Mitigation: define fixture-based compatibility cases endpoint by endpoint;
    migrate one protocol path at a time behind native integration tests.
- A broad "feature parity" rewrite would become untestable and delay usable
  native API-key models.
  - Mitigation: v1 deliberately excludes subscription passthrough/Desktop UI;
    create separate intents if those features are still needed.
- Incorrect lease lifetime can undercount active streams or deadlock capacity.
  - Mitigation: model a lease as an owned Rust guard whose release path is
    tested for success, error, disconnect, timeout, and cancellation.
- Config migration could unexpectedly alter existing Python users.
  - Mitigation: native config reads an explicit compatible subset and reports
    unsupported fields; it never mutates legacy settings automatically.

## Ambiguities

### Blocking

- What is the canonical native settings file/path and exact migration policy from
  `~/.codex-shim/models.json`? Read-compatible first is proposed, but needs
  approval before spec.
- For a saturated configured provider, should v1 queue FIFO until a configurable timeout,
  immediately return a retryable error, or allow both policies?
- Does native v1 need to serve Codex Desktop immediately, or is codexia CLI the
  sole supported client until a separate Desktop integration decision?

### Non-Blocking

- Metrics can begin in structured diagnostics/health output; a dashboard is not
  needed for v1.
- The former Python source can remain vendored temporarily as migration reference
  until native fixture parity is established.

## Assumptions

- Tokio plus the existing Codex Rust dependency ecosystem can supply the local
  server, upstream client, and SSE streaming primitives without adding a second
  runtime language.
- Per-process accounting is sufficient for the intended one-codexia-runtime v1.
- Provider limits are opt-in and attached to provider entries, such as
  `umans=4` and `cheapinference=1`; ChatGPT/Cursor have no concurrency policy.
- Model selection only needs API-key configured external models in native v1;
  subscription passthrough is a future product decision, not hidden scope.

## Spec Seeds

These are candidate requirements for the spec phase. Not binding.

- GIVEN no override is set, WHEN `codexia` native-shim mode starts, THEN it uses
  the codexia release binary and starts a loopback-only native shim without a
  Python or Go executable.
- GIVEN a configured OpenAI-compatible route, WHEN Codex sends `/v1/responses`,
  THEN the native shim relays a semantically equivalent streaming response.
- GIVEN a configured Anthropic-compatible route, WHEN Codex sends a compatible
  request, THEN the native shim translates and streams the response through the
  required Codex-facing endpoint.
- GIVEN `cheapinference` has a concurrency limit of one and its main request is
  streaming, WHEN another request resolves through CheapInference, THEN that
  request is not dispatched before the lease ends or its configured wait policy
  expires.
- GIVEN `cheapinference` is saturated and `umans` has remaining capacity up to
  its configured limit of four, WHEN requests arrive for both, THEN Umans
  dispatches without waiting behind CheapInference.
- GIVEN a queued request is cancelled, WHEN capacity becomes free, THEN it does
  not occupy a queue position or active lease.
- GIVEN a legacy Python-only subscription-passthrough model is selected, WHEN
  native v1 loads settings, THEN it reports that model as unsupported rather
  than silently routing it incorrectly.

## Intent Approval

Status: APPROVED

Approved by: Thoor
Date: 2026-07-25
