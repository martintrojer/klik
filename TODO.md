# TODO

## Code Quality

- [ ] **Deduplicate `CombinedFormatter` / `SymbolFormatter`**: `formatter.rs` has ~100 lines of identical symbol/punctuation logic in both. `CombinedFormatter` should compose the existing formatters rather than copy-paste them.
- [ ] **Replace `CharSummaryWithDeltas` tuple with struct**: 8-element tuple is unreadable at every call site. `CharRowData` in `ui/character_stats.rs` already has the right fields — promote it to the canonical type.
- [ ] **Fix `read_language_from_file` error handling**: Returns `Result` but uses `.expect()` three times internally, making the `Result` misleading. Either propagate errors or drop the `Result` wrapper.
- [ ] **Remove `pub use language as lang` alias**: `lib.rs` and `main.rs` both re-export this "compatibility" shim. Nothing external depends on it.
- [ ] **Remove `#[allow(clippy::new_without_default)]`** on `FileConfigStore::new` — a `Default` impl already exists.
- [ ] **Clean up `prepare_input` time fallback chain**: `typing_policy.rs:31-54` has nested if/else with magic `150` fallback and a redundant `started_at` check.
- [ ] **Remove `let _ = (idx, expected)` in `write_strict`**: If the values aren't needed, don't destructure them from `prepare_input`.

## Architecture

- [ ] Consider a distinct `Session` type owning `SessionConfig/SessionState` to reduce `Thok` responsibilities.
- [ ] Batch DB writes: buffer stats and flush less frequently (currently per char plus flush at end).

## Tests

- [ ] **Fix trivially-true UI test assertions**: Most UI tests in `ui.rs` use `|| !rendered.trim().is_empty()` fallbacks that make them pass for any non-empty buffer. Remove fallbacks, assert on specific expected content.
- [ ] **Consolidate duplicate smoke tests**: 6+ tests that only assert `buffer.area() == area` (doesn't-panic). Merge into one parameterized test.
- [ ] **Remove tautological constant tests**: `test_ui_constants` asserts a constant equals its literal value — catches nothing.
- [ ] **Fix `test_flag_independence_symbols_only`**: Sets `_found_symbols` but never asserts on it. Test cannot fail for the feature it claims to test.
- [ ] **Un-ignore integration test**: `integration_min_session` is `#[ignore]` and doesn't run in CI. The only end-to-end test has zero automated coverage.
- [ ] Move remaining long integration-style tests from `src/thok.rs` into `tests/`.
- [ ] Prefer headless tests over PTY where possible (use `ThokEventSource`/`Ticker` test impls).
