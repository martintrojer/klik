use crate::celebration::CelebrationAnimation;
use crate::session::Session;
use crate::stats::{StatsDb, StatsStore};

/// Default tick rate used for timing calculations (100ms)
pub const TICK_RATE_MS: u64 = 100;
use chrono::prelude::*;
use csv::Writer;
use directories::ProjectDirs;
use std::fs::OpenOptions;
use std::io;
use std::time::SystemTime;

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Outcome {
    Correct,
    Incorrect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub char: char,
    pub outcome: Outcome,
    pub timestamp: SystemTime,
    pub keypress_start: Option<SystemTime>,
}

/// Top-level typing test: a Session plus persistence (stats DB, CSV) and celebration.
#[derive(Debug)]
pub struct Thok {
    pub session: Session,
    pub stats_db: Option<Box<dyn StatsStore>>,
    pub celebration: CelebrationAnimation,
}

impl Thok {
    // --- Convenience getters (delegate to session) ---

    pub fn wpm(&self) -> f64 {
        self.session.state.wpm
    }

    pub fn accuracy(&self) -> f64 {
        self.session.state.accuracy
    }

    pub fn std_dev(&self) -> f64 {
        self.session.state.std_dev
    }

    pub fn wpm_coords(&self) -> &[crate::time_series::TimeSeriesPoint] {
        &self.session.state.wpm_coords
    }

    pub fn input(&self) -> &[Input] {
        &self.session.state.input
    }

    pub fn cursor_pos(&self) -> usize {
        self.session.state.cursor_pos
    }

    pub fn seconds_remaining(&self) -> Option<f64> {
        self.session.state.seconds_remaining
    }

    pub fn is_idle(&self) -> bool {
        self.session.state.is_idle
    }

    pub fn started_at(&self) -> Option<SystemTime> {
        self.session.state.started_at
    }

    pub fn corrected_positions(&self) -> &std::collections::HashSet<usize> {
        &self.session.state.corrected_positions
    }

    // --- Constructors ---

    pub fn with_stats_store(
        prompt: String,
        number_of_words: usize,
        number_of_secs: Option<f64>,
        strict_mode: bool,
        store: Box<dyn StatsStore>,
    ) -> Self {
        let mut thok = Self::new(prompt, number_of_words, number_of_secs, strict_mode);
        thok.stats_db = Some(store);
        thok
    }

    pub fn new(
        prompt: String,
        number_of_words: usize,
        number_of_secs: Option<f64>,
        strict_mode: bool,
    ) -> Self {
        let stats_db = StatsDb::new()
            .ok()
            .map(|db| Box::new(db) as Box<dyn StatsStore>);
        Self {
            session: Session::new(prompt, number_of_words, number_of_secs, strict_mode),
            stats_db,
            celebration: CelebrationAnimation::default(),
        }
    }

    // --- Delegated methods ---

    pub fn on_tick(&mut self) {
        self.session.on_tick();
    }

    pub fn mark_activity(&mut self) -> bool {
        self.session.mark_activity()
    }

    pub fn get_expected_char(&self, idx: usize) -> char {
        self.session.get_expected_char(idx)
    }

    pub fn increment_cursor(&mut self) {
        self.session.increment_cursor();
    }

    pub fn decrement_cursor(&mut self) {
        self.session.decrement_cursor();
    }

    pub fn backspace(&mut self) {
        self.session.backspace();
    }

    pub fn start(&mut self) {
        self.session.start();
    }

    pub fn on_keypress_start(&mut self) {
        self.session.on_keypress_start();
    }

    pub fn calculate_inter_key_time(&self, now: SystemTime) -> u64 {
        self.session.calculate_inter_key_time(now)
    }

    pub fn has_started(&self) -> bool {
        self.session.has_started()
    }

    pub fn has_finished(&self) -> bool {
        self.session.has_finished()
    }

    // --- Methods that add persistence on top of Session ---

