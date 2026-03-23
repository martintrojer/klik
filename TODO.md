# TODO

## Code Quality

- [x] **Deduplicate `CombinedFormatter` / `SymbolFormatter`**: Extracted shared `format_with_symbols()` helper; both formatters now delegate to it.
- [x] **Replace `CharSummaryWithDeltas` tuple with struct**: Promoted to named struct in `stats.rs`, removed duplicate `CharRowData` from `ui/character_stats.rs`.
- [x] **Fix `read_language_from_file` error handling**: Inlined into `Language::new()`, removed misleading `Result` wrapper.
- [x] **Remove `pub use language as lang` alias**: Removed from `lib.rs` and `main.rs`.
- [x] **Remove `#[allow(clippy::new_without_default)]`** on `FileConfigStore::new`.
- [x] **Clean up `prepare_input` time fallback chain**: Extracted `calculate_time_to_press()`, introduced `PreparedInput` struct, removed redundant checks.
- [x] **Remove `let _ = (idx, expected)` in `write_strict`**: `prepare_input` now returns only needed fields via `PreparedInput`.

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
