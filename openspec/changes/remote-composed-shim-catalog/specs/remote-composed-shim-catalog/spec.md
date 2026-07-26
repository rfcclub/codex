# Remote Composed Shim Catalog

### Requirement: Remote picker includes local composed models

The TUI SHALL merge its validated local composed-shim catalog into the remote
app-server's `model/list` result before constructing the model picker.

#### Scenario: composed models are shown remotely

- GIVEN a remote app-server returns one native model and the local config has
  a composed shim catalog with two non-conflicting models
- WHEN the remote TUI bootstraps
- THEN its available models contain the native model and both shim models
- AND each model appears exactly once.

#### Scenario: no composed catalog preserves remote behavior

- GIVEN a remote TUI has no composed shim catalog
- WHEN it bootstraps
- THEN its available models equal the remote app-server result.

### Requirement: Remote shim turns receive routing configuration

For a remote TUI with a composed shim configuration, the TUI SHALL include
only the composed manifest location and the `codex_native_shim` loopback
provider settings in its thread configuration request overrides.

#### Scenario: a selected composed model routes through the local shim

- GIVEN a remote TUI was launched by `codex shim --compose`
- WHEN it starts a thread with a composed shim model
- THEN the thread config override contains `composed_shim_extension_json`
- AND it contains the native shim provider's base URL, name, and Responses
  wire API
- AND the app-server can build a per-thread composed configuration.

#### Scenario: ordinary remote turns do not gain shim overrides

- GIVEN a remote TUI without a composed shim configuration
- WHEN it starts a thread
- THEN the thread config override contains no composed manifest or native shim
  provider keys.
