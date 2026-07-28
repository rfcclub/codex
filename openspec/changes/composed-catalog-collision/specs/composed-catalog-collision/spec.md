## ADDED Requirements

### Requirement: Remote catalog wins duplicate composed slugs

The TUI SHALL preserve the remote app-server model entry and skip a local
composed model with the same slug.

#### Scenario: Composed model is already remote
- **WHEN** remote `model/list` and the local composed catalog both contain
  `nahcrof/glm-5.2`
- **THEN** bootstrap succeeds and the picker contains exactly one
  `nahcrof/glm-5.2` entry, using the remote entry.
