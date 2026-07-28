# Implementation Plan: compose-native-gpt-provider

## Preparation

- [x] Review spec scenarios for compose-native-gpt-provider
- [x] Review design.md test strategy

## Tasks

### Task 1: Start composed GPT turns on native OpenAI

**Files:**
- Modify: `codex-rs/cli/src/main.rs`
- Test: `codex-rs/cli/tests/shim.rs`

- [x] **Step 1: Add a stale global shim provider to the existing native compose integration test.**

  Run: `just test -p codex-cli compose_auth_boundary_forbids_native_model_from_contacting_shim_upstream`
  Expected: FAIL

- [x] **Step 2: Add the compose-only native provider child override.**

  Run: `just test -p codex-cli compose_auth_boundary_forbids_native_model_from_contacting_shim_upstream`
  Expected: PASS

- [x] **Step 3: Format and verify** with focused tests, `just fmt`, and LoomKit.

## Verification

- [x] All scenarios passing (coverage = 100%)
- [x] `.traceability.yaml` updated
