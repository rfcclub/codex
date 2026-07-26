# Implementation Plan: remote-composed-shim-catalog

## Task 1: Remote picker catalog merge

- [x] RED: add a unit test for merging two local composed models into a remote
  model list and a no-catalog control test.
- [x] GREEN: add a small slug-checked merge helper in
  `tui/src/app_server_session.rs` and use it during bootstrap.
- [x] REFACTOR: keep conversion to `ModelPreset` after the merge so both
  sources retain normal picker behavior.

## Task 2: Per-thread composed routing overrides

- [x] RED: add unit tests that verify the exact three native provider fields
  plus manifest path are present only for composed configuration.
- [x] GREEN: extend `config_request_overrides_from_config` to pass those
  non-secret values.
- [x] REFACTOR: centralize the provider key construction next to the existing
  config override helper.

## Task 3: Verify

- [x] Run focused TUI tests and formatter.
- [ ] Run the app-server model-list suite.
- [x] Build the patched binary.
- [ ] Confirm a newly started composed remote TUI returns all seven shim slugs
  from its effective picker path without exposing credentials.
