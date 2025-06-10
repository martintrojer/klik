use chrono::{DateTime, Local};
use directories::ProjectDirs;
use rusqlite::{params, Connection, Result, OptionalExtension};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::language::CharacterDifficulty;

/// Type alias for character statistics with session deltas
/// (char, historical_avg_time, historical_miss_rate, historical_attempts,
///  session_avg_time_delta, session_miss_rate_delta, session_attempts_delta)
pub type CharSummaryWithDeltas = (char, f64, f64, i64, Option<f64>, Option<f64>, i64);

/// Character-level statistics for tracking typing performance (used during session)
#[derive(Debug, Clone)]
pub struct CharStat {
    pub character: char, // The base character (always lowercase for letters)
    pub time_to_press_ms: u64,
    pub was_correct: bool,
    pub was_uppercase: bool, // True if the original character was uppercase
    pub timestamp: DateTime<Local>,
    pub context_before: String,
    pub context_after: String,
}

/// Aggregated statistics for a character across multiple attempts in a session
#[derive(Debug, Clone)]
pub struct CharSessionStats {
    pub character: char,       // Base character (lowercase)
    pub total_attempts: u32,   // Total attempts for this character (any case)
    pub correct_attempts: u32, // Correct attempts for this character (any case)
    pub total_time_ms: u64,    // Total time for correct attempts (any case)
    pub min_time_ms: u64,      // Fastest time for any case
    pub max_time_ms: u64,      // Slowest time for any case
    // Uppercase-specific metrics
    pub uppercase_attempts: u32, // Total uppercase attempts
    pub uppercase_correct: u32,  // Correct uppercase attempts
    pub uppercase_time_ms: u64,  // Total time for correct uppercase attempts
    pub uppercase_min_time: u64, // Fastest uppercase time
    pub uppercase_max_time: u64, // Slowest uppercase time
}

/// Database manager for character statistics
#[derive(Debug)]
pub struct StatsDb {
    conn: Connection,
    session_buffer: HashMap<char, Vec<CharStat>>,
}

impl StatsDb {
    /// Initialize the database connection and create tables if needed
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path().unwrap_or_else(|| PathBuf::from("klik_stats.db"));

        // Create parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                    Some(format!("Failed to create directory: {}", e)),
                )
            })?;
        }

        let conn = Connection::open(&db_path)?;

        // Create the aggregated character statistics table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS char_session_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                character TEXT NOT NULL,
                total_attempts INTEGER NOT NULL,
                correct_attempts INTEGER NOT NULL,
                total_time_ms INTEGER NOT NULL,
                min_time_ms INTEGER NOT NULL,
                max_time_ms INTEGER NOT NULL,
                uppercase_attempts INTEGER NOT NULL DEFAULT 0,
                uppercase_correct INTEGER NOT NULL DEFAULT 0,
                uppercase_time_ms INTEGER NOT NULL DEFAULT 0,
                uppercase_min_time INTEGER NOT NULL DEFAULT 0,
                uppercase_max_time INTEGER NOT NULL DEFAULT 0,
                session_date TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        )?;

        // Create index for faster queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_char_session_stats_char ON char_session_stats(character)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_char_session_stats_date ON char_session_stats(session_date)",
            [],
        )?;

        Ok(StatsDb {
            conn,
            session_buffer: HashMap::new(),
        })
    }

    /// Get the database file path under $HOME/.local/state/klik
    fn get_db_path() -> Option<PathBuf> {
        // Try to use the XDG-compliant ~/.local/state directory first
        if let Ok(home) = std::env::var("HOME") {
            let state_dir = PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("klik");
            Some(state_dir.join("stats.db"))
        } else if let Some(proj_dirs) = ProjectDirs::from("", "", "klik") {
            // Fallback to system-specific directory
            let state_dir = proj_dirs.data_local_dir();
            Some(state_dir.join("stats.db"))
        } else {
            None
        }
    }

    /// Record a character statistic (buffers for session aggregation)
    pub fn record_char_stat(&mut self, stat: &CharStat) -> Result<()> {
        self.session_buffer
            .entry(stat.character)
            .or_default()
            .push(stat.clone());
        Ok(())
    }

    /// Record aggregated session statistics for characters
    pub fn record_session_stats(&self, session_stats: &[CharSessionStats]) -> Result<()> {
        let session_date = Local::now().format("%Y-%m-%d").to_string();

        for stat in session_stats {
            self.conn.execute(
                r#"
                INSERT INTO char_session_stats 
                (character, total_attempts, correct_attempts, total_time_ms, min_time_ms, max_time_ms, 
                 uppercase_attempts, uppercase_correct, uppercase_time_ms, uppercase_min_time, uppercase_max_time, session_date)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                "#,
                params![
                    stat.character.to_string(),
                    stat.total_attempts,
                    stat.correct_attempts,
                    stat.total_time_ms,
                    stat.min_time_ms,
                    stat.max_time_ms,
                    stat.uppercase_attempts,
                    stat.uppercase_correct,
                    stat.uppercase_time_ms,
                    stat.uppercase_min_time,
                    stat.uppercase_max_time,
                    session_date,
                ],
            )?;
        }

        Ok(())
    }

    /// Flush session buffer to database with aggregated statistics
    pub fn flush(&mut self) -> Result<()> {
        if self.session_buffer.is_empty() {
            return Ok(());
        }

        // Aggregate all buffered stats
        let session_stats = Self::aggregate_char_stats_from_buffer(&self.session_buffer);

        // Record to database
        self.record_session_stats(&session_stats)?;

        // Clear buffer
        self.session_buffer.clear();

        Ok(())
    }

    /// Record multiple character statistics in a batch transaction (aggregated)
    pub fn record_char_stats_batch(&mut self, stats: &[CharStat]) -> Result<()> {
        // Add to session buffer
        for stat in stats {
            self.record_char_stat(stat)?;
        }

        // Immediately flush for session end
        self.flush()
    }

    /// Aggregate buffered character statistics into session summaries
    fn aggregate_char_stats_from_buffer(
        buffer: &HashMap<char, Vec<CharStat>>,
    ) -> Vec<CharSessionStats> {
        let mut session_stats = Vec::new();

        for (&character, stats) in buffer {
            let mut char_session = CharSessionStats {
                character,
                total_attempts: 0,
                correct_attempts: 0,
                total_time_ms: 0,
                min_time_ms: u64::MAX,
                max_time_ms: 0,
                uppercase_attempts: 0,
                uppercase_correct: 0,
                uppercase_time_ms: 0,
                uppercase_min_time: u64::MAX,
                uppercase_max_time: 0,
            };

            for stat in stats {
                char_session.total_attempts += 1;

                if stat.was_uppercase {
                    char_session.uppercase_attempts += 1;
                }

                if stat.was_correct {
                    char_session.correct_attempts += 1;
                    char_session.total_time_ms += stat.time_to_press_ms;
                    char_session.min_time_ms = char_session.min_time_ms.min(stat.time_to_press_ms);
                    char_session.max_time_ms = char_session.max_time_ms.max(stat.time_to_press_ms);

                    if stat.was_uppercase {
                        char_session.uppercase_correct += 1;
                        char_session.uppercase_time_ms += stat.time_to_press_ms;
                        char_session.uppercase_min_time =
                            char_session.uppercase_min_time.min(stat.time_to_press_ms);
                        char_session.uppercase_max_time =
                            char_session.uppercase_max_time.max(stat.time_to_press_ms);
                    }
                }
            }

            // Fix min_time_ms for characters with no correct attempts
            if char_session.correct_attempts == 0 {
                char_session.min_time_ms = 0;
            }
            if char_session.uppercase_correct == 0 {
                char_session.uppercase_min_time = 0;
            }

            session_stats.push(char_session);
        }

        session_stats
    }

    /// Get session statistics for a specific character
    pub fn get_char_stats(&self, _character: char) -> Result<Vec<CharStat>> {
        // Return empty for now since we're moving to aggregated data
        // This maintains API compatibility but reduces storage
        Ok(Vec::new())
    }

    /// Get session-based statistics for a specific character
    pub fn get_char_session_stats(&self, character: char) -> Result<Vec<CharSessionStats>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT character, total_attempts, correct_attempts, total_time_ms, min_time_ms, max_time_ms,
                   uppercase_attempts, uppercase_correct, uppercase_time_ms, uppercase_min_time, uppercase_max_time
            FROM char_session_stats 
            WHERE character = ?1
            ORDER BY session_date DESC
            "#,
        )?;

        let stat_iter = stmt.query_map([character.to_string()], |row| {
            Ok(CharSessionStats {
                character: row.get::<_, String>(0)?.chars().next().unwrap_or('\0'),
                total_attempts: row.get(1)?,
                correct_attempts: row.get(2)?,
                total_time_ms: row.get(3)?,
                min_time_ms: row.get(4)?,
                max_time_ms: row.get(5)?,
                uppercase_attempts: row.get(6)?,
                uppercase_correct: row.get(7)?,
                uppercase_time_ms: row.get(8)?,
                uppercase_min_time: row.get(9)?,
                uppercase_max_time: row.get(10)?,
            })
        })?;

        let mut stats = Vec::new();
        for stat in stat_iter {
            stats.push(stat?);
        }

        Ok(stats)
    }

    /// Get average time to press for a character (from aggregated session data)
    pub fn get_avg_time_to_press(&self, character: char) -> Result<Option<f64>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT SUM(total_time_ms), SUM(correct_attempts)
            FROM char_session_stats 
            WHERE character = ?1 AND correct_attempts > 0
            "#,
        )?;

        let result: Result<(Option<i64>, Option<i64>), _> = stmt
            .query_row([character.to_string()], |row| {
                Ok((row.get(0)?, row.get(1)?))
            });

        match result {
            Ok((Some(total_time), Some(total_correct))) if total_correct > 0 => {
                Ok(Some(total_time as f64 / total_correct as f64))
            }
            _ => Ok(None),
        }
    }

    /// Get miss rate for a character (percentage of incorrect attempts) from aggregated data
    pub fn get_miss_rate(&self, character: char) -> Result<f64> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                SUM(total_attempts) as total,
                SUM(total_attempts - correct_attempts) as incorrect
            FROM char_session_stats 
            WHERE character = ?1
            "#,
        )?;

        let result: Result<(Option<i64>, Option<i64>), _> = stmt
            .query_row([character.to_string()], |row| {
                Ok((row.get(0)?, row.get(1)?))
            });

        match result {
            Ok((Some(total), Some(incorrect))) if total > 0 => {
                Ok((incorrect as f64 / total as f64) * 100.0)
            }
            _ => Ok(0.0),
        }
    }

    /// Get all character statistics summary from aggregated session data
    pub fn get_all_char_summary(&self) -> Result<Vec<(char, f64, f64, i64)>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                character,
                CASE 
                    WHEN SUM(correct_attempts) > 0 THEN 
                        CAST(SUM(total_time_ms) AS FLOAT) / SUM(correct_attempts)
                    ELSE 0.0
                END as avg_time,
                CASE 
                    WHEN SUM(total_attempts) > 0 THEN 
                        (SUM(total_attempts - correct_attempts) * 100.0 / SUM(total_attempts))
                    ELSE 0.0
                END as miss_rate,
                SUM(total_attempts) as total_attempts
            FROM char_session_stats 
            GROUP BY character
            ORDER BY character
            "#,
        )?;

        let summary_iter = stmt.query_map([], |row| {
            let char_str: String = row.get(0)?;
            let character = char_str.chars().next().unwrap_or('\0');
            let avg_time: f64 = row.get(1)?;
            let miss_rate: f64 = row.get(2)?;
            let total_attempts: i64 = row.get(3)?;

            Ok((character, avg_time, miss_rate, total_attempts))
        })?;

        let mut summary = Vec::new();
        for item in summary_iter {
            summary.push(item?);
        }

        Ok(summary)
    }

    /// Get historical character statistics summary excluding today's session
    /// This is used for delta calculations to compare against truly historical data
    pub fn get_historical_char_summary(&self) -> Result<Vec<(char, f64, f64, i64)>> {
        // Get the timestamp of the most recent session
        let mut max_stmt = self.conn.prepare(
            "SELECT MAX(created_at) FROM char_session_stats"
        )?;
        
        let latest_timestamp: Option<String> = max_stmt.query_row([], |row| {
            row.get(0)
        }).optional()?;
        
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                character,
                CASE 
                    WHEN SUM(correct_attempts) > 0 THEN 
                        CAST(SUM(total_time_ms) AS FLOAT) / SUM(correct_attempts)
                    ELSE 0.0
                END as avg_time,
                CASE 
                    WHEN SUM(total_attempts) > 0 THEN 
                        (SUM(total_attempts - correct_attempts) * 100.0 / SUM(total_attempts))
                    ELSE 0.0
                END as miss_rate,
                SUM(total_attempts) as total_attempts
            FROM char_session_stats 
            WHERE created_at < ?1 OR ?1 IS NULL
            GROUP BY character
            ORDER BY character
            "#,
        )?;

        let summary_iter = stmt.query_map([latest_timestamp], |row| {
            let char_str: String = row.get(0)?;
            let character = char_str.chars().next().unwrap_or('\0');
            let avg_time: f64 = row.get(1)?;
            let miss_rate: f64 = row.get(2)?;
            let total_attempts: i64 = row.get(3)?;

            Ok((character, avg_time, miss_rate, total_attempts))
        })?;

        let mut summary = Vec::new();
        for item in summary_iter {
            summary.push(item?);
        }

        Ok(summary)
    }

    /// Get session statistics from the current session buffer
    pub fn get_current_session_summary(&self) -> Vec<(char, f64, f64, i64)> {
        let mut summary = Vec::new();

        for (&character, stats) in &self.session_buffer {
            let total_attempts = stats.len() as i64;
            let correct_attempts = stats.iter().filter(|s| s.was_correct).count();

            let avg_time = if correct_attempts > 0 {
                let total_time: u64 = stats
                    .iter()
                    .filter(|s| s.was_correct)
                    .map(|s| s.time_to_press_ms)
                    .sum();
                total_time as f64 / correct_attempts as f64
            } else {
                0.0
            };

            let miss_rate = if total_attempts > 0 {
                let incorrect_attempts = total_attempts - correct_attempts as i64;
                (incorrect_attempts as f64 / total_attempts as f64) * 100.0
            } else {
                0.0
            };

            summary.push((character, avg_time, miss_rate, total_attempts));
        }

        summary.sort_by(|a, b| a.0.cmp(&b.0));
        summary
    }

    /// Get session statistics from the most recent session in the database
    /// This is used for delta calculations after the session buffer has been flushed
    pub fn get_latest_session_summary(&self) -> Result<Vec<(char, f64, f64, i64)>> {
        // Get the timestamp of the most recent session
        let mut max_stmt = self.conn.prepare(
            "SELECT MAX(created_at) FROM char_session_stats"
        )?;
        
        let latest_timestamp: Option<String> = max_stmt.query_row([], |row| {
            row.get(0)
        }).optional()?;
        
        if latest_timestamp.is_none() {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                character,
                CASE 
                    WHEN correct_attempts > 0 THEN 
                        CAST(total_time_ms AS FLOAT) / correct_attempts
                    ELSE 0.0
                END as avg_time,
                CASE 
                    WHEN total_attempts > 0 THEN 
                        (total_attempts - correct_attempts) * 100.0 / total_attempts
                    ELSE 0.0
                END as miss_rate,
                total_attempts
            FROM char_session_stats 
            WHERE created_at = ?1
            ORDER BY character
            "#,
        )?;

        let summary_iter = stmt.query_map([latest_timestamp], |row| {
            let char_str: String = row.get(0)?;
            let character = char_str.chars().next().unwrap_or('\0');
            let avg_time: f64 = row.get(1)?;
            let miss_rate: f64 = row.get(2)?;
            let total_attempts: i64 = row.get(3)?;

            Ok((character, avg_time, miss_rate, total_attempts))
        })?;

        let mut summary = Vec::new();
        for item in summary_iter {
            summary.push(item?);
        }

        Ok(summary)
    }

    /// Get character statistics with session deltas
    /// Returns: (char, historical_avg_time, historical_miss_rate, historical_attempts,
    ///          session_avg_time_delta, session_miss_rate_delta, session_attempts_delta)
    pub fn get_char_summary_with_deltas(&self) -> Result<Vec<CharSummaryWithDeltas>> {
        let historical_summary = self.get_historical_char_summary()?;

        // Try current session buffer first, fallback to latest session from database
        let session_summary = if self.session_buffer.is_empty() {
            // Session buffer is empty (already flushed), get latest session from database
            self.get_latest_session_summary()
                .unwrap_or_else(|_| Vec::new())
        } else {
            // Session buffer has data, use it
            self.get_current_session_summary()
        };

        let mut combined_summary = Vec::new();

        // Create a map of session data for quick lookup
        let session_map: std::collections::HashMap<char, (f64, f64, i64)> = session_summary
            .iter()
            .map(|(c, avg_time, miss_rate, attempts)| (*c, (*avg_time, *miss_rate, *attempts)))
            .collect();

        // Process historical data and add session deltas
        for (character, hist_avg_time, hist_miss_rate, hist_attempts) in historical_summary {
            let (session_avg_delta, session_miss_delta, session_attempts) = if let Some(&(
                session_avg,
                session_miss,
                session_att,
            )) =
                session_map.get(&character)
            {
                // Calculate deltas: negative means improvement for time/miss_rate
                let avg_delta = if session_avg > 0.0 && hist_avg_time > 0.0 {
                    Some(session_avg - hist_avg_time)
                } else {
                    None
                };
                let miss_delta = if hist_attempts > 0 {
                    Some(session_miss - hist_miss_rate)
                } else {
                    None
                };
                (avg_delta, miss_delta, session_att)
            } else {
                (None, None, 0)
            };

            combined_summary.push((
                character,
                hist_avg_time,
                hist_miss_rate,
                hist_attempts,
                session_avg_delta,
                session_miss_delta,
                session_attempts,
            ));
        }

        // Add characters that are only in the current session (new characters)
        for (character, session_avg, session_miss, session_attempts) in &session_summary {
            if !combined_summary
                .iter()
                .any(|(c, _, _, _, _, _, _)| c == character)
            {
                combined_summary.push((
                    *character,
                    *session_avg, // Use session data as historical since it's new
                    *session_miss,
                    *session_attempts,
                    None, // No delta for new characters
                    None,
                    *session_attempts,
                ));
            }
        }

        Ok(combined_summary)
    }

    /// Clear all statistics (for testing or reset purposes)
    pub fn clear_all_stats(&self) -> Result<()> {
        self.conn.execute("DELETE FROM char_session_stats", [])?;
        Ok(())
    }

    /// Get the actual database file path being used (for debugging)
    pub fn get_database_path() -> Option<PathBuf> {
        Self::get_db_path()
    }

    /// Check if the database file exists on disk
    pub fn database_exists() -> bool {
        if let Some(path) = Self::get_db_path() {
            path.exists()
        } else {
            false
        }
    }

    /// Get session statistics count
    pub fn get_session_count(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM char_session_stats", [], |row| {
                row.get(0)
            })
    }

    /// Get database size in bytes (useful for determining if compaction is needed)
    pub fn get_database_size(&self) -> Result<i64> {
        self.conn.query_row(
            "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        )
    }

    /// Check if database needs compaction based on size and age criteria
    pub fn needs_compaction(&self) -> Result<bool> {
        let session_count = self.get_session_count()?;
        let db_size = self.get_database_size()?;

        // Trigger compaction if:
        // - More than 1000 session records, OR
        // - Database size exceeds 10MB
        Ok(session_count > 1000 || db_size > 10 * 1024 * 1024)
    }

    /// Compact the database by merging older session data while preserving statistical accuracy
    /// This reduces storage while maintaining all the information needed for character difficulty analysis
    pub fn compact_database(&mut self) -> Result<()> {
        // Start a transaction for atomic operations
        let tx = self.conn.transaction()?;

        // Create a temporary table to store compacted data
        tx.execute(
            r#"
            CREATE TEMPORARY TABLE compacted_stats AS
            SELECT 
                character,
                SUM(total_attempts) as total_attempts,
                SUM(correct_attempts) as correct_attempts,
                SUM(total_time_ms) as total_time_ms,
                MIN(CASE WHEN min_time_ms > 0 THEN min_time_ms ELSE NULL END) as min_time_ms,
                MAX(max_time_ms) as max_time_ms,
                SUM(uppercase_attempts) as uppercase_attempts,
                SUM(uppercase_correct) as uppercase_correct,
                SUM(uppercase_time_ms) as uppercase_time_ms,
                MIN(CASE WHEN uppercase_min_time > 0 THEN uppercase_min_time ELSE NULL END) as uppercase_min_time,
                MAX(uppercase_max_time) as uppercase_max_time,
                'compacted_' || date('now') as session_date
            FROM char_session_stats
            WHERE session_date < date('now', '-30 days')  -- Only compact data older than 30 days
            GROUP BY character
            HAVING COUNT(*) > 1  -- Only compact characters with multiple sessions
            "#,
            [],
        )?;

        // Count how many records we're about to compact
        let records_to_compact: i64 = tx.query_row(
            "SELECT COUNT(*) FROM char_session_stats WHERE session_date < date('now', '-30 days')",
            [],
            |row| row.get(0),
        )?;

        // Count how many compacted records we'll create
        let compacted_records: i64 =
            tx.query_row("SELECT COUNT(*) FROM compacted_stats", [], |row| row.get(0))?;

        // Only proceed if compaction will actually reduce the number of records
        if compacted_records > 0 && records_to_compact > compacted_records {
            // Remove the old records that we're compacting
            tx.execute(
                "DELETE FROM char_session_stats WHERE session_date < date('now', '-30 days')",
                [],
            )?;

            // Insert the compacted data back into the main table
            tx.execute(
                r#"
                INSERT INTO char_session_stats (
                    character, total_attempts, correct_attempts, total_time_ms, 
                    min_time_ms, max_time_ms, uppercase_attempts, uppercase_correct, 
                    uppercase_time_ms, uppercase_min_time, uppercase_max_time, session_date
                )
                SELECT 
                    character, total_attempts, correct_attempts, total_time_ms,
                    COALESCE(min_time_ms, 0), max_time_ms, uppercase_attempts, uppercase_correct,
                    uppercase_time_ms, COALESCE(uppercase_min_time, 0), uppercase_max_time, session_date
                FROM compacted_stats
                "#,
                [],
            )?;

            // Clean up the temporary table
            tx.execute("DROP TABLE compacted_stats", [])?;
        }

        tx.commit()?;

        // Perform VACUUM and ANALYZE outside of transaction
        if compacted_records > 0 && records_to_compact > compacted_records {
            // Optimize the database file
            self.conn.execute("VACUUM", [])?;

            // Update statistics for query optimizer
            self.conn.execute("ANALYZE", [])?;
        }
        Ok(())
    }

    /// Perform automatic compaction if needed (called periodically)
    pub fn auto_compact(&mut self) -> Result<()> {
        if self.needs_compaction()? {
            self.compact_database()
        } else {
            Ok(())
        }
    }

    /// Get compaction statistics for debugging/monitoring
    pub fn get_compaction_info(&self) -> Result<(i64, i64, f64)> {
        let session_count = self.get_session_count()?;
        let db_size = self.get_database_size()?;
        let db_size_mb = db_size as f64 / (1024.0 * 1024.0);

        Ok((session_count, db_size, db_size_mb))
    }

    /// Get character difficulty metrics for intelligent word selection
    pub fn get_character_difficulties(&self) -> Result<HashMap<char, CharacterDifficulty>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                character,
                CASE 
                    WHEN SUM(correct_attempts) > 0 THEN 
                        CAST(SUM(total_time_ms) AS FLOAT) / SUM(correct_attempts)
                    ELSE 500.0
                END as avg_time,
                CASE 
                    WHEN SUM(total_attempts) > 0 THEN 
                        (SUM(total_attempts - correct_attempts) * 100.0 / SUM(total_attempts))
                    ELSE 50.0
                END as miss_rate,
                SUM(total_attempts) as total_attempts,
                CASE 
                    WHEN SUM(uppercase_correct) > 0 THEN 
                        CAST(SUM(uppercase_time_ms) AS FLOAT) / SUM(uppercase_correct)
                    ELSE 700.0
                END as uppercase_avg_time,
                CASE 
                    WHEN SUM(uppercase_attempts) > 0 THEN 
                        (SUM(uppercase_attempts - uppercase_correct) * 100.0 / SUM(uppercase_attempts))
                    ELSE 75.0
                END as uppercase_miss_rate,
                SUM(uppercase_attempts) as uppercase_attempts
            FROM char_session_stats 
            GROUP BY character
            HAVING SUM(total_attempts) >= 3  -- Only include characters with sufficient data
            ORDER BY character
            "#,
        )?;

        let difficulty_iter = stmt.query_map([], |row| {
            let char_str: String = row.get(0)?;
            let character = char_str.chars().next().unwrap_or('\0');
            let avg_time: f64 = row.get(1)?;
            let miss_rate: f64 = row.get(2)?;
            let total_attempts: i64 = row.get(3)?;
            let uppercase_avg_time: f64 = row.get(4)?;
            let uppercase_miss_rate: f64 = row.get(5)?;
            let uppercase_attempts: i64 = row.get(6)?;

            // Calculate uppercase penalty based on performance difference
            let uppercase_penalty = if uppercase_attempts > 0 {
                let time_penalty = (uppercase_avg_time - avg_time).max(0.0) / avg_time;
                let miss_penalty = (uppercase_miss_rate - miss_rate).max(0.0) / 100.0;
                (time_penalty + miss_penalty).min(1.0) // Cap at 1.0
            } else {
                0.5 // Default penalty when no uppercase data
            };

            Ok((
                character,
                CharacterDifficulty {
                    miss_rate,
                    avg_time_ms: avg_time,
                    total_attempts,
                    uppercase_miss_rate,
                    uppercase_avg_time,
                    uppercase_attempts,
                    uppercase_penalty,
                },
            ))
        })?;

        let mut difficulties = HashMap::new();
        for item in difficulty_iter {
            let (character, difficulty) = item?;
            difficulties.insert(character, difficulty);
        }

        Ok(difficulties)
    }
}

