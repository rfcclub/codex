## Architecture

`ComposedModelsManager` will derive the shim slug set, remove remote entries
with those slugs, then append shim models and sort by slug. The shim wins
because `composed_shim_model_slugs` already routes that slug to the shim
provider; a remote entry would be metadata inconsistent with the actual route.

## Components

`codex-rs/models-manager/src/composed_catalog.rs`: merge logic only.

## Data Model

No data-model or wire-format changes.

## Test Strategy

| Scenario ID | Test File | Type |
|-------------|-----------|------|
| remote-shim-collision | `codex-rs/models-manager/src/composed_catalog_tests.rs` | unit |

## Dependencies

No dependency changes.

## Migration

No migration. Existing startup failure becomes a deterministic catalog merge.
