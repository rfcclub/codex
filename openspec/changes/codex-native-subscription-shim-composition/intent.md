# Intent: codex-native-subscription-shim-composition

## Raw Request

"chung ta phai support subscription va shim luon nhe" — support native
OpenAI/ChatGPT subscription models and API-key shim models in one Codex model
selection surface.

## Problem

The committed `codex shim` launcher replaces the child process's entire model
catalog and provider with one loopback API-key provider. It deliberately
rejects ChatGPT/Cursor subscription routes. Consequently an operator must
choose either direct native subscription Codex or API-key shim mode at process
startup, rather than choosing either model class in one session.

## Desired Outcome

One explicitly launched Codex session can list and select both native
subscription models and configured API-key models. Selecting a subscription
model keeps the existing native OAuth/ChatGPT request path; selecting an
API-key model uses the loopback native shim and its provider admission policy.
No subscription credential is read, copied, proxied, or exposed by the shim.

## Users / Actors

- Operator running the patched Codex CLI with an existing native subscription.
- Codex model picker, model-resolution/config layers, and native OAuth client.
- Native API-key shim runtime and configured external provider accounts.

## Current Context

- `4b6bfb0561` added `codex-native-shim` and `codex shim`; it creates a
  private API-key-only catalog and sets one child-wide `model_provider`.
- `model_catalog_json` replaces the bundled catalog for a process
  (`codex-rs/core/src/config/mod.rs`), so catalog union cannot be achieved by
  simply adding a second file.
- The native subscription path is an existing Codex responsibility. Its OAuth
  and ChatGPT base URL behavior are not a shim transport contract.
- Native shim settings reject subscription-passthrough routes before binding.

## Proposed Direction

Introduce an internal model-route registry owned by Codex model resolution.
The registry unions bundled/native subscription entries with shim entries and
binds each selectable model to exactly one transport owner: `native` or
`shim`. The selected route determines request client/provider configuration at
turn creation. The shim remains loopback-only and receives only API-key route
traffic; direct subscription traffic never crosses it.

## Scope

- Define the composed catalog and route-ownership contract.
- Preserve native subscription discovery, login, auth refresh, and transport.
- Start the shim only when an API-key route is selected or eagerly when needed
  for picker availability, subject to the lifecycle decision in the spec.
- Keep API-key per-provider concurrency, diagnostics, and protocol translation
  scoped to shim-owned routes.
- Define model-slug collision, unsupported route, switch, startup, error, and
  rollback behavior.
- Add composition tests covering catalog, provider selection, auth isolation,
  switching, and shim lifecycle.

## Non-Goals

- Proxying ChatGPT/Codex OAuth credentials through the shim.
- Reimplementing subscription auth, web login, or desktop UI flows.
- Adding Cursor subscription passthrough; it remains separately designed.
- Distributed provider concurrency across processes.

## Constraints

- A subscription route must remain byte-for-byte on the native request path;
  the shim must never receive its headers, token, request body, or base URL.
- A configured API-key route must never cause native OAuth fallback.
- Direct bare `codex` behavior and existing stored subscription sessions remain
  unchanged unless the operator explicitly enables composition.
- Existing `codex shim` remains a compatible API-key-only escape hatch during
  migration and rollback.
- Catalog identifiers must be deterministic and collision-safe; user-visible
  labels must disclose source without leaking account or credential details.

## Success Criteria

- A composed session lists native subscription and API-key models together.
- Selecting each kind produces exactly its intended request transport.
- A shim API key, native OAuth token, and subscription base URL cannot appear
  in the other route's requests, logs, diagnostics, or catalog payload.
- Selecting a shim model enforces its configured provider budget; selecting a
  subscription model is not subject to shim admission.
- Removing the composed-mode flag restores pre-change direct Codex behavior.

## Risks

- A process-wide provider setting may be baked into session/client creation and
  make per-model routing non-local.
  - Mitigation: trace config ownership before implementation; add a typed route
    binding rather than mutating global config while a turn is running.
- Catalog replacement can hide bundled models or create ambiguous slugs.
  - Mitigation: use a deterministic union with explicit collision diagnostics.
- Starting the shim only after selection can race the first API-key turn.
  - Mitigation: define readiness and lifecycle ownership with a tested state
    machine.
- A broad compatibility layer could accidentally proxy subscription traffic.
  - Mitigation: test a negative boundary asserting the shim never observes a
    native subscription request.

## Ambiguities

### Blocking

- Whether shim runtime is eager for every composed session or lazy on first
  shim selection; this determines picker, readiness, and failure semantics.
- Which native subscription model catalog is authoritative when offline or
  unrefreshed, and whether composition can start without a valid subscription.
- Collision policy for a configured shim slug matching a bundled/native slug.

### Non-Blocking

- Subscription models do not receive shim concurrency policy in this change.
- A grouped picker UI can follow after the correct internal route contract;
  source labels are sufficient initially.

## Assumptions

- The existing model catalog can carry route-source metadata or a parallel
  private registry can bind catalog slugs to provider owners without changing
  the public server schema immediately.
- An opt-in composition launcher/flag is acceptable for the first release; it
  avoids changing direct Codex behavior before migration is proven.

## Spec Seeds

These are candidate requirements for the spec phase. Not binding. Derived from Desired Outcome, Constraints, Risks, and Success Criteria.

- GIVEN a valid native subscription and valid shim settings, WHEN composed mode
  starts, THEN its catalog contains both native and shim-owned entries exactly
  once with deterministic source labels.
- GIVEN a native subscription model is selected, WHEN a turn starts, THEN the
  native client handles auth and network I/O and the shim receives no request.
- GIVEN a shim-owned model is selected, WHEN a turn starts, THEN the loopback
  shim is ready before dispatch and its provider policy admits the request.
- GIVEN the same slug exists in both sources, WHEN composed mode validates,
  THEN startup fails with an actionable collision diagnostic before a turn.
- GIVEN composed mode is disabled, WHEN direct Codex starts, THEN its catalog,
  auth, and provider behavior match the pre-change native path.

## Intent Approval

Status: APPROVED

Approved by: Thoor
Date: 2026-07-25
