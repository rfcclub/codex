## Architecture

Use the existing `Client::prepare_response_items_for_request` normalization point, which is shared by HTTP, compaction, and websocket Responses requests. It already omits non-prefixed legacy IDs without mutating stored history. Extend its predicate to omit IDs longer than 64 characters.

## Components

`Client::prepare_response_items_for_request`: preserve only short prefixed IDs.

## Data Model

No schema or public API changes. Add a private maximum-length constant next to the request normalization logic.

## Test Strategy

| Scenario ID | Test File | Type |
|-------------|-----------|------|
| RID-1 | `codex-rs/core/tests/suite/client_websockets.rs` | integration |

## Dependencies

None.

## Migration

None. Existing local history remains intact; only invalid outbound IDs are omitted.
