## Architecture

The local merge remains scoped to `AppServerSession`. It will build a set of
remote slugs, append only local slugs not already present, and leave remote
metadata unchanged. The local configuration override path is untouched.

## Components

`merge_remote_model_presets` in `tui/src/app_server_session.rs`.

## Data Model

No type or wire-format changes.

## Test Strategy

| Scenario ID | Test File | Type |
|-------------|-----------|------|
| duplicate-slug | `tui/src/app_server_session.rs` | unit |

## Dependencies

No dependency changes.

## Migration

No migration. The previous bootstrap error becomes a single picker row.
