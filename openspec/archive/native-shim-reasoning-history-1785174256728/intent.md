# Intent: native-shim-reasoning-history

## Raw Request

Fix the recurring `native shim only supports string Responses input for this provider adapter` failure for custom models routed through 9router, then commit the verified fix.

## Problem

The native shim translates Responses history for chat-completions providers but rejects opaque `reasoning` history items emitted by Codex. A follow-up custom-model turn therefore fails even though function and custom-tool history is supported.

## Desired Outcome

Structured Responses history containing opaque reasoning items reaches the configured provider without this local translation error, while actionable message and tool history is preserved.

## Users / Actors

- Coda users selecting a custom 9router model.
- The native shim translating Codex Responses requests for non-Responses upstreams.

## Current Context

- `codex-rs/native-shim/src/translation.rs` accepts string, message, function, and custom-tool input, but rejects unknown item types.
- Live shim logs show `reasoning` items interleaved with message and tool history.
- The custom model configuration currently declares a chat-completions protocol; this change makes that compatibility translation resilient.

## Proposed Direction

Ignore opaque reasoning-history items when producing the lossy chat-completions representation, retaining the supported message and tool items around them.

## Scope

- Add regression coverage for interleaved reasoning history.
- Make the minimal native-shim translation change.
- Build and verify the patched binary before committing.

## Non-Goals

- Change 9router credentials, model choices, or rate-limit policy.
- Change Telegram routing.
- Rework the Python shim running on port 8829.

## Constraints

- Do not expose credentials in code, tests, logs, or commits.
- Preserve existing rejection for unsupported non-history input items.
- Maintain existing tool and message translation behavior.

## Success Criteria

- A Responses request with reasoning, tool, and message history translates successfully.
- The translated chat messages omit only opaque reasoning items.
- The native-shim test target and LoomKit verification pass.

## Risks

- Ignoring a future actionable item type could drop relevant context.
  - Mitigation: ignore only the documented opaque `reasoning` type; keep rejecting other unknown types.

## Assumptions

- Reasoning items are model-internal context and do not have a portable chat-completions equivalent; omitting them is the safe lossless-boundary behavior.

## Intent Approval

Status: IMPLEMENTING
