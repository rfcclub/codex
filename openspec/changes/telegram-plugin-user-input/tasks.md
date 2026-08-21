# Implementation Plan: telegram-plugin-user-input

## Preparation

- [x] Review spec scenarios for telegram-plugin-user-input
- [x] Review design.md test strategy

## Tasks

### Task 1: Authorize standalone Telegram input

**Files:**
- Modify: `codex-rs/codex-mcp/src/connection_manager.rs`
- Test: `codex-rs/codex-mcp/src/connection_manager_tests.rs`

- [x] **Step 1: Write the failing test**
  ```rust
  assert!(accepts_user_input_notifications("telegram"));
  ```

  Run: `just test -p codex-mcp telegram_mcp_server_accepts_user_input_notifications`
  Expected: FAIL

- [x] **Step 2: Write minimal implementation**
  ```rust
  matches!(server_name, "colony-mesh" | "telegram")
  ```

  Run: `just test -p codex-mcp telegram_mcp_server_accepts_user_input_notifications`
  Expected: PASS

- [ ] **Step 3: Verify active CLI runtime and real Telegram delivery**

## Verification

- [ ] All scenarios passing (coverage = 100%)
- [ ] `.traceability.yaml` updated