/// Helper function to calculate time difference in milliseconds
pub fn time_diff_ms(start: SystemTime, end: SystemTime) -> u64 {
    end.duration_since(start).unwrap_or_default().as_millis() as u64
}

/// Helper function to extract context around a character position
pub fn extract_context(text: &str, position: usize, context_size: usize) -> (String, String) {
    let chars: Vec<char> = text.chars().collect();

    let before_start = position.saturating_sub(context_size);

    let after_end = std::cmp::min(position + context_size + 1, chars.len());

    let before: String = chars[before_start..position].iter().collect();
    let after: String = chars[position + 1..after_end].iter().collect();

    (before, after)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> StatsDb {
        // Create an in-memory database for testing
        let conn = Connection::open_in_memory().unwrap();

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS char_session_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                character TEXT NOT NULL,
                total_attempts INTEGER NOT NULL,
                correct_attempts INTEGER NOT NULL,
                total_time_ms INTEGER NOT NULL,
                min_time_ms INTEGER NOT NULL,
                max_time_ms INTEGER NOT NULL,
                uppercase_attempts INTEGER NOT NULL DEFAULT 0,
                uppercase_correct INTEGER NOT NULL DEFAULT 0,
                uppercase_time_ms INTEGER NOT NULL DEFAULT 0,
                uppercase_min_time INTEGER NOT NULL DEFAULT 0,
                uppercase_max_time INTEGER NOT NULL DEFAULT 0,
                session_date TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_char_session_stats_char ON char_session_stats(character)",
            [],
        ).unwrap();

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_char_session_stats_date ON char_session_stats(session_date)",
            [],
        ).unwrap();

        StatsDb {
            conn,
            session_buffer: HashMap::new(),
        }
    }

    #[test]
    fn test_time_diff_ms() {
        let start = SystemTime::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let end = SystemTime::now();

        let diff = time_diff_ms(start, end);
        assert!(diff >= 8); // Allow some timing variance
        assert!(diff < 100); // More generous upper bound for slower systems
    }

    #[test]
    fn test_extract_context() {
        let text = "hello world test";
        let (before, after) = extract_context(text, 6, 3);

        assert_eq!(before, "lo ");
        assert_eq!(after, "orl");
    }

    #[test]
    fn test_extract_context_at_beginning() {
        let text = "hello world";
        let (before, after) = extract_context(text, 0, 3);

        assert_eq!(before, "");
        assert_eq!(after, "ell");
    }

    #[test]
    fn test_extract_context_at_end() {
        let text = "hello world";
        let (before, after) = extract_context(text, 10, 3);

        assert_eq!(before, "orl");
        assert_eq!(after, "");
    }

    #[test]
    fn test_record_and_retrieve_aggregated_stats() {
        let mut db = create_test_db();

        let stats = vec![
            CharStat {
                character: 'h',
                time_to_press_ms: 150,
                was_correct: true,
                was_uppercase: false,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "ello".to_string(),
            },
            CharStat {
                character: 'h',
                time_to_press_ms: 120,
                was_correct: true,
                was_uppercase: false,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "ello".to_string(),
            },
        ];

        db.record_char_stats_batch(&stats).unwrap();

        let avg = db.get_avg_time_to_press('h').unwrap();
        assert_eq!(avg, Some(135.0)); // (150 + 120) / 2
    }

    #[test]
    fn test_session_aggregation() {
        let mut db = create_test_db();

        let stats = vec![
            CharStat {
                character: 't',
                time_to_press_ms: 100,
                was_correct: true,
                was_uppercase: false,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "est".to_string(),
            },
            CharStat {
                character: 't',
                time_to_press_ms: 150,
                was_correct: false,
                was_uppercase: false,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "est".to_string(),
            },
            CharStat {
                character: 't',
                time_to_press_ms: 120,
                was_correct: true,
                was_uppercase: false,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "est".to_string(),
            },
        ];

        db.record_char_stats_batch(&stats).unwrap();

        let miss_rate = db.get_miss_rate('t').unwrap();
        assert!((miss_rate - 33.33).abs() < 0.1); // 1 out of 3 = 33.33%

        let avg_time = db.get_avg_time_to_press('t').unwrap();
        assert_eq!(avg_time, Some(110.0)); // (100 + 120) / 2 (only correct attempts)
    }

    #[test]
    fn test_clear_all_stats() {
        let mut db = create_test_db();

        let stat = CharStat {
            character: 'x',
            time_to_press_ms: 100,
            was_correct: true,
            was_uppercase: false,
            timestamp: Local::now(),
            context_before: "".to_string(),
            context_after: "yz".to_string(),
        };

        db.record_char_stats_batch(&[stat]).unwrap();
        let summary_before = db.get_all_char_summary().unwrap();
        assert_eq!(summary_before.len(), 1);

        db.clear_all_stats().unwrap();
        let summary_after = db.get_all_char_summary().unwrap();
        assert_eq!(summary_after.len(), 0);
    }

    #[test]
    fn test_flush() {
        let mut db = create_test_db();

        let stat = CharStat {
            character: 'f',
            time_to_press_ms: 120,
            was_correct: true,
            was_uppercase: false,
            timestamp: Local::now(),
            context_before: "".to_string(),
            context_after: "oo".to_string(),
        };

        db.record_char_stat(&stat).unwrap();

        // Before flush, no stats in database
        let summary_before = db.get_all_char_summary().unwrap();
        assert_eq!(summary_before.len(), 0);

        db.flush().unwrap();

        // After flush, stats are in database
        let summary_after = db.get_all_char_summary().unwrap();
        assert_eq!(summary_after.len(), 1);
        assert_eq!(summary_after[0].0, 'f');
    }

    #[test]
    fn test_session_count() {
        let db = create_test_db();
        let session_count = db.get_session_count().unwrap();

        // New database should have no entries
        assert_eq!(session_count, 0);
    }

    #[test]
    fn test_database_size() {
        let db = create_test_db();
        let size = db.get_database_size().unwrap();

        // New database should have some minimal size
        assert!(size > 0);
    }

    #[test]
    fn test_needs_compaction() {
        let db = create_test_db();

        // New database should not need compaction
        assert!(!db.needs_compaction().unwrap());
    }

    #[test]
    fn test_compaction_info() {
        let db = create_test_db();
        let (session_count, db_size, db_size_mb) = db.get_compaction_info().unwrap();

        assert_eq!(session_count, 0);
        assert!(db_size > 0);
        assert!(db_size_mb >= 0.0);
    }

    #[test]
    fn test_auto_compact() {
        let mut db = create_test_db();

        // Should not fail even with empty database
        assert!(db.auto_compact().is_ok());
    }

    #[test]
    fn test_compaction_preserves_data() {
        let mut db = create_test_db();

        // Add some test data with old dates
        let conn = &db.conn;

        // Insert some old data that should be compacted
        for i in 0..5 {
            conn.execute(
                r#"
                INSERT INTO char_session_stats (
                    character, total_attempts, correct_attempts, total_time_ms,
                    min_time_ms, max_time_ms, uppercase_attempts, uppercase_correct,
                    uppercase_time_ms, uppercase_min_time, uppercase_max_time, session_date
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                "#,
                params![
                    "a",
                    10 + i,
                    8 + i,
                    (150 + i * 10) * (8 + i),
                    100 + i * 5,
                    200 + i * 10,
                    2 + i / 2,
                    1 + i / 3,
                    (200 + i * 15) * (1 + i / 3),
                    120 + i * 8,
                    250 + i * 12,
                    format!("2023-01-{:02}", i + 1) // Old dates
                ],
            )
            .unwrap();
        }

        // Insert some recent data that should not be compacted
        conn.execute(
            r#"
            INSERT INTO char_session_stats (
                character, total_attempts, correct_attempts, total_time_ms,
                min_time_ms, max_time_ms, uppercase_attempts, uppercase_correct,
                uppercase_time_ms, uppercase_min_time, uppercase_max_time, session_date
            )
            VALUES ('a', 10, 8, 1200, 100, 200, 2, 1, 200, 120, 250, date('now'))
            "#,
            [],
        )
        .unwrap();

        // Get statistics before compaction
        let avg_before = db.get_avg_time_to_press('a').unwrap().unwrap();
        let miss_rate_before = db.get_miss_rate('a').unwrap();
        let session_count_before = db.get_session_count().unwrap();

        // Perform compaction
        db.compact_database().unwrap();

        // Get statistics after compaction
        let avg_after = db.get_avg_time_to_press('a').unwrap().unwrap();
        let miss_rate_after = db.get_miss_rate('a').unwrap();
        let session_count_after = db.get_session_count().unwrap();

        // Statistics should be preserved (approximately, allowing for floating point precision)
        assert!((avg_before - avg_after).abs() < 1.0);
        assert!((miss_rate_before - miss_rate_after).abs() < 1.0);

        // Session count should be reduced (old sessions compacted)
        assert!(session_count_after < session_count_before);

        // Should still have at least one record (the recent one + compacted data)
        assert!(session_count_after >= 1);
    }

    #[test]
    fn test_compaction_with_no_old_data() {
        let mut db = create_test_db();

        // Add only recent data
        let conn = &db.conn;
        conn.execute(
            r#"
            INSERT INTO char_session_stats (
                character, total_attempts, correct_attempts, total_time_ms,
                min_time_ms, max_time_ms, uppercase_attempts, uppercase_correct,
                uppercase_time_ms, uppercase_min_time, uppercase_max_time, session_date
            )
            VALUES ('b', 10, 8, 1200, 100, 200, 2, 1, 200, 120, 250, date('now'))
            "#,
            [],
        )
        .unwrap();

        let session_count_before = db.get_session_count().unwrap();

        // Compaction should not change anything with recent data
        db.compact_database().unwrap();

        let session_count_after = db.get_session_count().unwrap();
        assert_eq!(session_count_before, session_count_after);
    }

    #[test]
    fn test_current_session_summary() {
        let mut db = create_test_db();

        // Add some stats to the session buffer
        let stats = vec![
            CharStat {
                character: 'a',
                time_to_press_ms: 100,
                was_correct: true,
                was_uppercase: false,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "bc".to_string(),
            },
            CharStat {
                character: 'a',
                time_to_press_ms: 150,
                was_correct: false,
                was_uppercase: false,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "bc".to_string(),
            },
            CharStat {
                character: 'b',
                time_to_press_ms: 120,
                was_correct: true,
                was_uppercase: false,
                timestamp: Local::now(),
                context_before: "a".to_string(),
                context_after: "c".to_string(),
            },
        ];

        for stat in stats {
            db.record_char_stat(&stat).unwrap();
        }

        let session_summary = db.get_current_session_summary();

        assert_eq!(session_summary.len(), 2); // 'a' and 'b'

        // Find character 'a' in summary
        let a_stats = session_summary
            .iter()
            .find(|(c, _, _, _)| *c == 'a')
            .unwrap();
        assert_eq!(a_stats.1, 100.0); // avg_time (only correct attempts)
        assert_eq!(a_stats.2, 50.0); // miss_rate (1 out of 2)
        assert_eq!(a_stats.3, 2); // total_attempts

        // Find character 'b' in summary
        let b_stats = session_summary
            .iter()
            .find(|(c, _, _, _)| *c == 'b')
            .unwrap();
        assert_eq!(b_stats.1, 120.0); // avg_time
        assert_eq!(b_stats.2, 0.0); // miss_rate (0 out of 1)
        assert_eq!(b_stats.3, 1); // total_attempts
    }

    #[test]
    fn test_char_summary_with_deltas() {
        let mut db = create_test_db();

        // Add historical data to the database
        let conn = &db.conn;
        conn.execute(
            r#"
            INSERT INTO char_session_stats (
                character, total_attempts, correct_attempts, total_time_ms,
                min_time_ms, max_time_ms, uppercase_attempts, uppercase_correct,
                uppercase_time_ms, uppercase_min_time, uppercase_max_time, session_date
            )
            VALUES ('a', 10, 8, 1600, 100, 250, 2, 1, 200, 120, 250, '2024-01-01')
            "#,
            [],
        )
        .unwrap();

        // Add session data to the buffer
        let session_stats = vec![
            CharStat {
                character: 'a',
                time_to_press_ms: 150, // Faster than historical (200ms avg)
                was_correct: true,
                was_uppercase: false,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "bc".to_string(),
            },
            CharStat {
                character: 'a',
                time_to_press_ms: 170,
                was_correct: true,
                was_uppercase: false,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "bc".to_string(),
            },
        ];

        for stat in session_stats {
            db.record_char_stat(&stat).unwrap();
        }

        let summary_with_deltas = db.get_char_summary_with_deltas().unwrap();

        assert_eq!(summary_with_deltas.len(), 1);

        let (
            character,
            hist_avg,
            hist_miss,
            hist_attempts,
            time_delta,
            miss_delta,
            session_attempts,
        ) = &summary_with_deltas[0];

        assert_eq!(*character, 'a');
        assert_eq!(*hist_avg, 200.0); // 1600 / 8
        assert_eq!(*hist_miss, 20.0); // (10-8)/10 * 100
        assert_eq!(*hist_attempts, 10);
        assert_eq!(*session_attempts, 2);

        // Session average: (150+170)/2 = 160ms
        // Delta: 160 - 200 = -40ms (improvement)
        assert!(time_delta.is_some());
        assert_eq!(time_delta.unwrap(), -40.0);

        // Session miss rate: 0% (both correct)
        // Delta: 0 - 20 = -20% (improvement)
        assert!(miss_delta.is_some());
        assert_eq!(miss_delta.unwrap(), -20.0);

        println!(
            "✅ Delta test: time_delta={:?}, miss_delta={:?}",
            time_delta, miss_delta
        );
    }

    #[test]
    fn test_char_summary_with_new_character() {
        let mut db = create_test_db();

        // Add session data for a character not in historical data
        let session_stat = CharStat {
            character: 'z',
            time_to_press_ms: 180,
            was_correct: true,
            was_uppercase: false,
            timestamp: Local::now(),
            context_before: "".to_string(),
            context_after: "".to_string(),
        };

        db.record_char_stat(&session_stat).unwrap();

        let summary_with_deltas = db.get_char_summary_with_deltas().unwrap();

        assert_eq!(summary_with_deltas.len(), 1);

        let (
            character,
            hist_avg,
            hist_miss,
            hist_attempts,
            time_delta,
            miss_delta,
            session_attempts,
        ) = &summary_with_deltas[0];

        assert_eq!(*character, 'z');
        assert_eq!(*hist_avg, 180.0); // Uses session data as historical
        assert_eq!(*hist_miss, 0.0); // Uses session data as historical
        assert_eq!(*hist_attempts, 1); // Uses session data as historical
        assert_eq!(*session_attempts, 1);

        // No deltas for new characters
        assert!(time_delta.is_none());
        assert!(miss_delta.is_none());
    }
}
