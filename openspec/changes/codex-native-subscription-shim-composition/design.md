## Architecture

### Decision

Add `codex shim --compose` as an opt-in parent launcher. The parent validates
API-key shim settings, starts the loopback runtime eagerly, and writes one
private, credential-free shim-extension manifest for its child. The manifest
contains validated shim picker entries and their non-secret loopback bindings;
it contains no native catalog, OAuth material, or API keys.

The child keeps its normal authenticated native `ModelsManager` and wraps it in
`ComposedModelsManager`. On initial load and every native catalog refresh, that
manager merges the fresh eligible native catalog with the immutable shim
extension and builds a matching `ComposedRouteRegistry` atomically. The child
gets only the manifest path through an explicit config override. It does
**not** get a replacement process-wide shim provider. Instead, a turn
configuration builder resolves the selected slug against the registry before
creating that turn's client. The builder snapshots the resulting native or
shim provider into the turn configuration; changing the selected model cannot
mutate a client already owned by an in-flight turn.

```
codex shim --compose
  ├─ validate ~/.codex-shim/models.json
  ├─ start loopback NativeShimRuntime (eager /healthz ready)
  └─ write shim-extension manifest ────────────────┐ child-only path
                                                    ▼
                                  child native ModelsManager (normal auth/filter)
                                                    │ native refresh
                                                    ▼
                                  ComposedModelsManager + shim extension
                                                    │ atomic catalog + registry
                                                    ▼
                                      picker selects slug / next turn begins
                                                       │
                           ┌───────────────────────────┴──────────────────────────┐
                           ▼                                                      ▼
                  RouteOwner::Native                                      RouteOwner::Shim
                  existing OAuth client                                   loopback provider
                  no shim request                                         admission + relay
```

The native catalog remains the source of truth for native eligibility and
auth filtering. The shim catalog remains the source of truth for validated
API-key routes. Neither source gets rewritten to impersonate the other.

### Alternatives rejected

- **Proxy native subscription through the shim:** violates OAuth boundary and
  changes an existing trusted transport without a defined contract.
- **Use `model_catalog_json` alone:** core intentionally treats it as a full
  catalog replacement, so it would bypass the native manager's authentication
  filtering and refresh semantics; it also cannot attach transport ownership.
- **Switch process-wide `Config.model_provider` on picker selection:** races
  in-flight/parallel turns and leaks route state across session clients.
- **Lazy-start the shim on first request:** adds first-turn readiness/failure
  ambiguity after a user has selected a model. Eager startup keeps failure at
  explicit composed-mode startup.

## Components

| Component | Location | Change |
|---|---|---|
| composed CLI mode | `codex-rs/cli/src/main.rs` | Add `--compose` to `ShimCommand`; validate and serialize a shim-extension manifest, start/health-check runtime, pass its child-only override, and clean up on exit. Existing no-flag shim flow stays intact. |
| composed catalog manager | `codex-rs/models-manager/src/manager.rs` plus a new `composed_catalog.rs` | Wrap the authenticated native manager; merge each fresh eligible native catalog with the immutable shim extension deterministically, reject duplicate slugs, and atomically publish catalog plus registry. |
| route registry | `codex-rs/core/src/config/mod.rs` plus a new `composed_model_routes.rs` | Deserialize the private shim-extension manifest and expose the active manager-published lookup from selected slug to `RouteOwner`. It owns no credentials. |
| turn-provider resolver | `codex-rs/core/src/session/*` and client construction path | Snapshot the resolved `ModelProviderInfo` into a new turn/session configuration: native provider for subscription models, loopback `codex_native_shim` provider for shim models. |
| picker/session settings | `codex-rs/tui/src/app/thread_settings.rs` and session state | Resolve selected slug before next turn and preserve current turn's provider snapshot. |
| native shim runtime | `codex-rs/native-shim/src/{settings,relay}.rs` | Remains API-key-only; exposes readiness/diagnostics and receives no native subscription binding. |

The registry stores only a native-provider selector or a shim route slug plus
the loopback provider id. API keys stay only in `NativeSettings` inside the
parent/runtime process; OAuth remains under the native `AuthManager`.

## Data Model

```rust
enum RouteOwner {
    Native { provider_id: String },
    Shim { shim_route_slug: String, provider_id: String },
}

struct ComposedRouteBinding {
    model_slug: String,
    owner: RouteOwner,
}

struct ComposedRouteRegistry {
    bindings: BTreeMap<String, ComposedRouteBinding>,
}
```

- `model_slug` is globally unique in composed mode.
- `provider_id` is a non-secret provider identifier. For `Shim`, it is the
  loopback provider id; configured upstream/provider-entry IDs stay within the
  shim runtime for admission and diagnostics.
- The child override adds a private absolute `composed_shim_extension_json`
  path. The schema is internal and never written to `~/.codex/config.toml`.
- `ComposedModelsManager` publishes each `ModelsResponse` and its registry as
  one immutable snapshot. A native refresh may replace that snapshot only after
  collision validation succeeds; a failed refresh preserves the last valid
  snapshot and reports a credential-safe diagnostic.
- `ModelsResponse` remains compatible with the existing picker parser. Route
  ownership is intentionally parallel/private because its public model shape
  does not safely express per-model request transport.
- A turn holds a provider snapshot, not a mutable pointer to the selected
  model's route binding.

## Test Strategy

| Scenario ID | Test File | Type |
|-------------|-----------|------|
| composed-catalog-union | `codex-rs/models-manager/src/composed_catalog_tests.rs` | unit |
| composed-catalog-collision | `codex-rs/models-manager/src/composed_catalog_tests.rs` | unit |
| composed-catalog-refresh-is-atomic | `codex-rs/models-manager/src/composed_catalog_tests.rs` | unit |
| registry-validates-owners | `codex-rs/core/src/composed_model_routes_tests.rs` | unit |
| native-route-never-hits-shim | `codex-rs/core/tests/suite/composed_model_routing.rs` | integration |
| shim-route-never-uses-native-oauth | `codex-rs/core/tests/suite/composed_model_routing.rs` | integration |
| turn-snapshot-survives-switch | `codex-rs/core/tests/suite/composed_model_routing.rs` | integration |
| eager-runtime-ready-and-cleanup | `codex-rs/cli/tests/shim.rs` | integration |
| composed-child-catalog-and-overrides | `codex-rs/cli/tests/shim.rs` | integration |
| admission-is-shim-only | `codex-rs/native-shim/src/relay_tests.rs` | integration |
| direct-and-shim-only-regression | `codex-rs/cli/tests/shim.rs` and existing core model-manager tests | regression |

Tests use separate mock native and loopback/upstream endpoints. Negative
assertions inspect recorded requests and diagnostics, not merely returned
success, so the OAuth/shim boundary is proven rather than inferred.

## Dependencies

No new runtime dependency is required. Reuse `serde`, `tempfile`, existing
model-manager types, native auth manager, Tokio lifecycle primitives, and the
existing mock-server test dependencies. Cargo and Bazel metadata change only
if the new source module requires a presently unlinked workspace crate.

## Migration

No automatic migration and no config-file mutation.

1. Existing direct `codex` remains native subscription/direct behavior.
2. Existing `codex shim` remains API-key-only and keeps its default settings
   path and current-executable behavior.
3. Operators opt in with `codex shim --compose --settings PATH`.
4. Invalid settings or a cross-source collision stop before child launch;
   removing `--compose` is the immediate rollback.
5. After the composed mode is verified, a later separately approved change may
   decide whether a `codex-coda` launcher exposes it by default.
