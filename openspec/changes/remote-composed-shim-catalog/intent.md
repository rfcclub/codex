# Intent: remote-composed-shim-catalog

## Problem

`codex shim --compose -- --remote unix://...` passes the composed shim
manifest only to the local TUI process. The remote app-server owns
`model/list`, so it returns its own native catalog and `/model` cannot show
the configured shim models. If a shim model were selected by another means,
the app-server would also lack the per-thread composed manifest and loopback
provider settings needed to route that turn.

## Desired Outcome

When a TUI connected to a remote app-server has a composed shim configuration,
`/model` includes the configured shim entries and starting a thread with one
of them sends the minimum non-secret configuration required for that one
thread to use the local native shim.

## Non-Goals

- Do not mutate or restart the shared remote app-server.
- Do not make a local composed manifest visible to other remote clients.
- Do not transmit provider credentials or alter the native app-server catalog.
- Do not change non-composed or embedded TUI behavior.

## Success Criteria

- A remote TUI with a composed catalog shows each local shim model exactly once.
- A thread started from that TUI includes the composed manifest path and the
  native-shim loopback provider configuration in request overrides.
- A remote TUI without a composed catalog preserves the server-provided list
  and does not add shim-specific overrides.
