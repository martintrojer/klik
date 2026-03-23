# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build                    # debug build
cargo build --release          # release build
cargo test                     # run all tests
cargo test thok::tests::test_write  # run a single test
```

```bash
cargo run                      # run with defaults
cargo run -- -w 25 -s 60       # 25 words, 60 second time limit
RUST_LOG=debug cargo run        # debug logging
```

Coverage: `cargo tarpaulin --out Html`. See [TESTING.md](TESTING.md) for test structure and coverage status.

## Before Committing

Always run these before committing:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## Architecture

klik is a typing speed TUI: it shows a text prompt, measures typing speed (WPM) and accuracy in real-time, and displays results with a chart.

```
CLI Args -> App::new() -> Thok::new() -> Event Loop -> UI Rendering
                                            |
                                     Input Processing -> Results (WPM, accuracy, std dev)
                                                              |
                                                     CSV log + SQLite stats
```

**Key design decisions:**

- **WPM calculation**: Groups correct characters by second intervals, applies `(chars_per_second * 60) / 5` (5-char word standard). Only correct characters count. Located in `thok.rs::calc_results()`.
- **Event loop**: Uses `crossterm` events with a 100ms tick rate for timed sessions. Events are `ThokEvent::Key`, `ThokEvent::Resize`, `ThokEvent::Tick`.
- **UI rendering**: `Thok` implements ratatui's `Widget` trait directly. Two states: typing in progress (colored prompt with cursor) and finished (WPM chart + statistics). Colors: green=correct, red=incorrect (shows expected char), underlined=current, dim=remaining.
- **Language files**: JSON in `src/lang/*.json` with `{"name", "size", "words"}`. Loaded once at startup via `include_str!`.
- **Character stats**: Per-character typing performance tracked in SQLite (`~/.local/state/klik/stats.db`). Individual keystrokes buffered in memory during a session, aggregated into `char_session_stats` rows on flush. Stores total/correct attempts, timing (sum/min/max), and uppercase-specific metrics per character per session. The `StatsStore` trait abstracts persistence (`StatsDb` for SQLite, `InMemoryStatsStore` for tests, `NoopStatsStore` for no-op).
- **Results storage**: CSV append log at `~/.config/thokr/log.csv` for session summaries.
- **Database compaction**: Automatic after each session. Triggers when >1000 sessions or >10MB. Merges records older than 30 days by character, preserving statistical accuracy. Runs VACUUM to reclaim space.
- **Session lifecycle**: Result screen offers retry (same prompt), new (fresh prompt), stats view, tweet, or escape. Settings toggleable from results screen (word count, language, random, caps, strict, symbols, substitute) and persisted via `ConfigStore`.

**Module responsibilities:**

| Module | Role |
|---|---|
| `main.rs` | CLI (clap derive), `App`/`RuntimeSettings` structs, event loop, terminal setup/teardown |
| `thok.rs` | `Thok` struct: typing state, input processing, WPM/accuracy calculation, CSV persistence |
| `typing_policy.rs` | `write_normal`/`write_strict`: input handling strategies, char stat recording |
| `session.rs` | `SessionConfig` and `SessionState` data types |
| `stats.rs` | `StatsDb`/`StatsStore` trait: SQLite character stats, aggregation, compaction, difficulty queries |
| `ui.rs` | `Widget` impl for `App`: prompt rendering, results screen with chart |
| `ui/screen.rs` | `Screen` trait: `TypingScreen`, `ResultsScreen`, `CharacterStatsScreen` with key handling |
| `ui/character_stats.rs` | Character stats table rendering with sorting and scrolling |
| `ui/charting.rs` | Chart parameter computation and label formatting |
| `language/` | `Language`, `TextFormatter` trait (Basic/Capitalization/Symbol/Combined), `WordSelector` trait (Random/Intelligent/Substitution), sentence generation |
| `word_generator.rs` | `WordGenerator`: orchestrates word selection + formatting based on config flags |
| `config.rs` | `Config`/`ConfigStore` trait: JSON config persistence |
| `runtime.rs` | `ThokEventSource`/`Ticker` traits, `Runner`: event loop abstraction (testable) |
| `celebration.rs` | Particle animation for perfect accuracy sessions |
| `util.rs` | `mean()` and `std_dev()` math helpers |
| `app_dirs.rs` | Platform-specific directory resolution |
| `time_series.rs` | `TimeSeriesPoint` for WPM chart data |

## Adding a New Language

1. Create `src/lang/newlang.json` with `{"name", "size", "words"}` structure
2. Add enum variant to `SupportedLanguage` in `main.rs`
3. Update `as_lang()` method for file name mapping
4. Add tests in `lang/mod.rs`
