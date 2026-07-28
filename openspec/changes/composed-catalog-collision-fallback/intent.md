# Intent: composed-catalog-collision-fallback

## Raw Request

"Van co bug Error: composed shim model `crof/glm-5.2` collides with the remote app-server catalog. Em xem va chinh trong code, if it has collision then skip remote, go back to gpt model .. any model"

## Problem

The composed models manager rejects a local shim model when the authenticated
remote catalog advertises the same slug. That prevents the picker from
starting, including its unrelated GPT models.

## Desired Outcome

A collision produces one shim-owned picker entry for the shared slug and
retains all non-colliding remote models, including GPT fallbacks.

## Users / Actors

- Codex user running composed native-shim mode
- Remote app-server catalog and local shim extension

## Current Context

`ComposedModelsManager::merge_models` returns an error when either source
contains a duplicate slug. The selected route is already determined by the
shim slug set, so retaining remote metadata for a shim-owned slug would be
inconsistent with turn routing.

## Proposed Direction

Make the local shim authoritative for cross-source collisions: filter a
remote entry whose slug is owned by the shim, then append the shim entry.

## Scope

- Composed catalog merge and its focused regression tests

## Non-Goals

- Remote app-server catalog mutation, provider credentials, and routing changes

## Constraints

- Do not add duplicate picker entries or discard non-colliding remote models.

## Success Criteria

- A colliding `crof/glm-5.2` does not abort startup.
- The resulting catalog contains exactly one local shim `crof/glm-5.2` and a remote GPT model.

## Risks

- Remote and shim metadata may differ.
  - Mitigation: retain the shim entry because the selected slug is routed through the shim.

## Ambiguities

### Blocking

- None.

### Non-Blocking

- Treat "skip remote" as skipping the colliding remote entry, while retaining its unrelated models.

## Assumptions

- A shim-owned slug must be represented by shim metadata because all turns for
  that slug already route to the shim provider.

## Spec Seeds

These are candidate requirements for the spec phase. Not binding. Derived from Desired Outcome, Constraints, Risks, and Success Criteria.

- Cross-source collisions resolve to one shim-owned entry without removing GPT fallbacks.

## Intent Approval

Status: APPROVED

Approved by:
Date:
