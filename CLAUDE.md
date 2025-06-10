# CLAUDE.md - Development Notes

This file contains essential notes and context for future development work on the klik codebase.

## Project Overview

**klik** is a sleek typing TUI (Terminal User Interface) application written in Rust that provides:
- Real-time typing speed (WPM) measurement
- Accuracy tracking with visual feedback
- Multiple language support (English, English1k, English10k)
- Customizable typing sessions (word count, time limits, custom prompts)
- Statistical analysis with standard deviation calculations
- Results logging and historical tracking
- Optional social sharing (Twitter integration)

## Architecture Overview

### Core Components

```
src/
├── main.rs          # CLI, App state, event handling, TUI initialization
├── thok.rs          # Core typing logic, WPM calculation, results processing
├── ui.rs            # Ratatui widgets, rendering logic, visual feedback
├── util.rs          # Mathematical utilities (mean, std_dev)
└── lang/
    └── mod.rs       # Language processing, word/sentence generation
```

### Key Dependencies

- **ratatui**: Terminal UI framework (migrated from tui-rs)
- **crossterm**: Cross-platform terminal manipulation
- **clap**: CLI argument parsing with derive macros
- **serde/serde_json**: Serialization for language files and config
- **chrono**: Date/time handling for logging
- **rand**: Random word/sentence generation
- **webbrowser**: Optional social sharing functionality

## Code Architecture Deep Dive

### Data Flow

```
CLI Args → App::new() → Thok::new() → UI Rendering
    ↓           ↓           ↓            ↓
Language → Text Generation → Input Processing → Results Calculation
    ↓           ↓           ↓            ↓
File I/O ← Results Storage ← Statistics ← Visual Feedback
```

### Key Structs and Their Responsibilities

#### `Thok` (src/thok.rs)
- **Purpose**: Core typing session management
- **Key Fields**:
  - `prompt: String` - Text to be typed
  - `input: Vec<Input>` - User's typed characters with timestamps
  - `wpm_coords: Vec<(f64, f64)>` - WPM over time for charting
  - `cursor_pos: usize` - Current typing position
  - `started_at: Option<SystemTime>` - Session start time
- **Critical Methods**:
  - `write(char)` - Process user input, calculate correctness
  - `calc_results()` - Compute final WPM, accuracy, standard deviation
  - `save_results()` - Persist session data to CSV log

#### `App` (src/main.rs)
- **Purpose**: Application state and lifecycle management
- **Key Fields**:
  - `cli: Option<Cli>` - Command-line configuration
  - `thok: Thok` - Current typing session
- **Critical Methods**:
  - `new(Cli)` - Initialize app with CLI parameters
  - `reset(Option<String>)` - Start new session with optional custom prompt

#### `Language` (src/lang/mod.rs)
- **Purpose**: Text generation and language file handling
- **Key Fields**:
  - `words: Vec<String>` - Available words for typing
  - `name: String` - Language identifier
- **Critical Methods**:
  - `get_random(usize)` - Generate random word list
  - `get_random_sentence(usize)` - Generate random sentences

### Event Handling Architecture

```rust
// Event loop pattern in main.rs
enum ThokEvent {
    Key(KeyEvent),    // User input
    Resize,           // Terminal resize
    Tick,             // Timer tick (for timed sessions)
}
```

**Key Event Handlers**:
- **Character Input**: Processed through `thok.write(char)`
- **Backspace**: Handled by `thok.backspace()`
- **Escape**: Exit current session
- **Arrow Keys**: Navigation between sessions (left=retry, right=new)
- **Result Screen**: 'r'etry, 'n'ew, 't'weet, 'esc'ape

## Critical Implementation Details

### WPM Calculation Algorithm

Located in `thok.rs::calc_results()`:

```rust
// 1. Group correct characters by second intervals
// 2. Calculate cumulative character count over time
// 3. Apply formula: (chars_per_second * 60) / 5 = WPM
// 4. Generate coordinate pairs for real-time charting
```

**Important Notes**:
- Uses 5-character word standard (industry convention)
- Handles sub-second timing precision
- Accounts for typing bursts and pauses
- Filters out incorrect characters from WPM calculation

### Accuracy Calculation

```rust
let accuracy = ((correct_chars.len() as f64 / total_input.len() as f64) * 100.0).round();
```

### Standard Deviation for Consistency Measurement

- Calculated on characters-per-second intervals
- Used to measure typing consistency
- Lower values indicate more consistent typing rhythm

### Language File Format

JSON structure in `src/lang/*.json`:
```json
{
  "name": "english",
  "size": 1000,
  "words": ["the", "be", "to", "of", "and", ...]
}
```

## UI Architecture (Ratatui)

### Widget Hierarchy

```
Terminal
└── Frame
    └── Thok Widget (implements Widget trait)
        ├── In Progress: Typing Interface
        │   ├── Timer display (if timed session)
        │   ├── Prompt with colored characters
        │   └── Cursor indication
        └── Finished: Results Screen
            ├── WPM Chart (line graph)
            ├── Statistics (WPM, accuracy, std dev)
            └── Action options
```

