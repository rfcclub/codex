# Implementation Plan: composed-catalog-collision

## Preparation

- [x] Review spec scenarios for composed-catalog-collision
- [x] Review design.md test strategy

## Tasks

### Task 1: Deduplicate composed model slugs

**Files:**
- Modify: `codex-rs/tui/src/app_server_session.rs`
- Test: `codex-rs/tui/src/app_server_session.rs`

- [x] **Step 1: Write the failing test** for a duplicate slug that asserts the
  remote entry is retained exactly once.

  Run: `just test -p codex-tui remote_picker_deduplicates_composed_model_that_matches_server_model`
  Expected: FAIL

- [x] **Step 2: Skip composed entries whose slug is already remote.**

  Run: `just test -p codex-tui remote_picker_deduplicates_composed_model_that_matches_server_model`
  Expected: PASS

- [x] **Step 3: Format and verify** with `just fmt`, focused TUI tests, and
  LoomKit.

## Verification

- [x] All scenarios passing with 100% LoomKit traceability
