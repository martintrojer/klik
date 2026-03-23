# klik

Typing speed TUI with intelligent practice targeting and detailed analytics.

![demo](doc/demo.gif)

- **Adaptive** -- analyzes character-level miss rate and timing, selects practice words targeting your weakest letters
- **Analytics** -- per-character statistics with session deltas, historical comparison, color-coded performance
- **Flexible** -- capitalization, symbols, strict mode, character substitution, custom prompts, timed sessions

## Install

Pre-built binaries on the [releases page](https://github.com/martintrojer/klik/releases), or:

```bash
mise use github:martintrojer/klik  # mise
cargo install klik                 # cargo
```

## Quick start

```bash
klik                               # 15 intelligently selected words
klik -w 50 --capitalize --symbols  # 50 words with caps and symbols
klik --substitute                  # "almost English" words for intensive practice
klik --strict                      # stop on errors, require correction
klik -p "your custom text here"    # custom prompt
klik -w 10 -s 60                   # 10 words, 60-second time limit
```

## Documentation

- **[User Guide](doc/GUIDE.md)** -- practice modes, navigation, settings, data storage

## License

[MIT](./LICENSE) -- fork of [thokr](https://github.com/jrnxf/thokr) by jrnxf.
