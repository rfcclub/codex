# Intent: compose-native-gpt-provider

## Raw Request

"neu la gpt model thi dung cho qua shim, skip shim di"

## Problem

A global legacy shim provider can remain selected when a composed Coda session starts, sending GPT turns through the shim instead of the native OpenAI provider.

## Desired Outcome

GPT turns in composed mode start on the native OpenAI provider; only a selected custom shim model switches to the loopback provider for that turn.

## Users / Actors

- Thoor
- Composed Coda CLI and native OpenAI provider

## Current Context

`run_shim_command` passes only the composed extension and loopback provider configuration to its child. It leaves an existing global `model_provider` choice intact.

## Proposed Direction

Pass an explicit `model_provider = "openai"` child override in compose mode. The existing per-turn custom model selection then replaces it with `codex_native_shim` only for configured custom slugs.

## Scope

- Compose launcher provider override and a CLI integration regression test.

## Non-Goals

- Legacy Python shim's single-provider interactive mode; it cannot switch providers inside one process.

## Constraints

- Preserve the credential-free native shim extension and custom per-turn routing.

## Success Criteria

- A composed CLI launched with a stale global shim provider sends a GPT request to the native provider, not the shim upstream.

## Risks

- Custom routes could stop reaching the loopback runtime.
  - Mitigation: retain the existing `build_per_turn_config` custom-slug override and test its existing selected-shim path.

## Ambiguities

### Blocking

- None.

### Non-Blocking

- The OpenAI provider is available for a GPT model, as it is for the subscription-backed Coda session.

## Assumptions

- `openai` remains the native provider ID; this is the established built-in provider name.

## Spec Seeds

These are candidate requirements for the spec phase. Not binding. Derived from Desired Outcome, Constraints, Risks, and Success Criteria.

- Compose mode overrides stale global shim provider selection.

## Intent Approval

Status: IMPLEMENTING

Approved by:
Date:
