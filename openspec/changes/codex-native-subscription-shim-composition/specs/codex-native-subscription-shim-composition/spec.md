## ADDED Requirements

### Requirement: Opt-in composed catalog

The CLI SHALL provide an explicit composed mode that derives one deterministic
catalog from the authenticated native subscription model manager and validated
API-key shim settings. It SHALL preserve all eligible native entries and add
every shim entry exactly once. Each presented entry SHALL have a private typed
route-owner binding of either `native` or `shim`; source labels MAY be visible
but owner bindings SHALL NOT be derived from a display label.

#### Scenario: Both model classes are selectable

- **GIVEN** native subscription models are available and shim settings contain
  two valid API-key routes
- **WHEN** composed mode starts
- **THEN** the picker catalog contains the native entries plus exactly those two
  shim entries, and every entry has one route-owner binding

#### Scenario: Native refresh updates catalog and bindings atomically

- **GIVEN** composed mode has an already published catalog and route registry
- **WHEN** the native model manager refreshes its eligible catalog
- **THEN** the picker observes either the prior valid catalog and bindings or
  one fully validated replacement pair, never a mixed catalog and registry

#### Scenario: Native catalog remains authoritative when shim is absent

- **GIVEN** composed mode is not requested
- **WHEN** direct Codex starts with a native subscription
- **THEN** its catalog and default-model behavior equal the pre-composition
  native path

### Requirement: Route-owned transport and auth isolation

At turn creation the selected model's route owner SHALL determine the provider
and request client. A `native` route SHALL use the existing native
subscription/OAuth path and SHALL NOT contact the shim. A `shim` route SHALL
use only the loopback shim provider and SHALL NOT use native OAuth as fallback.
The shim SHALL NOT receive, persist, log, or derive a subscription token,
subscription base URL, or native request body.

#### Scenario: Subscription selection bypasses the shim

- **GIVEN** composed mode has a live shim runtime and a native subscription
  model is selected
- **WHEN** a turn is dispatched
- **THEN** the native request client receives the turn, shim request count = 0,
  and no shim diagnostics field contains subscription credentials or URL

#### Scenario: API-key selection bypasses native OAuth

- **GIVEN** composed mode has a configured API-key model and native OAuth is
  available
- **WHEN** the API-key model is selected and a turn is dispatched
- **THEN** the loopback shim receives exactly one request for its configured
  upstream route and native OAuth request count = 0

### Requirement: Shim lifecycle and admission remain route-scoped

Composed mode SHALL start the shim eagerly after settings validation and before
the child becomes interactive, so the picker can safely select a shim route
without a first-turn race. It SHALL stop the shim and remove private runtime
state on child exit. Provider admission, queue diagnostics, and API-key route
errors SHALL apply only to `shim` routes; a native subscription route SHALL
not consume or wait for shim capacity.

#### Scenario: Shim readiness precedes API-key dispatch

- **GIVEN** composed mode has valid shim settings
- **WHEN** the child becomes ready to accept a selected shim model
- **THEN** loopback health is ready before the first API-key dispatch

#### Scenario: Subscription traffic does not consume shim capacity

- **GIVEN** a limit-one shim provider has one active streamed request
- **WHEN** a native subscription turn starts
- **THEN** the native turn dispatches without waiting for that provider's lease

### Requirement: Fail-closed catalog identity and configuration

Composed mode SHALL reject duplicate slugs across the native and shim catalogs,
an unavailable/invalid explicit shim settings file, and any unresolved route
owner before launching an interactive child. Its diagnostics SHALL name only
the colliding slug or configuration field, never credentials. It SHALL NOT
silently shadow a native model with a shim model or vice versa.

#### Scenario: Cross-source slug collision fails before child launch

- **GIVEN** a shim route slug equals an eligible native catalog slug
- **WHEN** composed mode validates its catalog
- **THEN** startup fails with an actionable collision diagnostic and no child
  process or shim listener is left running

#### Scenario: Invalid shim settings do not degrade into native-only mode

- **GIVEN** composed mode is explicitly requested and its shim settings are
  missing or invalid
- **WHEN** startup is attempted
- **THEN** it fails before child launch instead of silently removing shim
  routes or mutating native configuration

### Requirement: Session switching is turn-safe and backward compatible

Model selection or switching SHALL resolve route ownership before the next turn
is created and SHALL keep an in-flight turn bound to its original provider.
The existing `codex shim` API-key-only command SHALL remain available with its
current catalog, lifecycle, and no-PATH behavior. Direct bare `codex` SHALL
remain a no-composition path unless the operator explicitly enables composed
mode.

#### Scenario: Switching changes only the next turn

- **GIVEN** a native turn is in flight and the operator selects a shim model
- **WHEN** the native turn completes and the next turn starts
- **THEN** the completed turn remains native-owned and only the next turn is
  dispatched through the shim

#### Scenario: Existing shim-only launcher is unchanged

- **GIVEN** valid API-key shim settings
- **WHEN** the operator invokes existing `codex shim` without composed mode
- **THEN** only shim catalog entries are exposed and the child still launches
  the current executable without resolving `codex` from PATH
