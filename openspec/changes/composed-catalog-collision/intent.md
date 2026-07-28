# Intent: composed-catalog-collision

## Raw Request

"van bi loi Error: composed shim model `crof/glm-5.2` collides with the remote app-server catalog"

## Problem

Remote bootstrap rejects a composed model when the remote app-server already
advertises the same model slug. This prevents `/model` from opening even though
the remote catalog already contains a usable entry.

## Desired Outcome

A remote picker starts successfully, shows each model slug once, and retains
the server-provided entry when it duplicates a local composed shim slug.

## Users / Actors

- Codex user running a composed native shim against a remote app-server
- Local TUI bootstrap and remote app-server model catalog

## Current Context

`merge_remote_model_presets` in `codex-rs/tui/src/app_server_session.rs`
returns an error for every duplicate local composed slug.

## Proposed Direction

Treat the remote entry as authoritative and skip the duplicate local entry.

## Scope

- Remote picker merge behavior and unit regression coverage

## Non-Goals

- Changes to the remote app-server catalog, credentials, or thread routing

## Constraints

- Do not add duplicate picker rows or overwrite remote model metadata.

## Success Criteria

- A duplicate composed slug produces one remote picker entry without an error.

## Risks

- A remote duplicate might not use the local shim configuration.
  - Mitigation: preserve existing local per-thread composed config overrides.

## Ambiguities

### Blocking

- None.

### Non-Blocking

- The remote catalog entry is valid for the local TUI session.

## Assumptions

- Remote metadata is authoritative because it is filtered for that app-server
  connection and it was already selected before local composition runs.

## Spec Seeds

These are candidate requirements for the spec phase. Not binding. Derived from Desired Outcome, Constraints, Risks, and Success Criteria.

- Deduplicate by slug while preserving the remote model.

## Intent Approval

Status: APPROVED

Approved by:
Date:
