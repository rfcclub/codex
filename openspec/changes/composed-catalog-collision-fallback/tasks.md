# Implementation Plan: composed-catalog-collision-fallback

## Preparation

- [x] Review spec scenarios for composed-catalog-collision-fallback
- [x] Review design.md test strategy

## Tasks

### Task 1: Prefer the shim entry on a cross-source collision

**Files:**
- Modify: `codex-rs/models-manager/src/composed_catalog.rs`
- Test: `codex-rs/models-manager/src/composed_catalog_tests.rs`

- [x] **Step 1: Write the failing test** for a remote `crof/glm-5.2` and
  local shim `crof/glm-5.2` that also preserves `gpt-5.6`.

  Run: `just test -p codex-models-manager composed_catalog_tests`
  Expected: FAIL

- [x] **Step 2: Filter colliding remote models before appending shim models.**

  Run: `just test -p codex-models-manager composed_catalog_tests`
  Expected: PASS

- [x] **Step 3: Format and verify** with `just fmt`, focused tests, and LoomKit.

## Verification

- [x] All scenarios passing with 100% LoomKit traceability
