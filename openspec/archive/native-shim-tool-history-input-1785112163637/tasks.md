# Implementation Plan: native-shim-tool-history-input

## Preparation

- [x] Review spec scenarios for native-shim-tool-history-input
- [x] Review design.md test strategy

## Tasks

### Task 1: Translate function-call result history

**Files:**
- Modify: `codex-rs/native-shim/src/translation.rs`
- Test: `codex-rs/native-shim/src/translation_tests.rs`

- [x] **Step 1: Write the failing test**
  Add a test with `function_call`, `function_call_output`, and trailing user
  input. It must expect the full translated Chat Completions message list.

  Run: `cd codex-rs && just test -p codex-native-shim`
  Expected: FAIL

- [x] **Step 2: Write minimal implementation**
  Add the narrow `function_call_output` conversion with call id and text
  output. Do not add native tool execution support.

  Run: `cd codex-rs && just test -p codex-native-shim`
  Expected: PASS

- [x] **Step 3: Run LoomKit verification**
  Run: `loomkit verify native-shim-tool-history-input`
  Expected: PASS at 100% traceability coverage.

## Verification

- [x] All scenarios passing (coverage = 100%)
- [x] `.traceability.yaml` updated
