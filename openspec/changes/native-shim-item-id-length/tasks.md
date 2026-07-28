# Implementation Plan: native-shim-item-id-length

## Preparation

- [x] Review spec scenarios for native-shim-item-id-length
- [x] Review design.md test strategy

## Tasks

### Task 1: Omit provider-incompatible response item IDs

**Files:**
- Modify: `codex-rs/core/src/client.rs`
- Test: `codex-rs/core/tests/suite/client_websockets.rs`

- [x] **Step 1: Write the failing integration test** for a 65-character prefixed ID.

  Run: `just test -p codex-core responses_websocket_omits_unprefixed_item_ids_without_mutating_prompt`
  Expected: FAIL

- [x] **Step 2: Omit overlong IDs in request normalization.**

  Run: `just test -p codex-core responses_websocket_omits_unprefixed_item_ids_without_mutating_prompt`
  Expected: PASS

- [x] **Step 3: Format and verify** with the focused core test, `just fmt`, and LoomKit.

## Verification

- [x] All scenarios passing (coverage = 100%)
- [x] `.traceability.yaml` updated