### Color Coding System

- **Green**: Correctly typed characters
- **Red**: Incorrectly typed characters (shows expected char)
- **Underlined**: Current character to type
- **Dim**: Remaining characters to type
- **Magenta**: Chart line color

### Layout Management

- **Responsive**: Adapts to terminal size
- **Centering**: Small prompts centered, large prompts left-aligned
- **Wrapping**: Long prompts wrap across multiple lines
- **Margins**: Configurable horizontal/vertical margins

## Testing Architecture

### Coverage Status: 67.26% (228/339 lines)

**Excellent Coverage (90-100%)**:
- `util.rs`: 100% (18/18) - Mathematical functions
- `lang/mod.rs`: 100% (19/19) - Language processing
- `ui.rs`: 97.4% (76/78) - UI rendering
- `thok.rs`: 91.4% (85/93) - Core typing logic

**Infrastructure Coverage (Expected Lower)**:
- `main.rs`: 22.9% (30/131) - Terminal/event handling

### Test Categories

1. **Unit Tests**: Function-level testing in each module
2. **Edge Cases**: Boundary conditions, empty inputs, error states
3. **Integration**: Module interaction testing
4. **Property Tests**: Mathematical function validation

### Running Tests

```bash
# All tests
cargo test

# Coverage analysis
./scripts/coverage.sh

# Quick coverage check
./scripts/quick-coverage.sh
```

## Development Workflow

### Code Standards

- **Formatting**: Use `cargo fmt` (enforced by CI)
- **Linting**: Use `cargo clippy` (enforced by CI) 
- **Testing**: Maintain >90% coverage on business logic
- **Documentation**: Update CLAUDE.md for architectural changes

### Development Task Completion Checklist

**IMPORTANT**: Always complete these steps at the end of each development task:

1. **Format Code**: Run `cargo fmt` to ensure consistent formatting
2. **Lint Code**: Run `cargo clippy` and fix all warnings
3. **Test Changes**: Run `cargo test` to verify no regressions
4. **Build Check**: Run `cargo build --release` to ensure clean compilation

**Code Quality Standards:**
- All clippy warnings must be resolved before considering a task complete
- Code formatting must be consistent (automated by `cargo fmt`)
- New functionality must include comprehensive tests
- Type complexity warnings should be addressed with type aliases when needed

### Continuous Integration

**Modern GitHub Actions CI Pipeline:**
- **Multi-platform testing**: Ubuntu, Windows, macOS
- **Multi-version testing**: Stable and Beta Rust
- **Code quality**: Formatting, linting, security audit
- **Coverage reporting**: Integrated with Codecov
- **MSRV checking**: Ensures minimum Rust version compatibility
- **Release automation**: Automated binary builds and crates.io publishing

**CI Workflow Features:**
- Rust caching for faster builds
- Security audit with cargo-audit
- Minimal versions testing
- Cross-platform binary generation
- Coverage reporting with cargo-tarpaulin

### Key Development Commands

```bash
# Development build and run
cargo run

# Run with custom parameters
cargo run -- -w 25 -s 60

# Debug build with verbose output
RUST_LOG=debug cargo run

# Release build
cargo build --release
```

## Known Considerations and Gotchas

### Terminal Compatibility

- **Raw Mode**: Must be properly enabled/disabled
- **Alternate Screen**: Used to preserve terminal content
- **Cross-platform**: Handled by crossterm, but edge cases exist
- **Resize Handling**: Event-driven resize support

### Timing Precision

- **SystemTime**: Used for high-precision timing
- **Tick Rate**: 100ms intervals for timed sessions
- **Race Conditions**: Careful handling of start/stop timing

### File I/O

- **Config Directory**: Uses platform-specific directories
- **CSV Logging**: Append-only logging to `~/.config/thokr/log.csv`
- **SQLite Database**: Character statistics stored in `~/.config/thokr/thokr_stats.db`
- **Database Compaction**: Automatic compression of old data to maintain performance
- **Error Handling**: Graceful degradation if logging fails

### Memory Management

- **Input Storage**: Grows with typing session length
- **Coordinate Storage**: Used for charting, cleaned per session
- **Language Loading**: Loaded once at startup

### Database Management & Compaction

**Character Statistics Database:**
- **Storage**: SQLite database storing aggregated character-level typing performance
- **Session-based**: Each typing session creates aggregated records per character
- **Growth Pattern**: Database grows over time as more sessions are completed

**Automatic Compaction:**
- **Trigger Conditions**: 
  - More than 1000 session records, OR
  - Database size exceeds 10MB
- **Compaction Strategy**: 
  - Merges character statistics from sessions older than 30 days
  - Preserves all statistical accuracy (totals, averages, min/max times)
  - Keeps recent sessions (last 30 days) unmodified for detailed analysis
