# thokr
✨ sleek typing tui with intelligent practice targeting and detailed analytics

[![GitHub Build Workflow](https://github.com/thatvegandev/thokr/actions/workflows/build.yml/badge.svg)](https://github.com/thatvegandev/thokr/actions/workflows/build.yml)
[![GitHub Deploy Workflow](https://github.com/thatvegandev/thokr/actions/workflows/deploy.yml/badge.svg)](https://github.com/thatvegandev/thokr/actions/workflows/deploy.yml)
[![License](https://img.shields.io/badge/License-MIT-default.svg)](./LICENSE.md)
[![Crate Version](https://img.shields.io/crates/v/thokr)](https://crates.io/crates/thokr)
[![Github Stars](https://img.shields.io/github/stars/thatvegandev/thokr)](https://github.com/thatvegandev/thokr/stargazers)

![demo](https://github.com/thatvegandev/assets/raw/main/thokr/demo.gif)

## 🧠 Intelligent Word Selection

**thokr now features smart word selection that adapts to your typing weaknesses!**

By default, thokr analyzes your character-level performance and intelligently selects practice words containing the letters you struggle with most. This targeted approach helps you improve faster by focusing on your specific weak points.

### How it works:
- **Character Analysis**: Tracks your miss rate and timing for each character
- **Smart Scoring**: Words are scored based on difficulty of their characters  
- **Adaptive Selection**: Prioritizes words with your most problematic letters
- **Balanced Practice**: Selects from top 30% of difficult words to avoid repetition

### Controls:
- **Default**: Intelligent selection targets your weakest characters
- **Legacy mode**: Use `--random-words` flag for traditional random selection
- **Realistic practice**: Use `--capitalize` flag for capitalization, punctuation, and commas

## Usage

For detailed usage run `thokr -h`.

```
A sleek typing TUI with intelligent word selection that adapts to your weaknesses, detailed performance analytics, and historical progress tracking.

Usage: thokr [OPTIONS]

Options:
  -w, --number-of-words <NUMBER_OF_WORDS>
          number of words to use in test
          
          [default: 15]

  -f, --full-sentences <NUMBER_OF_SENTENCES>
          number of sentences to use in test

  -s, --number-of-secs <NUMBER_OF_SECS>
          number of seconds to run test

  -p, --prompt <PROMPT>
          custom prompt to use

  -l, --supported-language <SUPPORTED_LANGUAGE>
          language to pull words from
          
          [default: english]
          [possible values: english, english1k, english10k]

      --random-words
          use random word selection instead of intelligent character-based selection (default: intelligent selection that targets your weakest characters)

      --capitalize
          enable capitalization, punctuation, and commas for realistic typing practice

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```


## Installation

### Cargo

```sh
$ cargo install thokr
```

### Docker

```sh
$ docker run -it thatvegandev/thokr
```

### Arch Linux

Install `thokr-git` from the AUR

## Usage

For detailed usage run `thokr -h`.

### Examples

| command                        |                                                    test contents |
|:-------------------------------|-----------------------------------------------------------------:|
| `thokr`                        |   15 intelligently selected words targeting your weakest letters |
| `thokr -w 100`                 |  100 intelligently selected words from common English vocabulary |
| `thokr -w 100 -l english1k`    |  100 intelligent words from the 1000 most common English words |
| `thokr --capitalize`           |  15 intelligent words with capitalization, punctuation, and commas |
| `thokr --random-words`         |                  15 randomly selected words (legacy behavior) |
| `thokr -w 10 -s 5`             |  10 intelligent words with hard stop at 5 seconds |
| `thokr -p "$(cat foo.txt)"`    |                   custom prompt with the output of `cat foo.txt` |
| `thokr -f 4`                   | 4 grammatical sentences with full stops; overrides word settings |

_During a test you can press ← to start over or → to see a new prompt (assuming
you didn't supply a custom one)_

## Supported Languages

The following languages are available by default:

| name         |                     description |
| :----------- | ------------------------------: |
| `english`    |   200 most common English words |
| `english1k`  |  1000 most common English words |
| `english10k` | 10000 most common English words |

## 📊 Performance Analytics

thokr provides detailed character-level analytics to help you understand and improve your typing:

### Character Statistics View
- **Miss Rate**: Percentage of incorrect attempts per character
- **Average Time**: Time taken to type each character
- **Attempt Count**: Total practice attempts for each character
- **Color Coding**: Visual indicators for performance levels (green/yellow/red)

### Navigation & Sorting
- **Sort Options**: Character, Average Time, Miss Rate, or Attempts
- **Scroll Support**: Navigate through all tracked characters
- **Real-time Updates**: Statistics update after each typing session

### Access Analytics
Press `s` on the results screen to view detailed character statistics and identify areas for improvement.

## 💾 Data Storage

thokr automatically tracks your typing performance with two complementary systems:

### CSV Logging
Upon completion of each test, a summary row is appended to `log.csv` for long-term progress tracking.

### Character Statistics Database
Detailed character-level performance data is stored in a SQLite database (`stats.db`) to power intelligent word selection. The database uses an efficient session-based storage architecture that scales well with usage.

**Storage locations:**

| platform | value                                                            |                                        example |
| :------- | ---------------------------------------------------------------- | ---------------------------------------------: |
| Linux    | $HOME/.local/state/thokr/ or fallback to config directory       |                      /home/colby/.local/state/thokr |
| macOS    | $HOME/Library/Application Support/_project_path_                 | /Users/Colby/Library/Application Support/thokr |
| Windows  | {FOLDERID*RoamingAppData}\_project_path*\config                  |    C:\Users\Colby\AppData\Roaming\thokr\config |

## Roadmap

- [x] 🧠 **Intelligent Word Selection** *(completed)*
  - Smart word selection based on character-level performance analysis
  - Adaptive practice targeting your weakest typing areas
  - Comprehensive character statistics and analytics dashboard

- [x] 🔤 **Realistic Typing Practice** *(completed)*
  - Capitalization, punctuation, and comma integration
  - Case-sensitive difficulty tracking and scoring
  - Advanced formatting for real-world typing scenarios

- [ ] ⚡️ Performance Optimizations
  - Optimize TUI rendering for smoother experience at high tick rates
  - Implement incremental rendering using StatefulWidget patterns
  - Reduce computational overhead during active typing sessions

- [ ] 🔠 Multi-language Support  
  - Support for additional languages beyond English word sets
  - Proper handling of accented characters and special symbols
  - International keyboard layout compatibility improvements

- [ ] 📈 Enhanced Analytics
  - Progress tracking over time with trend visualization
  - Session comparison and improvement metrics
  - Advanced filtering and data export capabilities

- [ ] 🎯 Personalized Training
  - Custom difficulty curves based on individual progress
  - Targeted exercises for specific character combinations
  - Adaptive timing and word length recommendations

## Contributing

All contributions are **greatly appreciated**.

If you have a suggestion that would make thokr better, please fork the repo and
create a [pull request](https://github.com/thatvegandev/thokr/pulls). You can
also simply open an issue and select `Feature Request`

1. Fork the repo
2. Create your feature branch (`git checkout -b [your_username]/xyz`)
3. Commit your changes (`git commit -m 'add some xyz'`)
4. Rebase off main (`git fetch --all && git rebase origin/main`)
5. Push to your branch (`git push origin [your_username]/xyz`)
6. Fill out pull request template

See the [open issues](https://github.com/thatvegandev/thokr/issues) for a full
list of proposed features (and known issues).

## License

Distributed under the MIT License. See [LICENSE.md](./LICENSE.md) for more
information.

## Acknowledgments

Check out these amazing projects that inspired thokr!

- [monkeytype](https://github.com/Miodec/monkeytype)
- [tui-rs](https://github.com/fdehau/tui-rs)
- [ttyper](https://github.com/max-niederman/ttyper)

## Follow

[![github](https://img.shields.io/github/followers/thatvegandev?style=social)](https://github.com/thatvegandev)
[![twitter](https://img.shields.io/twitter/follow/thatvegandev?color=white&style=social)](https://twitter.com/thatvegandev)
[![youtube](https://img.shields.io/youtube/channel/subscribers/UCEDfokz6igeN4bX7Whq49-g?style=social)](https://youtube.com/user/thatvegandev)
