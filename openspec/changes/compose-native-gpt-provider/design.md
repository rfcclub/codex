## Architecture

Compose mode has two provider layers: native OpenAI is the default child provider, and `build_per_turn_config` selects `codex_native_shim` only for custom slugs listed in the extension. Set the first layer explicitly in the child arguments so stale machine configuration cannot override it.

## Components

`run_shim_command` child construction and the compose auth-boundary integration test.

## Data Model

No data-model or public interface changes.

## Test Strategy

| Scenario ID | Test File | Type |
|-------------|-----------|------|
| CNG-1 | `codex-rs/cli/tests/shim.rs` | integration |

## Dependencies

None.

## Migration

None.