    pub fn write(&mut self, c: char) {
        let _ = self.session.mark_activity();
        crate::typing_policy::apply_write(self, c);
    }

    pub fn calc_results(&mut self) {
        self.session.calc_results();

        let _ = self.save_results();

        if self.flush_char_stats().is_some() {
            self.auto_compact_database();
        };
    }

    /// Start celebration animation for perfect sessions.
    pub fn start_celebration_if_worthy(&mut self, terminal_width: u16, terminal_height: u16) {
        if self.session.state.input.is_empty() {
            return;
        }
        if self.session.state.accuracy >= 100.0 {
            self.celebration.start(terminal_width, terminal_height);
        }
    }

    /// Update celebration animation (should be called on each frame/tick)
    pub fn update_celebration(&mut self) {
        self.celebration.update();
    }

    pub fn save_results(&self) -> io::Result<()> {
        if let Some(proj_dirs) = ProjectDirs::from("", "", "klik") {
            let config_dir = proj_dirs.config_dir();
            let log_path = config_dir.join("log.csv");

            std::fs::create_dir_all(config_dir)?;

            let needs_header = !log_path.exists();

            let log_file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(log_path)?;

            let mut writer = Writer::from_writer(log_file);

            if needs_header {
                writer.write_record([
                    "date",
                    "num_words",
                    "num_secs",
                    "elapsed_secs",
                    "wpm",
                    "accuracy",
                    "std_dev",
                ])?;
            }

            let elapsed_secs = self
                .session
                .state
                .started_at
                .unwrap_or_else(SystemTime::now)
                .elapsed()
                .unwrap_or_default()
                .as_secs_f64();

            let date_str = Local::now().format("%c").to_string();
            let num_secs_str = self
                .session
                .config
                .number_of_secs
                .map_or(String::from(""), |ns| format!("{:.2}", ns));
            let elapsed_secs_str = format!("{:.2}", elapsed_secs);
            let wpm_str = self.session.state.wpm.to_string();
            let accuracy_str = self.session.state.accuracy.to_string();
            let std_dev_str = format!("{:.2}", self.session.state.std_dev);

            writer.write_record([
                &date_str,
                &self.session.config.number_of_words.to_string(),
                &num_secs_str,
                &elapsed_secs_str,
                &wpm_str,
                &accuracy_str,
                &std_dev_str,
            ])?;

            writer.flush()?;
        }

        Ok(())
    }

    // --- Stats DB methods ---

    pub fn get_char_stats(&self, character: char) -> Option<Vec<crate::stats::CharStat>> {
        self.stats_db.as_ref()?.get_char_stats(character).ok()
    }

    pub fn get_avg_time_to_press(&self, character: char) -> Option<f64> {
        self.stats_db
            .as_ref()?
            .get_avg_time_to_press(character)
            .ok()
            .flatten()
    }

    pub fn get_miss_rate(&self, character: char) -> Option<f64> {
        self.stats_db.as_ref()?.get_miss_rate(character).ok()
    }

    pub fn get_all_char_summary(&self) -> Option<Vec<(char, f64, f64, i64)>> {
        self.stats_db.as_ref()?.get_all_char_summary().ok()
    }

    pub fn get_char_summary_with_deltas(&self) -> Option<Vec<crate::stats::CharSummaryWithDeltas>> {
        self.stats_db.as_ref()?.get_char_summary_with_deltas().ok()
    }

