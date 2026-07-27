# Implementation Plan: native-shim-reasoning-history

## Preparation

- [x] Review spec scenarios for native-shim-reasoning-history
- [x] Review design.md test strategy

## Tasks

### Task 1: Support opaque reasoning history

**Files:**
- Modify: `codex-rs/native-shim/src/translation.rs`
- Test: `codex-rs/native-shim/src/translation_tests.rs`

- [x] **Step 1: Write the failing test**
  Add a structured Responses history test containing a `reasoning` item before supported tool and message items.

  Run: `just test -p codex-native-shim translation_tests::translates_responses_history_with_reasoning_to_chat_completions`
  Expected: FAIL

- [x] **Step 2: Write minimal implementation**
  Skip only `reasoning` items in the Responses-to-chat message iterator.

  Run: `just test -p codex-native-shim translation_tests::translates_responses_history_with_reasoning_to_chat_completions`
  Expected: PASS

- [x] **Step 3: Verify and commit**
  Run the native-shim tests, formatting, LoomKit verification, then commit only this change.

## Verification

- [x] All scenarios passing
- [x] `loomkit verify native-shim-reasoning-history` passes
