# Implementation Plan: codex-native-subscription-shim-composition

## Preparation

- [ ] Re-read `specs/codex-native-subscription-shim-composition/spec.md` and
  `design.md`; map each scenario to the test named below.
- [ ] Preserve the current `codex shim` flow as the regression baseline before
  adding `--compose`; do not alter direct bare `codex` behavior.

## Tasks

### Task 1: Add a validated, credential-free shim-extension manifest

**Files:**
- Modify: `codex-rs/native-shim/src/settings.rs`
- Modify: `codex-rs/native-shim/src/lib.rs`
- Test: `codex-rs/native-shim/src/settings_tests.rs`

- [ ] **Step 1: Write failing tests** for serializing a validated route into a
  shim extension, rejecting an empty or duplicate extension slug, and proving
  its JSON contains no API key or upstream authorization value.

  Run: `cargo test -p codex-native-shim settings_tests`
  Expected: FAIL because the manifest type and validation do not exist.

- [ ] **Step 2: Implement the smallest manifest type and conversion** from
  `NativeSettings`. Include only picker metadata, route slug, loopback provider
  identity, and non-secret admission metadata; make credential-bearing fields
  impossible to serialize into the manifest.

  Run: `cargo test -p codex-native-shim settings_tests`
  Expected: PASS

### Task 2: Compose the native manager with the shim extension atomically

**Files:**
- Create: `codex-rs/models-manager/src/composed_catalog.rs`
- Modify: `codex-rs/models-manager/src/lib.rs`
- Modify: `codex-rs/models-manager/src/manager.rs`
- Test: `codex-rs/models-manager/src/composed_catalog_tests.rs`

- [ ] **Step 1: Write failing unit tests** for native-plus-shim union,
  cross-source collision rejection, owner binding for every result, and an
  atomic native refresh (valid new catalog replaces both catalog and routes;
  collision preserves the previous valid snapshot).

  Run: `cargo test -p codex-models-manager composed_catalog_tests`
  Expected: FAIL because `ComposedModelsManager` does not exist.

- [ ] **Step 2: Implement `ComposedModelsManager`** as a wrapper around the
  existing authenticated native manager. It must merge fresh eligible native
  models with immutable validated shim entries in deterministic slug order and
  publish an immutable catalog/route-registry snapshot only after validation.

  Run: `cargo test -p codex-models-manager composed_catalog_tests`
  Expected: PASS

### Task 3: Resolve the selected model to a turn-local provider snapshot

**Files:**
- Create: `codex-rs/core/src/composed_model_routes.rs`
- Modify: `codex-rs/core/src/config/mod.rs`
- Modify: the concrete session/client construction module identified by the
  existing `model_provider` lookup
- Test: `codex-rs/core/src/composed_model_routes_tests.rs`
- Test: `codex-rs/core/tests/suite/composed_model_routing.rs`

- [ ] **Step 1: Write failing tests** for unknown/ambiguous owners, native
  selection making no shim call, shim selection making no native OAuth call,
  and switching models while a turn is active leaving that turn's provider
  unchanged.

  Run: `cargo test -p codex-core composed_model_routes_tests --test suite composed_model_routing`
  Expected: FAIL because route lookup and per-turn snapshots do not exist.

- [ ] **Step 2: Add private composed-mode config plumbing and route resolver.**
  Read only the manifest path supplied by the launcher; retain the normal
  native auth manager and resolve a native or `codex_native_shim` provider at
  next-turn creation. Store the resolved provider in the turn configuration,
  never by mutating process-wide `Config.model_provider`.

  Run: `cargo test -p codex-core composed_model_routes_tests --test suite composed_model_routing`
  Expected: PASS

### Task 4: Add the opt-in launcher without changing shim-only behavior

**Files:**
- Modify: `codex-rs/cli/src/main.rs`
- Modify: `codex-rs/cli/src/cli.rs` (or the actual `ShimCommand` definition)
- Test: `codex-rs/cli/tests/shim.rs`

- [ ] **Step 1: Write failing integration tests** for `codex shim --compose`
  validating settings, starting `/healthz` before child interactivity, passing
  only a temporary shim-extension path and loopback base URL, cleaning both up
  after child exit, and failing collision/invalid-settings before child launch.
  Retain existing no-flag shim test expectations unchanged.

  Run: `cargo test -p codex-cli --test shim`
  Expected: FAIL because `--compose` and its lifecycle do not exist.

- [ ] **Step 2: Implement `--compose` as a separate branch** in
  `run_shim_command`: validate settings, create the non-secret manifest,
  start and health-check the API-key-only runtime, launch the current
  executable with private composed-mode overrides, and use one cleanup guard
  for every child-exit path. Keep the existing no-flag overrides untouched.

  Run: `cargo test -p codex-cli --test shim`
  Expected: PASS

### Task 5: Verify admission is isolated to shim routes and regressions hold

**Files:**
- Modify: `codex-rs/native-shim/src/relay_tests.rs`
- Test: `codex-rs/native-shim/src/relay_tests.rs`
- Test: existing native auth/model-manager tests affected by Tasks 2–3

- [ ] **Step 1: Write failing relay coverage** showing a saturated
  limit-one shim route queues only a second shim route while a native turn is
  dispatched without acquiring shim capacity.

  Run: `cargo test -p codex-native-shim relay_tests`
  Expected: FAIL because the composed-route boundary has no assertion.

- [ ] **Step 2: Add only the test hooks needed** to observe the existing
  admission boundary; do not add OAuth parsing, proxying, or subscription
  credentials to `native-shim`.

  Run: `cargo test -p codex-native-shim relay_tests && cargo test -p codex-models-manager && cargo test -p codex-core`
  Expected: PASS

## Verification

- [ ] Run the targeted suite from each task and record exact commands/results
  in the implementation PR or handoff.
- [ ] Run `loomkit verify codex-native-subscription-shim-composition`; every
  requirement/scenario must have linked tests and 100% traceability.
- [ ] Manually inspect captured mock requests: native-route tests show zero
  shim requests, shim-route tests show zero native OAuth requests, and no
  assertion/log fixture includes subscription material or API keys.
- [ ] Confirm `codex shim` without `--compose` and bare `codex` retain their
  existing startup command, catalog ownership, and default-model semantics.
