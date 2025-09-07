# TODO: Improvements and Open Tasks

This document tracks pending refactors, bug fixes, and quality improvements.

## Bugs (Subtle/Edge Cases)
- [ ] Unicode prompt length mismatch: `has_finished()` uses `prompt.len()` (bytes) while input counts typed chars. Use `prompt.chars().count()` and avoid slicing strings with byte indices in UI.
- [ ] UI slicing bug: `ui.rs` slices `prompt[start..]` where `start` is a character index, not a byte index. Replace with char-iterator based rendering or grapheme-aware slicing.
- [ ] Timer underflow: clamp `seconds_remaining` at zero in `on_tick()` to avoid negative values and odd UI displays.
- [ ] Accuracy with zero input: `calc_results()` divides by `input.len()`; when zero this yields NaN. Guard and define desired behavior (e.g., 0%).
- [ ] Tweet URL encoding: results tweet link uses malformed URL encoding (`github.com%martintrojer`). Fix to `%2Fmartintrojer%2Fklik`.
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
