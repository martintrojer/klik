# TODO

## Simplify

- [ ] **Consolidate stats query surface**: `StatsDb` has 6 query methods that are slight variations (with/without datetime, historical/current/latest). Reduce to 2-3 with an options parameter.
- [ ] **Replace `CharSummaryWithDateTime` tuple**: Same treatment as `CharSummaryWithDeltas` — convert to a named struct or merge into `CharSummaryWithDeltas`.
- [ ] **Clean up `typing_policy.rs` field access**: Currently reaches deep into `thok.session.state` with long chains. Consider passing `&mut Session` directly instead of `&mut Thok`.
- [ ] **Simplify language abstractions**: The formatter/selector trait hierarchy is heavy for what it does. Consider collapsing `BasicFormatter`/`CapitalizationFormatter`/`SymbolFormatter`/`CombinedFormatter` into a single function with flags.
- [ ] **Unify config structs**: `RuntimeSettings`, `Cli`, `Config`, `WordGenConfig` have overlapping fields. Reduce the number of intermediate representations.

## Fix

- [ ] **Share stats DB connection with word generator**: `word_generator.rs` opens a new SQLite connection on every prompt generation to read character difficulties, instead of reusing the one `Thok` already owns.
- [ ] **Trim slow integration tests**: `integration_training_sessions.rs` is 700+ lines with many `thread::sleep` calls. Consolidate or use headless `Runner` to avoid real timing.
