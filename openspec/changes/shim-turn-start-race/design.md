# Design

`ActiveTurn` records the sub-id that owns a startup reservation. `start_task`
records ownership before awaiting lifecycle work, then verifies that ownership
before attaching its `RunningTask`. `abort_all_tasks` removes an idle
reservation as well as a running task so a replacement owns a fresh slot.

The regression test asserts that replacing an idle reservation leaves no active
turn; the custom-model CLI smoke validates the real startup path.
