# User Guide

## Practice modes

| Flag | Effect |
|------|--------|
| (default) | Intelligent word selection targeting your weakest characters |
| `--substitute` | "Almost English" words with weak characters substituted in |
| `--capitalize` | Capitalization and punctuation |
| `--symbols` | Brackets, operators, and special characters |
| `--strict` | Must correct errors before proceeding |
| `-p "text"` | Custom prompt |

All flags combine freely: `klik -w 50 --capitalize --symbols --strict`

## Languages

| Flag | Words |
|------|-------|
| `-l english` (default) | 200 most common |
| `-l english1k` | 1,000 most common |
| `-l english10k` | 10,000 most common |

## Navigation

**During typing:**
- `Esc` -- quit

**Results screen:**
- `r` -- retry (same prompt)
- `n` -- new prompt
- `s` -- character statistics view
- `t` -- tweet results
- `Esc` -- quit

**Results screen settings (toggle and persist):**
- `w` -- cycle word count (15/25/50/100)
- `l` -- cycle language
- `1` -- random words
- `2` -- capitalization
- `3` -- strict mode
- `4` -- symbols
- `5` -- substitution

**Character stats screen:**
- `1-4` -- sort by character/time/miss rate/attempts
- `Space` -- toggle sort direction
- `Up/Down/PgUp/PgDn/Home` -- scroll
- `b` or `Backspace` -- back to results

## Data storage

| Path | Contents |
|------|----------|
| `~/.config/klik/log.csv` | Session summaries (WPM, accuracy, std dev) |
| `~/.local/state/klik/stats.db` | Per-character typing statistics (SQLite) |
| `~/.config/klik/config.json` | Persisted settings |

The stats database compacts automatically when it exceeds 1,000 sessions or 10 MB, merging records older than 30 days.

## Adaptive word selection

klik tracks per-character miss rate and timing across sessions. When selecting practice words, it:

1. Scores each word by the difficulty of its constituent characters
2. Weights toward words containing your most problematic letters
3. Balances difficulty to avoid repetitive content

This means each session naturally focuses on the characters you need to practice most.
