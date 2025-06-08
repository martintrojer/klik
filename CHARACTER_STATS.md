# Character-Level Statistics

This document describes the character-level statistics tracking feature in thokr, which monitors typing performance for individual characters.

## Overview

The character statistics feature tracks detailed metrics for each character you type during typing tests, including:

- **Time to press**: How long it takes from when a keypress starts until the character is registered
- **Accuracy**: Whether each character was typed correctly or incorrectly
- **Context**: The characters before and after each typed character
- **Timestamp**: When each character was typed

All data is stored in a SQLite database located at `$HOME/.local/state/thokr/stats.db`.

## Database Schema

### Character Statistics Table

```sql
CREATE TABLE character_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    character TEXT NOT NULL,              -- The expected character
    time_to_press_ms INTEGER NOT NULL,    -- Time in milliseconds to press the key
    was_correct BOOLEAN NOT NULL,         -- Whether the character was typed correctly
    timestamp TEXT NOT NULL,              -- ISO 8601 timestamp when typed
    context_before TEXT,                  -- 3 characters before this position
    context_after TEXT,                   -- 3 characters after this position
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### Indexes

- `idx_character_stats_char`: Index on character for fast character-specific queries
- `idx_character_stats_timestamp`: Index on timestamp for chronological queries

## Features

### Automatic Tracking

Character statistics are automatically collected during typing tests:

1. **Keypress Start**: When a key is pressed, the system records the start time
2. **Character Processing**: When the character is registered, the system:
   - Calculates the time difference (time to press)
   - Determines if the character was correct or incorrect
   - Extracts context (surrounding characters)
   - Records all data to the database immediately
3. **Training Run Completion**: When a typing test finishes, the system:
   - Calculates overall results (WPM, accuracy, etc.)
   - Ensures all character statistics are flushed to disk
   - Saves the session summary to the existing CSV log

### Statistics Analysis

The system provides several analysis methods:

#### Per-Character Statistics
```rust
// Get all statistics for a specific character
let stats = thok.get_char_stats('a');

// Get average time to press for a character
let avg_time = thok.get_avg_time_to_press('a'); // Returns milliseconds

// Get miss rate for a character (percentage incorrect)
let miss_rate = thok.get_miss_rate('a'); // Returns 0.0 to 100.0
```

#### Summary Statistics
```rust
// Get summary for all characters: (char, avg_time_ms, miss_rate_%, total_attempts)
let summary = thok.get_all_char_summary();
```

#### Database Operations
```rust
// Flush any pending statistics to disk
thok.flush_char_stats();

// For advanced usage - batch recording (requires mutable database reference)
let mut stats_db = StatsDb::new().unwrap();
stats_db.record_char_stats_batch(&char_stats_vector);
```

## Data Storage Location

The database is stored at:
- **Linux/macOS**: `$HOME/.local/state/thokr/stats.db`
- **Windows**: `%APPDATA%\thokr\stats.db`

The directory is automatically created if it doesn't exist.

## Privacy and Data

- All data is stored locally on your machine
- No data is transmitted over the network
- The database can be manually deleted to clear all statistics
- Character context is limited to 3 characters before and after for privacy

## Performance Considerations

- Database operations are non-blocking and won't affect typing performance
- Statistics collection has minimal overhead (< 1ms per character)
- Database uses SQLite with bundled static compilation for reliability
- Indexes ensure fast queries even with large datasets
- Real-time recording during typing with batch flush at training completion
- Graceful error handling - database failures don't interrupt typing experience

## Technical Implementation

### Character Timing

The timing mechanism works as follows:

1. **Keypress Detection**: When a key event is detected in the main event loop
2. **Start Timer**: `thok.on_keypress_start()` records the current system time
3. **Character Processing**: `thok.write(char)` processes the character and:
   - Calculates time difference from start to now
   - Creates a `CharStat` record with all relevant data
   - Stores the record in the SQLite database immediately
   - Resets the timer for the next character
4. **Training Completion**: `thok.calc_results()` is called which:
   - Processes all WPM and accuracy calculations
   - Calls `flush_char_stats()` to ensure all data is committed
   - Saves session summary to CSV log

### Error Handling

- Database initialization failures are handled gracefully (statistics simply won't be recorded)
- Database write failures don't interrupt typing tests
- Missing database connections return `None` for all statistics queries

### Context Extraction

Context extraction provides insight into character difficulty:

```rust
// Extract 3 characters before and after position 6 in "hello world"
let (before, after) = extract_context("hello world", 6, 3);
// before = "lo ", after = "orl"
```

## Example Usage

```rust
use thokr::Thok;

// Create a new typing test
let mut thok = Thok::new("hello world".to_string(), 2, None);

// Simulate typing with timing
thok.on_keypress_start();
thok.write('h');

thok.on_keypress_start();
thok.write('e');

// After typing, query statistics
if let Some(avg_time) = thok.get_avg_time_to_press('h') {
    println!("Average time to press 'h': {:.1}ms", avg_time);
}

if let Some(miss_rate) = thok.get_miss_rate('h') {
    println!("Miss rate for 'h': {:.1}%", miss_rate);
}
```

## Future Enhancements

Potential future improvements:

1. **Statistics Export**: Export data to CSV/JSON formats
2. **Visualization**: Charts showing improvement over time
3. **Recommendations**: Suggest practice for problematic characters
4. **Difficulty Analysis**: Identify character combinations that cause issues
5. **Progress Tracking**: Monitor improvement trends over time
6. **Custom Drills**: Generate practice text focusing on weak characters

## Testing

The character statistics module includes comprehensive tests:

- Database operations (create, read, update, delete)
- Character timing accuracy
- Context extraction
- Error handling
- Integration with typing logic

Run tests with:
```bash
cargo test stats::
```

## Database Maintenance

To clear all statistics:
```bash
rm ~/.local/state/thokr/stats.db
```

The database will be automatically recreated on the next typing test.

To inspect the database manually:
```bash
sqlite3 ~/.local/state/thokr/stats.db
.schema
SELECT * FROM character_stats LIMIT 10;
```