## ADDED Requirements

### Requirement: Responses requests omit overlong item IDs
The client SHALL omit any response-item ID longer than 64 characters from an outbound Responses request without mutating the prompt history.

#### Scenario: Shim-originated prefixed ID exceeds the provider maximum
- **WHEN** a prompt contains a prefixed response-item ID with 65 characters
- **THEN** the serialized Responses request omits that item's `id`, and the original prompt still contains the long ID

#### Scenario: Valid Responses ID is retained
- **WHEN** a prompt contains a prefixed response-item ID with at most 64 characters
- **THEN** the serialized Responses request preserves that `id`
