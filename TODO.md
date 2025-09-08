# TODO: Improvements and Open Tasks

This document tracks pending refactors, bug fixes, and quality improvements.

## Bugs (Subtle/Edge Cases)
- [ ] Prepare input past end: `prepare_input()` can compare typed char vs `' '` when `idx` >= prompt length. Bail early if session finished.
- [ ] CSV logging safety: `save_results()` writes raw CSV without quoting/escaping (date contains commas); switch to a CSV writer or quote fields.
- [ ] Idle time reset math: verify `mark_activity()` time shifting logic preserves elapsed accurately across idle transitions.

## Performance/Quality
- [ ] Reduce UI allocations: building spans and to_string per char; consider preallocating and avoiding repeated string conversions.
- [ ] Batch DB writes: consider buffering stats and flushing less frequently (currently per char plus flush at end).
- [ ] Error handling: avoid silently ignoring DB errors in stats recording; surface logs in debug/test modes.
- [ ] Config persistence on setting toggles at Results screen (optional): persist updated `runtime_settings` via `ConfigStore`.

## Architecture
- [ ] Consider a distinct `Session` type (wrapper) owning `SessionConfig/SessionState` to reduce responsibilities in `Thok`.

## Tests Organization
- [ ] Move remaining long integration-style tests from `src/thok.rs` into `tests/` where feasible.

## Testing Policy
- [ ] Every new abstraction (trait/struct/module) must include targeted unit tests.
- [ ] Refactor existing tests to use the new abstraction (migrate mocks/helpers accordingly).
- [ ] Prefer headless tests over PTY where possible (use `ThokEventSource`/`Ticker` test impls).
- [ ] Keep integration coverage by adding at least one end-to-end path using the new pieces.

## Completed (Recent)
- [x] Unicode finish check uses prompt char count (not bytes).
- [x] UI prompt slicing fixed to use char iteration (Unicode-safe).
- [x] Clamp `seconds_remaining` at 0 in `on_tick()`.
- [x] Guard accuracy calc for empty input (avoid NaN).
- [x] Correct tweet URL encoding in Results screen.
