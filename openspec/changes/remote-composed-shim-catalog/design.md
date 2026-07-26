# Design

## Data Flow

The local TUI already has the child-only `Config`, including
`composed_shim_catalog`, `composed_shim_extension_json`, and the
`codex_native_shim` provider definition. The remote app-server must remain
unchanged at bootstrap because it is shared by other clients.

During remote bootstrap, `AppServerSession` receives the app-server's model
list and appends the local composed catalog. It rejects duplicate slugs rather
than silently changing a server-owned model.

When the TUI starts a thread, `config_request_overrides_from_config` adds the
manifest path and only the non-secret provider fields needed by the native
loopback shim. The existing app-server thread config loader applies those
request overrides, and existing composed-shim turn routing switches matching
slugs to `codex_native_shim`.

## Boundaries

- The merge is local to one `AppServerSession` and does not update the remote
  server's model manager.
- The remote request carries no API key: the provider points at the already
  running loopback shim, which owns upstream credentials.
- Existing models remain sourced from the app-server and retain its filtering.

## Test Strategy

| Requirement | Test location |
|---|---|
| remote picker merge | `tui/src/app_server_session.rs` unit tests |
| no-catalog behavior | `tui/src/app_server_session.rs` unit tests |
| per-thread request overrides | `tui/src/app_server_session.rs` unit tests |
| no shim override without compose | `tui/src/app_server_session.rs` unit tests |

The focused TUI tests are followed by the existing app-server model-list suite
and a live read-only probe of the current app-server after the rebuilt binary
is installed and the operator starts a new composed remote TUI.
