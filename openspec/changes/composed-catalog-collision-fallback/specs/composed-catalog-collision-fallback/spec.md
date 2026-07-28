## ADDED Requirements

### Requirement: Shim catalog wins a cross-source slug collision

The composed catalog SHALL replace a remote model entry with the local shim
entry when both have the same slug. It SHALL preserve remote entries whose
slugs are not owned by the shim.

#### Scenario: Remote GLM collides with local shim GLM
- **WHEN** the remote catalog contains `crof/glm-5.2` and `gpt-5.6`, and the
  shim catalog contains `crof/glm-5.2`
- **THEN** composition succeeds and the catalog contains exactly one
  `crof/glm-5.2` entry using shim metadata and one `gpt-5.6` entry.
