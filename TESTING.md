# Testing

## Running Tests

```bash
cargo test                              # all tests
cargo test thok::tests                  # specific module
cargo test test_calc_results            # pattern match
cargo test -- --nocapture               # with output
cargo test --test integration_min_session  # PTY integration test (Unix only)
```

## Coverage

```bash
./scripts/coverage.sh                   # full coverage analysis
./scripts/quick-coverage.sh             # quick check
cargo tarpaulin --out Html              # HTML report
```

## Test Structure

```
src/
├── main.rs          # CLI parsing, app init tests
├── thok.rs          # Core typing logic, WPM calculation, results
├── ui.rs            # Widget rendering, layout, color coding
├── util.rs          # mean(), std_dev() math helpers
└── lang/
    └── mod.rs       # Language loading, word/sentence generation

tests/
└── integration_min_session.rs  # PTY-driven end-to-end test (Unix)
```

## Coverage by Module

| Module | Coverage | Notes |
|---|---|---|
| `util.rs` | 100% | Math functions, all edge cases |
| `lang/mod.rs` | 100% | Word generation, JSON parsing |
| `ui.rs` | ~97% | Rendering states, layout |
| `thok.rs` | ~91% | Input handling, WPM calc, state transitions |
| `main.rs` | ~23% | Expected low - terminal/event loop infrastructure |