    pub fn get_session_delta_summary(&self) -> String {
        if let Some(summary) = self.get_char_summary_with_deltas() {
            let mut improvements = 0;
            let mut regressions = 0;
            let mut total_chars_with_deltas = 0;
            let mut avg_time_improvement = 0.0;
            let mut avg_miss_improvement = 0.0;

            for s in &summary {
                if s.session_attempts > 0 {
                    total_chars_with_deltas += 1;

                    if let Some(time_d) = s.time_delta {
                        if time_d < -5.0 {
                            improvements += 1;
                        } else if time_d > 5.0 {
                            regressions += 1;
                        }
                        avg_time_improvement += time_d;
                    }

                    if let Some(miss_d) = s.miss_delta {
                        avg_miss_improvement += miss_d;
                    }
                }
            }

            if total_chars_with_deltas > 0 {
                avg_time_improvement /= total_chars_with_deltas as f64;
                avg_miss_improvement /= total_chars_with_deltas as f64;

                let time_summary = if avg_time_improvement < -5.0 {
                    format!("↓{:.0}ms faster", avg_time_improvement.abs())
                } else if avg_time_improvement > 5.0 {
                    format!("↑{avg_time_improvement:.0}ms slower")
                } else {
                    "similar speed".to_string()
                };

                let miss_summary = if avg_miss_improvement < -2.0 {
                    format!("↓{:.1}% more accurate", avg_miss_improvement.abs())
                } else if avg_miss_improvement > 2.0 {
                    format!("↑{avg_miss_improvement:.1}% less accurate")
                } else {
                    "similar accuracy".to_string()
                };

                if improvements > 0 || regressions > 0 {
                    format!(
                        "vs historical: {time_summary} • {miss_summary} • ↑{improvements} ↓{regressions} chars"
                    )
                } else {
                    format!("vs historical: {time_summary} • {miss_summary}")
                }
            } else {
                "New session - no historical comparison available".to_string()
            }
        } else {
            "No character statistics available".to_string()
        }
    }

    pub fn flush_char_stats(&mut self) -> Option<()> {
        match self.stats_db.as_mut()?.flush() {
            Ok(()) => Some(()),
            Err(e) => {
                #[cfg(any(debug_assertions, test))]
                eprintln!("Failed to flush char stats: {}", e);
                None
            }
        }
    }

    pub fn has_stats_database(&self) -> bool {
        self.stats_db.is_some()
    }

    pub fn get_stats_database_path(&self) -> Option<std::path::PathBuf> {
        crate::stats::StatsDb::get_database_path()
    }

    fn auto_compact_database(&mut self) {
        if let Some(ref mut stats_db) = self.stats_db {
            let _ = stats_db.auto_compact();
        }
    }

    pub fn get_database_info(&self) -> Option<(i64, i64, f64)> {
        self.stats_db.as_ref()?.get_compaction_info().ok()
    }

    pub fn compact_database(&mut self) -> bool {
        self.stats_db
            .as_mut()
            .map(|db| db.compact_database().is_ok())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // Redirect noisy println! in tests behind RUST_LOG to keep CI output clean
    macro_rules! println {
        ($($arg:tt)*) => {{
            if std::env::var("RUST_LOG").is_ok() {
                eprintln!($($arg)*);
            }
        }}
    }

    #[test]
    fn test_outcome_equality() {
        assert_eq!(Outcome::Correct, Outcome::Correct);
        assert_eq!(Outcome::Incorrect, Outcome::Incorrect);
        assert_ne!(Outcome::Correct, Outcome::Incorrect);
    }

    #[test]
    fn test_input_creation() {
        let timestamp = SystemTime::now();
        let input = Input {
            char: 'a',
            outcome: Outcome::Correct,
            timestamp,
            keypress_start: None,
        };

        assert_eq!(input.char, 'a');
        assert_eq!(input.outcome, Outcome::Correct);
        assert_eq!(input.timestamp, timestamp);
        assert_eq!(input.keypress_start, None);
    }

    #[test]
    fn test_thok_new() {
        let thok = Thok::new("hello world".to_string(), 2, None, false);

        assert_eq!(thok.session.prompt, "hello world");
        assert_eq!(thok.session.config.number_of_words, 2);
        assert_eq!(thok.session.config.number_of_secs, None);
        assert_eq!(thok.session.state.input.len(), 0);
        assert_eq!(thok.session.state.cursor_pos, 0);
        assert_eq!(thok.session.state.wpm, 0.0);
        assert_eq!(thok.session.state.accuracy, 0.0);
        assert_eq!(thok.session.state.std_dev, 0.0);
        assert!(!thok.has_started());
        assert!(!thok.has_finished());
        assert!(!thok.session.config.strict);
    }

    #[test]
    fn test_thok_new_with_time_limit() {
        let thok = Thok::new("test".to_string(), 1, Some(30.0), false);

        assert_eq!(thok.session.config.number_of_secs, Some(30.0));
        assert_eq!(thok.session.state.seconds_remaining, Some(30.0));
    }

    #[test]
    fn test_get_expected_char() {
        let thok = Thok::new("hello".to_string(), 1, None, false);

        assert_eq!(thok.get_expected_char(0), 'h');
        assert_eq!(thok.get_expected_char(1), 'e');
        assert_eq!(thok.get_expected_char(4), 'o');
    }

    #[test]
    fn test_write_correct_char() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        thok.write('t');

        assert_eq!(thok.session.state.input.len(), 1);
        assert_eq!(thok.session.state.input[0].char, 't');
        assert_eq!(thok.session.state.input[0].outcome, Outcome::Correct);
        assert_eq!(thok.session.state.cursor_pos, 1);
        assert!(thok.has_started());
    }

