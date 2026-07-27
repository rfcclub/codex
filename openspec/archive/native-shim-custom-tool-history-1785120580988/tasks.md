# Implementation Plan: native-shim-custom-tool-history

## Preparation

- [x] Review spec scenarios and test strategy.

## Task 1: Normalize custom tool history

**Files:**

- Modify: `codex-rs/native-shim/src/translation.rs`
- Test: `codex-rs/native-shim/src/translation_tests.rs`

- [x] **Step 1: Write failing tests** for custom tool input and content-item output on Chat Completions and Anthropic routes.
  - Run: `just test -p codex-native-shim`
  - Result: RED — both tests returned `UnsupportedResponsesInput`.

- [x] **Step 2: Add the minimal shared normalizer support.**
  - Run: `just test -p codex-native-shim`
  - Result: GREEN — 54 passed.

- [ ] **Step 3: Commit.**
  - Run: `git commit -m "fix(shim): translate custom tool history"`

## Verification

- [ ] `loomkit verify native-shim-custom-tool-history` passes.
- [ ] Fresh release binary passes a direct shim relay probe.
