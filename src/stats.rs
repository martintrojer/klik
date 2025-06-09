use chrono::{DateTime, Local};
use directories::ProjectDirs;
use rusqlite::{params, Connection, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::lang::CharacterDifficulty;

/// Character-level statistics for tracking typing performance (used during session)
#[derive(Debug, Clone)]
pub struct CharStat {
    pub character: char,
    pub time_to_press_ms: u64,
    pub was_correct: bool,
    pub timestamp: DateTime<Local>,
    pub context_before: String,
    pub context_after: String,
}

/// Aggregated statistics for a character across multiple attempts in a session
#[derive(Debug, Clone)]
pub struct CharSessionStats {
    pub character: char,
    pub total_attempts: u32,
    pub correct_attempts: u32,
    pub total_time_ms: u64,
    pub min_time_ms: u64,
    pub max_time_ms: u64,
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
        let db_path = Self::get_db_path().unwrap_or_else(|| PathBuf::from("thokr_stats.db"));
        
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
            session_buffer: HashMap::new() 
        })
    }

    /// Get the database file path under $HOME/.local/state/thokr
    fn get_db_path() -> Option<PathBuf> {
        // Try to use the XDG-compliant ~/.local/state directory first
        if let Ok(home) = std::env::var("HOME") {
            let state_dir = PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("thokr");
            Some(state_dir.join("stats.db"))
        } else if let Some(proj_dirs) = ProjectDirs::from("", "", "thokr") {
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
            .or_insert_with(Vec::new)
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
                (character, total_attempts, correct_attempts, total_time_ms, min_time_ms, max_time_ms, session_date)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    stat.character.to_string(),
                    stat.total_attempts,
                    stat.correct_attempts,
                    stat.total_time_ms,
                    stat.min_time_ms,
                    stat.max_time_ms,
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
    fn aggregate_char_stats_from_buffer(buffer: &HashMap<char, Vec<CharStat>>) -> Vec<CharSessionStats> {
        let mut session_stats = Vec::new();
        
        for (&character, stats) in buffer {
            let mut char_session = CharSessionStats {
                character,
                total_attempts: 0,
                correct_attempts: 0,
                total_time_ms: 0,
                min_time_ms: u64::MAX,
                max_time_ms: 0,
            };
            
            for stat in stats {
                char_session.total_attempts += 1;
                if stat.was_correct {
                    char_session.correct_attempts += 1;
                    char_session.total_time_ms += stat.time_to_press_ms;
                    char_session.min_time_ms = char_session.min_time_ms.min(stat.time_to_press_ms);
                    char_session.max_time_ms = char_session.max_time_ms.max(stat.time_to_press_ms);
                }
            }
            
            // Fix min_time_ms for characters with no correct attempts
            if char_session.correct_attempts == 0 {
                char_session.min_time_ms = 0;
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
            SELECT character, total_attempts, correct_attempts, total_time_ms, min_time_ms, max_time_ms
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

        let result: Result<(Option<i64>, Option<i64>), _> = stmt.query_row([character.to_string()], |row| {
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

        let result: Result<(Option<i64>, Option<i64>), _> = stmt.query_row([character.to_string()], |row| {
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
        self.conn.query_row(
            "SELECT COUNT(*) FROM char_session_stats",
            [],
            |row| row.get(0)
        )
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
                SUM(total_attempts) as total_attempts
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

            Ok((character, CharacterDifficulty {
                miss_rate,
                avg_time_ms: avg_time,
                total_attempts,
            }))
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
    end.duration_since(start)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Helper function to extract context around a character position
pub fn extract_context(text: &str, position: usize, context_size: usize) -> (String, String) {
    let chars: Vec<char> = text.chars().collect();
    
    let before_start = if position >= context_size {
        position - context_size
    } else {
        0
    };
    
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
                session_date TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        ).unwrap();
        
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
            session_buffer: HashMap::new() 
        }
    }

    #[test]
    fn test_time_diff_ms() {
        let start = SystemTime::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let end = SystemTime::now();
        
        let diff = time_diff_ms(start, end);
        assert!(diff >= 10);
        assert!(diff < 50); // Should be reasonably close
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
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "ello".to_string(),
            },
            CharStat {
                character: 'h',
                time_to_press_ms: 120,
                was_correct: true,
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
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "est".to_string(),
            },
            CharStat {
                character: 't',
                time_to_press_ms: 150,
                was_correct: false,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "est".to_string(),
            },
            CharStat {
                character: 't',
                time_to_press_ms: 120,
                was_correct: true,
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
}