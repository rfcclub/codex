# Implementation Plan: native-shim-all-provider-history

## Preparation

- [x] Review spec scenarios for native-shim-all-provider-history
- [x] Review design.md test strategy

## Tasks

### Task 1: Normalize Anthropic Responses history

**Files:**
- Modify: `codex-rs/native-shim/src/translation.rs`
- Test: `codex-rs/native-shim/src/translation_tests.rs`

- [x] **Step 1: Write the failing test**
  Add a failing structured tool-history test for `responses_to_anthropic_message`.

  Run: `cd codex-rs && just test -p codex-native-shim translates_structured_responses_tool_history_to_anthropic_messages`
  Expected: FAIL

- [x] **Step 2: Write minimal implementation**
  Reuse the normalizer and split system text from non-system Anthropic messages.

  Run: `cd codex-rs && just test -p codex-native-shim`
  Expected: PASS

- [x] **Step 3: Verify and commit**
  Run: `loomkit verify native-shim-all-provider-history`, then commit only the
  native-shim code/tests and this change's archived artifacts.

## Verification

- [x] All scenarios passing (coverage = 100%)
- [x] `.traceability.yaml` updated
