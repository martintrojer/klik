# klik
✨ sleek typing tui with intelligent practice targeting and detailed analytics

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/martintrojer/klik/actions)
[![Release](https://img.shields.io/github/v/release/martintrojer/klik)](https://github.com/martintrojer/klik/releases)
[![codecov](https://codecov.io/gh/martintrojer/klik/branch/main/graph/badge.svg)](https://codecov.io/gh/martintrojer/klik)
[![License](https://img.shields.io/badge/License-MIT-default.svg)](./LICENSE.md)
[![Crate Version](https://img.shields.io/crates/v/klik)](https://crates.io/crates/klik)
[![Github Stars](https://img.shields.io/github/stars/martintrojer/klik)](https://github.com/martintrojer/klik/stargazers)

![demo](https://github.com/martintrojer/assets/raw/main/klik/demo.gif)

> **Note**: klik is a fork of [thokr](https://github.com/jrnxf/thokr) with significant enhancements including intelligent word selection, character-level analytics, session delta tracking, and comprehensive performance insights.

## Features

- **🧠 Intelligent Word Selection**: Adapts to your typing weaknesses by analyzing character-level performance
- **📊 Detailed Analytics**: Character statistics with session deltas and historical comparison
- **🔀 Character Substitution**: "Almost English" words with strategic character replacement for targeted practice
- **🎛️ Flexible Modes**: Support for capitalization, symbols, strict mode, and custom prompts
- **💾 Smart Storage**: SQLite database with automatic compaction and CSV logging
- **🎨 Rich TUI**: Real-time WPM tracking, accuracy display, and celebration animations

## Quick Start

### Installation

```sh
# Via Cargo
cargo install klik
```

### Basic Usage

```sh
# Intelligent word selection (default)
klik

# 50 words with capitalization and symbols
klik -w 50 --capitalize --symbols

# Character substitution mode for intensive practice
klik --substitute

# Strict mode (stop on errors)
klik --strict

# Custom prompt
klik -p "your custom text here"

# View detailed help
klik -h
```

## Usage Examples

| Command | Description |
|---------|-------------|
| `klik` | 15 intelligently selected words targeting your weakest letters |
| `klik -w 100` | 100 intelligent words from common English vocabulary |
| `klik --substitute` | "Almost English" words with weak characters substituted in |
| `klik --capitalize --symbols` | Words with capitalization, punctuation, and special characters |
| `klik --strict` | Stop on errors and require correction before proceeding |
| `klik -w 10 -s 60` | 10 words with 60-second time limit |

### Navigation

- **During typing**: `←` to restart, `→` for new prompt, `Esc` to quit
- **Results screen**: `s` for character statistics, `r` to retry, `n` for new test, `t` to tweet

## Intelligent Features

### Adaptive Word Selection
- Analyzes your character-level miss rate and timing
- Scores words based on difficulty of their constituent characters
- Prioritizes practice words containing your most problematic letters
- Balances difficulty to avoid repetitive content

### Character Analytics
- **Real-time tracking**: Miss rate and average time per character
- **Session deltas**: Compare current session performance vs historical data
- **Visual indicators**: Color-coded performance levels (green/yellow/red)
- **Historical trends**: SQLite database stores detailed performance metrics

### Flexible Practice Modes
- **Substitution mode** (`--substitute`): Strategic character replacement in real words
- **Realistic practice** (`--capitalize`): Capitalization, punctuation, and commas
- **Symbol training** (`--symbols`): Brackets, operators, and special characters
- **Strict mode** (`--strict`): Must correct errors before proceeding
- **Custom prompts** (`-p`): Practice with your own text

All flags work independently and can be combined for customized practice sessions.

## Supported Languages

| Language | Description |
|----------|-------------|
| `english` | 200 most common English words |
| `english1k` | 1000 most common English words |
| `english10k` | 10000 most common English words |

Use with `-l` flag: `klik -l english1k`

## Data Storage

klik automatically tracks your performance:

- **CSV Logging**: Session summaries in `~/.config/klik/log.csv`
- **Character Database**: Detailed statistics in `~/.local/state/klik/stats.db` (Linux/macOS)
- **Automatic Compaction**: Database optimization for long-term usage

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE.md](./LICENSE.md) file for details.

## Acknowledgments

- Original [thokr](https://github.com/jrnxf/thokr) project by jrnxf
- [monkeytype](https://github.com/Miodec/monkeytype) for typing test inspiration
- [ratatui](https://github.com/ratatui-org/ratatui) for the excellent TUI framework