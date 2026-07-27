# Self-Check

## Claims
- `responses_to_anthropic_message` now normalizes supported structured Responses input with the same helper as Chat Completions.
- System/developer text is emitted as Anthropic `system`; tool calls and outputs remain ordered Anthropic messages.
- The OpenAI Responses relay preserves structured input while replacing only the model slug.

## Evidence
```json
[
  {
    "type": "command",
    "command": "cd codex-rs && just test -p codex-native-shim",
    "exit_code": 0,
    "output": "52 passed, 0 skipped"
  },
  {
    "type": "command",
    "command": "cd codex-rs && cargo fmt --check --package codex-native-shim",
    "exit_code": 0,
    "output": "rustfmt check passed"
  },
  {
    "type": "command",
    "command": "loomkit verify native-shim-all-provider-history",
    "exit_code": 0,
    "output": "Coverage: 100% (2/2)"
  }
]
```

<!-- Evidence format:
[
  { "type": "command", "command": "bun test", "exit_code": 0, "output": "12 pass 0 fail" },
  { "type": "file", "file": "src/foo.ts", "content": "relevant snippet" }
]
-->

## Known Limitations
- Unsupported structured input item types still return `UnsupportedResponsesInput`.

## Unresolved Assumptions
- A configured live Anthropic provider was not invoked; route behavior is covered by native-shim unit and relay tests.