- **Process**:
  1. Groups old sessions by character
  2. Aggregates attempts, times, accuracy data
  3. Replaces multiple old records with single compacted record
  4. Runs VACUUM to reclaim disk space
  5. Updates query optimizer statistics

**Benefits:**
- **Performance**: Maintains fast query performance as database grows
- **Storage**: Reduces disk usage while preserving statistical value
- **Accuracy**: All character difficulty metrics remain mathematically correct
- **Automatic**: Runs transparently after each typing session completion

**Manual Compaction:**
```rust
// Available via Thok methods for testing/maintenance
let mut thok = /* ... */;
let success = thok.compact_database(); // Returns true if successful
let (sessions, size_bytes, size_mb) = thok.get_database_info().unwrap();
```

## Common Development Tasks

### Adding New Language

1. Create `src/lang/newlang.json` with proper structure
2. Add enum variant to `SupportedLanguage` in `main.rs`
3. Update `as_lang()` method for file name mapping
4. Add tests in `lang/mod.rs`

### Modifying WPM Calculation

**Location**: `thok.rs::calc_results()`
**Test Coverage**: Comprehensive tests in `thok::tests`
**Considerations**: 
- Maintain backward compatibility for logged data
- Update tests to match new calculation
- Consider impact on charting coordinates

### UI Layout Changes

**Location**: `ui.rs::render()`
**Testing**: Widget tests cover major scenarios
**Considerations**:
- Test with various terminal sizes
- Ensure responsive behavior
- Maintain accessibility (color blind friendly)

### Adding New CLI Options

1. Add field to `Cli` struct in `main.rs`
2. Update `App::new()` to handle new option
3. Add tests in `main.rs::tests`
4. Update help documentation

## Performance Considerations

### Bottlenecks

1. **Terminal Rendering**: Minimize unnecessary redraws
2. **Language Loading**: Large language files (english10k) load once
3. **Statistics Calculation**: O(n) complexity, acceptable for typing sessions
4. **File I/O**: Append-only logging, minimal impact

### Optimization Opportunities

- **Lazy Loading**: Language files could be loaded on demand
- **Rendering Optimization**: Reduce widget tree complexity
- **Memory Usage**: Consider streaming for very long sessions
- **Startup Time**: Profile language file loading

## Security Considerations

### Data Privacy

- **Local Storage**: All data stored locally in user config directory
- **No Network**: No data transmission except optional social sharing
- **Logging**: Contains typing statistics but not actual typed content

### Social Features

- **Twitter Integration**: Uses system default browser
- **URL Encoding**: Properly encoded to prevent injection
- **Optional**: Can be disabled by browser availability check

## Future Development Ideas

### Feature Enhancements

1. **Custom Language Support**: User-provided word lists
2. **Themes**: Color scheme customization
3. **More Statistics**: Heat maps, typing patterns, progress tracking
4. **Multiplayer**: Network-based competitive typing
5. **Import/Export**: Session data portability

### Technical Improvements

1. **Configuration File**: TOML/JSON config for user preferences
2. **Plugin System**: Extensible language/theme system
3. **Better Error Handling**: User-friendly error messages
4. **Async Architecture**: Non-blocking I/O operations
5. **WASM Support**: Browser-based version

### Code Quality

1. **Property-Based Testing**: More comprehensive mathematical testing
2. **Integration Tests**: Full workflow testing
3. **Benchmarking**: Performance regression testing
4. **Documentation**: API documentation with examples

## Debugging Tips

### Common Issues

1. **Terminal State**: Use `cargo run` instead of `./target/debug/thokr` to ensure proper cleanup
2. **Timing Issues**: Check `TICK_RATE_MS` for timed session problems
3. **Language Loading**: Verify JSON syntax in language files
4. **UI Rendering**: Use different terminal emulators to test compatibility

### Debug Commands

```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Run with backtrace
RUST_BACKTRACE=1 cargo run

# Check for memory leaks (if using valgrind)
valgrind --tool=memcheck cargo run
```

### Test-Driven Development

- Write tests first for new features
- Use property-based tests for mathematical functions
- Mock external dependencies (file I/O, system time)
- Maintain test coverage above 90% for business logic

## Migration Notes

### From tui-rs to ratatui

- **Completed**: All UI code migrated to ratatui 0.29
- **Breaking Changes**: Import paths updated, some API changes handled
- **Benefits**: Better performance, active maintenance, new features

### Dependency Updates

- **Clap**: Using clap 4.x with derive macros
- **Crossterm**: Latest version for better cross-platform support
- **Serde**: Standard serialization with derive features

## Contact and Resources

- **Repository**: https://github.com/martintrojer/klik
- **Issues**: Report bugs and feature requests on GitHub
- **Documentation**: This file and TESTING.md for comprehensive coverage
- **Dependencies**: Check Cargo.toml for current version requirements

---

**Last Updated**: June 2025
**Coverage**: 67.26% (excellent for this type of application)
**Status**: Mature codebase with comprehensive testing and documentation