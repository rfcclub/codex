# Self-Check

## Claims
- `codex-native-shim` now translates a Responses `function_call` and its
  `function_call_output` into ordered text messages for Chat Completions
  providers such as Xiaomi Mimo.
- Unknown structured Responses item types still return
  `UnsupportedResponsesInput` instead of dropping context silently.

## Evidence
```json
[
  {
    "type": "command",
    "command": "cd codex-rs && just test -p codex-native-shim",
    "exit_code": 0,
    "output": "51 passed, 0 failed"
  },
  {
    "type": "command",
    "command": "loomkit verify native-shim-tool-history-input",
    "exit_code": 0,
    "output": "Coverage: 100% (2/2), PASS"
  },
  {
    "type": "file",
    "file": "codex-rs/native-shim/src/translation_tests.rs",
    "content": "Mixed function_call, function_call_output, and user history is asserted as a complete Chat Completions message list."
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
- The text-only adapter does not execute native tool calls or translate a new
  upstream tool call back to Responses tool-call output.
- Image, computer, and other non-text Responses items remain unsupported.

## Unresolved Assumptions
- Xiaomi accepts ordinary assistant and user text messages through its
  configured Chat Completions endpoint; the live smoke remains after the
  patched binary is rebuilt and installed.
