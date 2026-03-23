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

- [x] **Fix trivially-true UI test assertions**: Replaced with specific content assertions (`contains("42")`, `contains("(r)etry")`, etc).
- [x] **Consolidate duplicate smoke tests**: Merged 6+ size/edge-case tests into `test_render_various_sizes` and `test_render_edge_case_prompts`.
- [x] **Remove tautological constant tests**: Deleted `test_ui_constants` and `test_ui_constants_consistency`.
- [x] **Fix `test_flag_independence_symbols_only`**: Now asserts `found_symbols` over 100 trials.
- [x] **Remove emoji bloat from tests**: Cleaned up println/panic in thok.rs, celebration.rs, integration tests.
- [ ] **Un-ignore integration test**: `integration_min_session` is `#[ignore]` and doesn't run in CI.
- [ ] Move remaining long integration-style tests from `src/thok.rs` into `tests/`.
- [ ] Prefer headless tests over PTY where possible (use `ThokEventSource`/`Ticker` test impls).