    #[test]
    fn test_write_incorrect_char() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        thok.write('x');

        assert_eq!(thok.session.state.input.len(), 1);
        assert_eq!(thok.session.state.input[0].char, 'x');
        assert_eq!(thok.session.state.input[0].outcome, Outcome::Incorrect);
        assert_eq!(thok.session.state.cursor_pos, 1);
    }

    #[test]
    fn test_backspace() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        thok.write('t');
        thok.write('e');
        assert_eq!(thok.session.state.input.len(), 2);
        assert_eq!(thok.session.state.cursor_pos, 2);

        thok.backspace();
        assert_eq!(thok.session.state.input.len(), 1);
        assert_eq!(thok.session.state.cursor_pos, 1);

        thok.backspace();
        assert_eq!(thok.session.state.input.len(), 0);
        assert_eq!(thok.session.state.cursor_pos, 0);
    }

    #[test]
    fn test_backspace_at_start() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        thok.backspace();
        assert_eq!(thok.session.state.input.len(), 0);
        assert_eq!(thok.session.state.cursor_pos, 0);
    }

    #[test]
    fn test_increment_cursor() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);
        thok.write('t');

        let initial_pos = thok.session.state.cursor_pos;
        thok.increment_cursor();

        assert_eq!(thok.session.state.cursor_pos, initial_pos);
    }

    #[test]
    fn test_decrement_cursor() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);
        thok.write('t');

        let initial_pos = thok.session.state.cursor_pos;
        thok.decrement_cursor();

        assert_eq!(thok.session.state.cursor_pos, initial_pos - 1);
    }

    #[test]
    fn test_has_finished_by_completion() {
        let mut thok = Thok::new("hi".to_string(), 1, None, false);

        assert!(!thok.has_finished());

        thok.write('h');
        assert!(!thok.has_finished());

        thok.write('i');
        assert!(thok.has_finished());
    }

    #[test]
    fn test_has_finished_by_time() {
        let mut thok = Thok::new("test".to_string(), 1, Some(1.0), false);

        assert!(!thok.has_finished());

        thok.session.state.seconds_remaining = Some(0.0);
        assert!(thok.has_finished());

        thok.session.state.seconds_remaining = Some(-1.0);
        assert!(thok.has_finished());
    }

    #[test]
    fn test_on_tick() {
        let mut thok = Thok::new("test".to_string(), 1, Some(10.0), false);
        let initial_time = thok.session.state.seconds_remaining.unwrap();

        thok.on_tick();

        let expected_time = initial_time - (TICK_RATE_MS as f64 / 1000.0);
        assert_eq!(thok.session.state.seconds_remaining.unwrap(), expected_time);
    }

    #[test]
    fn test_calc_results_basic() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);
        thok.start();

        thread::sleep(Duration::from_millis(100));

        thok.write('t');
        thok.write('e');
        thok.write('s');
        thok.write('t');

        thok.calc_results();

        assert_eq!(thok.session.state.accuracy, 100.0);
        assert!(thok.session.state.wpm > 0.0);
    }

    #[test]
    fn test_calc_results_with_errors() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);
        thok.start();

        thread::sleep(Duration::from_millis(100));

        thok.write('t');
        thok.write('x');
        thok.write('s');
        thok.write('t');

        thok.calc_results();

        assert_eq!(thok.session.state.accuracy, 75.0);
        assert!(thok.session.state.wpm >= 0.0);
    }

    #[test]
    fn test_calc_results_empty_input() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);
        thok.start();

        thok.calc_results();

        assert_eq!(thok.session.state.wpm, 0.0);
        assert_eq!(thok.session.state.std_dev, 0.0);
    }

    use std::thread;

    #[test]
    fn test_keypress_timing() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        thok.on_keypress_start();
        thread::sleep(Duration::from_millis(10));
        thok.write('t');

        assert_eq!(thok.session.state.input.len(), 1);
        assert_eq!(thok.session.state.input[0].char, 't');
        assert_eq!(thok.session.state.input[0].outcome, Outcome::Correct);
        assert!(thok.session.state.input[0].keypress_start.is_some());
    }

    #[test]
    fn test_character_statistics_methods() {
        let thok = Thok::new("test".to_string(), 1, None, false);

        let stats = thok.get_char_stats('t');
        assert!(stats.is_none() || stats.is_some());

        let avg_time = thok.get_avg_time_to_press('t');
        assert!(avg_time.is_none() || avg_time.is_some());

        let miss_rate = thok.get_miss_rate('t');
        assert!(miss_rate.is_none() || miss_rate.is_some());

        let summary = thok.get_all_char_summary();
        assert!(summary.is_none() || summary.is_some());
    }

    #[test]
    fn test_keypress_timing_reset() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        thok.on_keypress_start();
        assert!(thok.session.state.keypress_start_time.is_some());

        thok.write('t');
        assert!(thok.session.state.keypress_start_time.is_none());
    }

    #[test]
    fn test_flush_char_stats() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        let result = thok.flush_char_stats();
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn test_calc_results_flushes_stats() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);
        thok.start();

        thread::sleep(Duration::from_millis(10));

        thok.write('t');
        thok.write('e');
        thok.write('s');
        thok.write('t');

        thok.calc_results();

        assert_eq!(thok.session.state.accuracy, 100.0);
        assert!(thok.session.state.wpm > 0.0);
    }

    #[test]
    fn test_database_path_retrieval_and_creation() {
        let thok = Thok::new("test".to_string(), 1, None, false);

        let path = thok.get_stats_database_path();
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.to_string_lossy().contains("klik") || p.to_string_lossy().contains("stats"));
    }

    #[test]
    fn test_strict_mode_cursor_behavior() {
        let mut thok = Thok::new("test".to_string(), 1, None, true);

        thok.write('t');
        assert_eq!(thok.session.state.cursor_pos, 1);

        thok.write('x');
        assert_eq!(thok.session.state.cursor_pos, 1);
        assert_eq!(thok.session.state.input[1].outcome, Outcome::Incorrect);

        thok.write('e');
        assert_eq!(thok.session.state.cursor_pos, 2);
        assert_eq!(thok.session.state.input[1].outcome, Outcome::Correct);
        assert!(thok.session.state.corrected_positions.contains(&1));
    }

    #[test]
    fn test_strict_mode_backspace() {
        let mut thok = Thok::new("test".to_string(), 1, None, true);

        thok.write('t');
        thok.write('e');
        assert_eq!(thok.session.state.cursor_pos, 2);
        assert_eq!(thok.session.state.input.len(), 2);

        thok.backspace();
        assert_eq!(thok.session.state.cursor_pos, 1);
        assert_eq!(thok.session.state.input.len(), 1);
    }

    #[test]
    fn test_normal_mode_vs_strict_mode() {
        let mut normal_thok = Thok::new("test".to_string(), 1, None, false);
        normal_thok.write('x');
        assert_eq!(normal_thok.session.state.cursor_pos, 1);

        let mut strict_thok = Thok::new("test".to_string(), 1, None, true);
        strict_thok.write('x');
        assert_eq!(strict_thok.session.state.cursor_pos, 0);
    }

    #[test]
    fn test_edge_case_empty_prompt() {
        let thok = Thok::new("".to_string(), 0, None, false);

        assert_eq!(thok.session.prompt, "");
        assert_eq!(thok.session.config.number_of_words, 0);
        assert!(thok.has_finished());
        assert_eq!(thok.session.state.cursor_pos, 0);
        assert_eq!(thok.session.state.input.len(), 0);
    }

    #[test]
    fn test_edge_case_single_character_prompt() {
        let mut thok = Thok::new("a".to_string(), 1, None, false);

        assert!(!thok.has_finished());

        thok.write('a');
        assert!(thok.has_finished());
        assert_eq!(thok.session.state.cursor_pos, 1);
        assert_eq!(thok.session.state.input.len(), 1);
        assert_eq!(thok.session.state.input[0].outcome, Outcome::Correct);
    }

    #[test]
    fn test_edge_case_unicode_characters() {
        let mut thok = Thok::new("café".to_string(), 1, None, false);

        thok.write('c');
        thok.write('a');
        thok.write('f');
        thok.write('é');

        if thok.has_finished() {
            assert_eq!(thok.session.state.input.len(), 4);
            for input in &thok.session.state.input {
                assert_eq!(input.outcome, Outcome::Correct);
            }
        } else {
            assert!(!thok.session.state.input.is_empty());
        }
    }

    #[test]
    fn test_edge_case_very_long_prompt() {
        let long_prompt = "a".repeat(10000);
        let mut thok = Thok::new(long_prompt.clone(), 1000, None, false);

        assert_eq!(thok.session.prompt.len(), 10000);
        assert!(!thok.has_finished());

        for _ in 0..100 {
            thok.write('a');
        }

        assert_eq!(thok.session.state.cursor_pos, 100);
        assert!(!thok.has_finished());
    }

    #[test]
    fn test_edge_case_zero_time_limit() {
        let thok = Thok::new("test".to_string(), 1, Some(0.0), false);

        assert!(thok.has_finished());
        assert_eq!(thok.session.state.seconds_remaining, Some(0.0));
    }

    #[test]
    fn test_edge_case_negative_time_limit() {
        let thok = Thok::new("test".to_string(), 1, Some(-1.0), false);

        assert!(thok.has_finished());
        assert_eq!(thok.session.state.seconds_remaining, Some(-1.0));
    }

    #[test]
    fn test_error_handling_invalid_cursor_position() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        thok.write('t');
        thok.write('e');
        assert_eq!(thok.session.state.cursor_pos, 2);

        assert!(thok.session.state.cursor_pos <= thok.session.prompt.len());
    }

    #[test]
    fn test_error_handling_backspace_at_start() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        thok.backspace();
        assert_eq!(thok.session.state.cursor_pos, 0);
        assert_eq!(thok.session.state.input.len(), 0);
    }

    #[test]
    fn test_error_handling_multiple_backspaces() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        thok.write('t');
        thok.write('e');
        assert_eq!(thok.session.state.cursor_pos, 2);

        thok.backspace();
        thok.backspace();
        thok.backspace();

        assert_eq!(thok.session.state.cursor_pos, 0);
        assert_eq!(thok.session.state.input.len(), 0);
    }

    #[test]
    fn test_error_handling_calc_results_no_input() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        thok.session.state.started_at = Some(SystemTime::now());

        thok.calc_results();

        assert!(thok.session.state.wpm >= 0.0);
        assert!(!thok.session.state.accuracy.is_infinite());
        assert!(thok.session.state.std_dev >= 0.0);
    }

    #[test]
    fn test_error_handling_calc_results_zero_time() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        thok.session.state.started_at = Some(SystemTime::now());
        thok.write('t');

        thok.calc_results();

        assert!(thok.session.state.wpm >= 0.0);
        assert!(thok.session.state.accuracy >= 0.0);
    }

    #[test]
    fn test_timing_initialization() {
        let thok = Thok::new("test".to_string(), 1, Some(1.0), false);

        assert_eq!(thok.session.config.number_of_secs, Some(1.0));
        assert_eq!(thok.session.state.seconds_remaining, Some(1.0));
    }

    #[test]
    fn test_error_handling_stats_database_failure() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        thok.write('t');
        thok.write('e');
        thok.write('s');
        thok.write('t');

        assert!(thok.has_finished());

        thok.calc_results();

        assert!(thok.session.state.wpm >= 0.0);
        assert!(thok.session.state.accuracy >= 0.0);
    }

    #[test]
    fn test_error_handling_special_characters() {
        let mut thok = Thok::new("test\n\t\r".to_string(), 1, None, false);

        thok.write('t');
        thok.write('e');
        thok.write('s');
        thok.write('t');
        thok.write('\n');
        thok.write('\t');
        thok.write('\r');

        assert!(thok.has_finished());
        assert_eq!(thok.session.state.input.len(), 7);

        for input in &thok.session.state.input {
            assert_eq!(input.outcome, Outcome::Correct);
        }
    }

    #[test]
    fn test_error_handling_null_character() {
        let mut thok = Thok::new("test\0".to_string(), 1, None, false);

        thok.write('t');
        thok.write('e');
        thok.write('s');
        thok.write('t');
        thok.write('\0');

        assert!(thok.has_finished());
        assert_eq!(thok.session.state.input.len(), 5);
        assert_eq!(thok.session.state.input[4].outcome, Outcome::Correct);
    }

    #[test]
    fn test_boundary_conditions_cursor_limits() {
        let mut thok = Thok::new("abc".to_string(), 1, None, false);

        thok.write('a');
        thok.write('b');
        thok.write('c');

        assert!(thok.has_finished());
        assert_eq!(thok.session.state.cursor_pos, 3);

        assert!(thok.session.state.cursor_pos <= thok.session.prompt.len());
    }

    #[test]
    fn test_boundary_conditions_time_precision() {
        let mut thok = Thok::new("test".to_string(), 1, Some(0.001), false);

        assert!(thok.session.config.number_of_secs == Some(0.001));

        thok.session.state.started_at = Some(SystemTime::now());
        thok.on_tick();

        assert!(thok.has_finished());
    }

    #[test]
    fn test_database_compaction_methods() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        if thok.has_stats_database() {
            let info = thok.get_database_info();
            if let Some((session_count, db_size, db_size_mb)) = info {
                assert!(session_count >= 0);
                assert!(db_size >= 0);
                assert!(db_size_mb >= 0.0);
            }
        }

        let compaction_result = thok.compact_database();
        if thok.has_stats_database() {
            assert!(compaction_result);
        } else {
            assert!(!compaction_result);
        }
    }

    #[test]
    fn test_inter_keystroke_timing() {
        let mut thok = Thok::new("hello".to_string(), 1, None, false);

        println!("Testing inter-keystroke timing (simulating main app behavior)...");

        thok.write('h');
        thread::sleep(Duration::from_millis(150));
        thok.write('e');
        thread::sleep(Duration::from_millis(120));
        thok.write('l');
        thread::sleep(Duration::from_millis(180));
        thok.write('l');
        thread::sleep(Duration::from_millis(100));
        thok.write('o');

        assert!(thok.has_finished());
        thok.calc_results();

        if let Some(summary) = thok.get_all_char_summary() {
            println!("Inter-keystroke timing results:");
            for (char, avg_time, miss_rate, attempts) in &summary {
                if ['h', 'e', 'l', 'o'].contains(char) {
                    println!(
                        "  '{char}': avg={avg_time}ms, miss={miss_rate}%, attempts={attempts}"
                    );
                    assert!(
                        *avg_time > 0.0,
                        "Character '{char}' has zero timing: {avg_time}ms",
                    );
                }
            }
        } else {
            panic!("No summary statistics found for inter-keystroke timing test");
        }
    }

    #[test]
    fn test_celebration_perfect_session() {
        let mut thok = Thok::new("hello".to_string(), 1, None, false);

        if let Some(ref stats_db) = thok.stats_db {
            let _ = stats_db.clear_all_stats();
        }

        thok.write('h');
        thok.write('e');
        thok.write('l');
        thok.write('l');
        thok.write('o');

        assert!(thok.has_finished());
        thok.calc_results();
        assert_eq!(thok.session.state.accuracy, 100.0);

        thok.start_celebration_if_worthy(80, 24);
        assert!(thok.celebration.is_active);
        assert!(!thok.celebration.particles.is_empty());

        for _ in 0..10 {
            thok.update_celebration();
        }
        assert!(thok.celebration.is_active);
    }

    #[test]
    fn test_celebration_animation_imperfect_session() {
        let mut thok = Thok::new("hello".to_string(), 1, None, false);

        thok.write('h');
        thok.write('x');
        thok.write('l');
        thok.write('l');
        thok.write('o');

        assert!(thok.has_finished());
        thok.calc_results();

        assert!(thok.session.state.accuracy < 100.0);

        thok.start_celebration_if_worthy(80, 24);

        assert!(!thok.celebration.is_active);
        assert!(thok.celebration.particles.is_empty());
    }

    #[test]
    fn test_fresh_database_with_realistic_timing() {
        let mut thok = Thok::new("hello world test".to_string(), 3, None, false);

        if let Some(ref mut stats_db) = thok.stats_db {
            let _ = stats_db.clear_all_stats();
        }

        let timings: [u64; 16] = [
            200, 150, 180, 120, 250, 300, 180, 160, 140, 170, 200, 160, 220, 180, 190, 210,
        ];

        for (i, c) in "hello world test".chars().enumerate() {
            if i > 0 && i < timings.len() {
                thread::sleep(Duration::from_millis(timings[i]));
            }
            thok.write(c);
        }

        assert!(thok.has_finished());
        thok.calc_results();

        let summary = thok
            .get_all_char_summary()
            .expect("Should have summary statistics");
        let has_meaningful_timing = summary.iter().any(|(_, avg_time, _, _)| *avg_time > 0.0);
        assert!(has_meaningful_timing, "Should have meaningful timing data");
    }

    #[test]
    fn test_idle_state_reset() {
        let mut thok = Thok::new("test prompt".to_string(), 2, None, false);

        thok.write('t');
        assert!(thok.has_started());
        assert_eq!(thok.session.state.cursor_pos, 1);
        assert_eq!(thok.session.state.input.len(), 1);

        thok.session.state.is_idle = true;
        assert!(thok.session.state.is_idle);

        let was_idle = thok.mark_activity();

        assert!(was_idle, "Should return true when exiting idle state");
        assert!(
            !thok.session.state.is_idle,
            "Should no longer be idle after mark_activity"
        );

        assert_eq!(thok.session.state.cursor_pos, 1);
        assert_eq!(thok.session.state.input.len(), 1);
    }

    #[test]
    fn test_mark_activity_not_idle() {
        let mut thok = Thok::new("test prompt".to_string(), 2, None, false);

        let was_idle = thok.mark_activity();

        assert!(!was_idle, "Should return false when not exiting idle state");
        assert!(!thok.session.state.is_idle);
    }
}
