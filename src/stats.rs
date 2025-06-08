use chrono::{DateTime, Local};
use directories::ProjectDirs;
use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use std::time::SystemTime;

/// Character-level statistics for tracking typing performance
#[derive(Debug, Clone)]
pub struct CharStat {
    pub character: char,
    pub time_to_press_ms: u64,
    pub was_correct: bool,
    pub timestamp: DateTime<Local>,
    pub context_before: String,
    pub context_after: String,
}

/// Database manager for character statistics
#[derive(Debug)]
pub struct StatsDb {
    conn: Connection,
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
        
        // Create the character_stats table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS character_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                character TEXT NOT NULL,
                time_to_press_ms INTEGER NOT NULL,
                was_correct BOOLEAN NOT NULL,
                timestamp TEXT NOT NULL,
                context_before TEXT,
                context_after TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        )?;

        // Create index for faster queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_character_stats_char ON character_stats(character)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_character_stats_timestamp ON character_stats(timestamp)",
            [],
        )?;

        Ok(StatsDb { conn })
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

    /// Record a character statistic
    pub fn record_char_stat(&self, stat: &CharStat) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO character_stats 
            (character, time_to_press_ms, was_correct, timestamp, context_before, context_after)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                stat.character.to_string(),
                stat.time_to_press_ms,
                stat.was_correct,
                stat.timestamp.to_rfc3339(),
                stat.context_before,
                stat.context_after,
            ],
        )?;

        Ok(())
    }

    /// Flush any pending writes to ensure data is committed to disk
    pub fn flush(&self) -> Result<()> {
        // SQLite automatically commits after each INSERT unless in a transaction
        // This is mostly a no-op for our use case, but provides a consistent API
        Ok(())
    }

    /// Record multiple character statistics in a batch transaction
    pub fn record_char_stats_batch(&mut self, stats: &[CharStat]) -> Result<()> {
        let tx = self.conn.transaction()?;
        
        for stat in stats {
            tx.execute(
                r#"
                INSERT INTO character_stats 
                (character, time_to_press_ms, was_correct, timestamp, context_before, context_after)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    stat.character.to_string(),
                    stat.time_to_press_ms,
                    stat.was_correct,
                    stat.timestamp.to_rfc3339(),
                    stat.context_before,
                    stat.context_after,
                ],
            )?;
        }
        
        tx.commit()?;
        Ok(())
    }

    /// Get statistics for a specific character
    pub fn get_char_stats(&self, character: char) -> Result<Vec<CharStat>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT character, time_to_press_ms, was_correct, timestamp, context_before, context_after
            FROM character_stats 
            WHERE character = ?1
            ORDER BY timestamp DESC
            "#,
        )?;

        let stat_iter = stmt.query_map([character.to_string()], |row| {
            let timestamp_str: String = row.get(3)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(3, "timestamp".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Local);

            Ok(CharStat {
                character: row.get::<_, String>(0)?.chars().next().unwrap_or('\0'),
                time_to_press_ms: row.get(1)?,
                was_correct: row.get(2)?,
                timestamp,
                context_before: row.get(4)?,
                context_after: row.get(5)?,
            })
        })?;

        let mut stats = Vec::new();
        for stat in stat_iter {
            stats.push(stat?);
        }

        Ok(stats)
    }

    /// Get average time to press for a character
    pub fn get_avg_time_to_press(&self, character: char) -> Result<Option<f64>> {
        let mut stmt = self.conn.prepare(
            "SELECT AVG(time_to_press_ms) FROM character_stats WHERE character = ?1 AND was_correct = 1",
        )?;

        let avg: Option<f64> = stmt.query_row([character.to_string()], |row| row.get(0))?;
        Ok(avg)
    }

    /// Get miss rate for a character (percentage of incorrect attempts)
    pub fn get_miss_rate(&self, character: char) -> Result<f64> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                COUNT(*) as total,
                SUM(CASE WHEN was_correct = 0 THEN 1 ELSE 0 END) as incorrect
            FROM character_stats 
            WHERE character = ?1
            "#,
        )?;

        let (total, incorrect): (i64, i64) = stmt.query_row([character.to_string()], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

        if total == 0 {
            Ok(0.0)
        } else {
            Ok((incorrect as f64 / total as f64) * 100.0)
        }
    }

    /// Get all character statistics summary
    pub fn get_all_char_summary(&self) -> Result<Vec<(char, f64, f64, i64)>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                character,
                AVG(CASE WHEN was_correct = 1 THEN time_to_press_ms END) as avg_time,
                (SUM(CASE WHEN was_correct = 0 THEN 1 ELSE 0 END) * 100.0 / COUNT(*)) as miss_rate,
                COUNT(*) as total_attempts
            FROM character_stats 
            GROUP BY character
            ORDER BY character
            "#,
        )?;

        let summary_iter = stmt.query_map([], |row| {
            let char_str: String = row.get(0)?;
            let character = char_str.chars().next().unwrap_or('\0');
            let avg_time: Option<f64> = row.get(1)?;
            let miss_rate: f64 = row.get(2)?;
            let total_attempts: i64 = row.get(3)?;

            Ok((character, avg_time.unwrap_or(0.0), miss_rate, total_attempts))
        })?;

        let mut summary = Vec::new();
        for item in summary_iter {
            summary.push(item?);
        }

        Ok(summary)
    }

    /// Clear all statistics (for testing or reset purposes)
    pub fn clear_all_stats(&self) -> Result<()> {
        self.conn.execute("DELETE FROM character_stats", [])?;
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
            CREATE TABLE IF NOT EXISTS character_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                character TEXT NOT NULL,
                time_to_press_ms INTEGER NOT NULL,
                was_correct BOOLEAN NOT NULL,
                timestamp TEXT NOT NULL,
                context_before TEXT,
                context_after TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        ).unwrap();
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_character_stats_char ON character_stats(character)",
            [],
        ).unwrap();

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_character_stats_timestamp ON character_stats(timestamp)",
            [],
        ).unwrap();
        
        StatsDb { conn }
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
    fn test_record_and_retrieve_char_stat() {
        let db = create_test_db();
        
        let stat = CharStat {
            character: 'h',
            time_to_press_ms: 150,
            was_correct: true,
            timestamp: Local::now(),
            context_before: "".to_string(),
            context_after: "ello".to_string(),
        };

        db.record_char_stat(&stat).unwrap();
        
        let stats = db.get_char_stats('h').unwrap();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].character, 'h');
        assert_eq!(stats[0].time_to_press_ms, 150);
        assert!(stats[0].was_correct);
    }

    #[test]
    fn test_avg_time_to_press() {
        let db = create_test_db();
        
        let stats = vec![
            CharStat {
                character: 'a',
                time_to_press_ms: 100,
                was_correct: true,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "bc".to_string(),
            },
            CharStat {
                character: 'a',
                time_to_press_ms: 200,
                was_correct: true,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "bc".to_string(),
            },
        ];

        for stat in stats {
            db.record_char_stat(&stat).unwrap();
        }

        let avg = db.get_avg_time_to_press('a').unwrap();
        assert_eq!(avg, Some(150.0));
    }

    #[test]
    fn test_miss_rate() {
        let db = create_test_db();
        
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
            CharStat {
                character: 't',
                time_to_press_ms: 180,
                was_correct: false,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "est".to_string(),
            },
        ];

        for stat in stats {
            db.record_char_stat(&stat).unwrap();
        }

        let miss_rate = db.get_miss_rate('t').unwrap();
        assert_eq!(miss_rate, 50.0); // 2 out of 4 incorrect = 50%
    }

    #[test]
    fn test_clear_all_stats() {
        let db = create_test_db();
        
        let stat = CharStat {
            character: 'x',
            time_to_press_ms: 100,
            was_correct: true,
            timestamp: Local::now(),
            context_before: "".to_string(),
            context_after: "yz".to_string(),
        };

        db.record_char_stat(&stat).unwrap();
        assert_eq!(db.get_char_stats('x').unwrap().len(), 1);

        db.clear_all_stats().unwrap();
        assert_eq!(db.get_char_stats('x').unwrap().len(), 0);
    }

    #[test]
    fn test_flush() {
        let db = create_test_db();
        
        let stat = CharStat {
            character: 'f',
            time_to_press_ms: 120,
            was_correct: true,
            timestamp: Local::now(),
            context_before: "".to_string(),
            context_after: "oo".to_string(),
        };

        db.record_char_stat(&stat).unwrap();
        db.flush().unwrap();
        
        let stats = db.get_char_stats('f').unwrap();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].character, 'f');
    }

    #[test]
    fn test_batch_record() {
        let mut db = create_test_db();
        
        let stats = vec![
            CharStat {
                character: 'b',
                time_to_press_ms: 100,
                was_correct: true,
                timestamp: Local::now(),
                context_before: "".to_string(),
                context_after: "atch".to_string(),
            },
            CharStat {
                character: 'a',
                time_to_press_ms: 110,
                was_correct: true,
                timestamp: Local::now(),
                context_before: "b".to_string(),
                context_after: "tch".to_string(),
            },
            CharStat {
                character: 't',
                time_to_press_ms: 95,
                was_correct: false,
                timestamp: Local::now(),
                context_before: "ba".to_string(),
                context_after: "ch".to_string(),
            },
        ];

        db.record_char_stats_batch(&stats).unwrap();
        
        assert_eq!(db.get_char_stats('b').unwrap().len(), 1);
        assert_eq!(db.get_char_stats('a').unwrap().len(), 1);
        assert_eq!(db.get_char_stats('t').unwrap().len(), 1);
        
        let miss_rate = db.get_miss_rate('t').unwrap();
        assert_eq!(miss_rate, 100.0); // 1 out of 1 incorrect = 100%
    }
}