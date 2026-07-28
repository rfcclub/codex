## Why

A remote catalog can contain a model slug also configured in the local shim.
The current composition rejects that catalog and prevents any picker model,
including GPT fallbacks, from being used.

## What Changes

- Filter remote model entries whose slug is provided by the composed shim.
- Retain each shim entry and all non-colliding remote models.
- Replace collision-rejection coverage with shim-authoritative merge coverage.

## Capabilities

### New Capabilities
- `composed-catalog-collision-fallback`: Resolve cross-source collisions while preserving usable remote fallback models.

### Modified Capabilities
<!-- None. -->

## Impact

`codex-rs/models-manager/src/composed_catalog.rs` and focused unit tests. No
wire format, dependency, or public app-server API changes.
