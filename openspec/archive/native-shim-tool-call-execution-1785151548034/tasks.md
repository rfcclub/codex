# Implementation Plan: native-shim-tool-call-execution

## Preparation

- [x] Review spec scenarios for native-shim-tool-call-execution.
- [x] Review design.md test strategy.

## Tasks

### Task 1: Translate Chat tool contracts

**Files:**
- Modify: `codex-rs/native-shim/src/translation.rs`
- Test: `codex-rs/native-shim/src/translation_tests.rs`

- [x] **Step 1: Write failing tests** for a Responses function/custom tool request, a non-streaming Chat tool call, and streamed tool-call deltas.
  - Run: `just test -p codex-native-shim`
  - Expected: RED.
- [x] **Step 2: Implement minimal request and response translation.**
  - Run: `just test -p codex-native-shim`
  - Expected: GREEN.
- [x] **Step 3: Add a relay regression** proving the upstream Chat request retains its tool declaration.
  - Run: `just test -p codex-native-shim`
  - Expected: GREEN.

## Verification

- [ ] `loomkit verify native-shim-tool-call-execution` passes.
- [x] `just test -p codex-native-shim` passes.
- [ ] Build and directly smoke the patched binary through the configured shim.
