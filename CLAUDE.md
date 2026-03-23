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

Coverage analysis: `./scripts/coverage.sh` or `./scripts/quick-coverage.sh`. See [TESTING.md](TESTING.md) for detailed coverage status and test architecture.

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
- **Results storage**: CSV append log at `~/.config/thokr/log.csv`. SQLite database at `~/.config/thokr/thokr_stats.db` for per-character statistics.
- **Database compaction**: Automatic after each session. Triggers when >1000 sessions or >10MB. Merges records older than 30 days by character, preserving statistical accuracy. Runs VACUUM to reclaim space.
- **Session lifecycle**: Result screen offers retry (same prompt), new (fresh prompt), tweet, or escape. Arrow keys also navigate: left=retry, right=new.

**Module responsibilities:**

| Module | Role |
|---|---|
| `main.rs` | CLI (clap derive), `App` struct, event loop, terminal setup/teardown |
| `thok.rs` | `Thok` struct: typing state, input processing, WPM/accuracy calculation, CSV/SQLite persistence, database compaction |
| `ui.rs` | `Widget` impl for `Thok`: prompt rendering, results screen with chart, layout/centering logic |
| `util.rs` | `mean()` and `std_dev()` math helpers |
| `lang/mod.rs` | `Language` struct: word list loading, random word/sentence generation |

## Adding a New Language

1. Create `src/lang/newlang.json` with `{"name", "size", "words"}` structure
2. Add enum variant to `SupportedLanguage` in `main.rs`
3. Update `as_lang()` method for file name mapping
4. Add tests in `lang/mod.rs`
