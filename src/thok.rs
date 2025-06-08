use crate::stats::{extract_context, time_diff_ms, CharStat, StatsDb};
use crate::util::std_dev;
use crate::TICK_RATE_MS;
use chrono::prelude::*;
use directories::ProjectDirs;
use itertools::Itertools;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::{char, collections::HashMap, time::SystemTime};

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

/// represents a test being displayed to the user
#[derive(Debug)]
pub struct Thok {
    pub prompt: String,
    pub input: Vec<Input>,
    pub raw_coords: Vec<(f64, f64)>,
    pub wpm_coords: Vec<(f64, f64)>,
    pub cursor_pos: usize,
    pub started_at: Option<SystemTime>,
    pub seconds_remaining: Option<f64>,
    pub number_of_secs: Option<f64>,
    pub number_of_words: usize,
    pub wpm: f64,
    pub accuracy: f64,
    pub std_dev: f64,
    pub stats_db: Option<StatsDb>,
    pub keypress_start_time: Option<SystemTime>,
}

impl Thok {
    pub fn new(prompt: String, number_of_words: usize, number_of_secs: Option<f64>) -> Self {
        let stats_db = StatsDb::new().ok();
        Self {
            prompt,
            input: vec![],
            raw_coords: vec![],
            wpm_coords: vec![],
            cursor_pos: 0,
            started_at: None,
            number_of_secs,
            number_of_words,
            seconds_remaining: number_of_secs,
            wpm: 0.0,
            accuracy: 0.0,
            std_dev: 0.0,
            stats_db,
            keypress_start_time: None,
        }
    }

    pub fn on_tick(&mut self) {
        self.seconds_remaining =
            Some(self.seconds_remaining.unwrap() - (TICK_RATE_MS as f64 / 1000_f64));
    }

    pub fn get_expected_char(&self, idx: usize) -> char {
        self.prompt.chars().nth(idx).unwrap()
    }

    pub fn increment_cursor(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += 1;
        }
    }

    pub fn decrement_cursor(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn calc_results(&mut self) {
        let correct_chars = self
            .input
            .clone()
            .into_iter()
            .filter(|i| i.outcome == Outcome::Correct)
            .collect::<Vec<Input>>();

        let elapsed_secs = self.started_at.unwrap().elapsed().unwrap().as_millis() as f64;

        let whole_second_limit = elapsed_secs.floor();

        let correct_chars_per_sec: Vec<(f64, f64)> = correct_chars
            .clone()
            .into_iter()
            .fold(HashMap::new(), |mut map, i| {
                let mut num_secs = i
                    .timestamp
                    .duration_since(self.started_at.unwrap())
                    .unwrap()
                    .as_secs_f64();

                if num_secs == 0.0 {
                    num_secs = 1.;
                } else if num_secs.ceil() <= whole_second_limit {
                    if num_secs > 0. && num_secs < 1. {
                        // this accounts for the initiated keypress at 0.000
                        num_secs = 1.;
                    } else {
                        num_secs = num_secs.ceil()
                    }
                } else {
                    num_secs = elapsed_secs;
                }

                *map.entry(num_secs.to_string()).or_insert(0) += 1;
                map
            })
            .into_iter()
            .map(|(k, v)| (k.parse::<f64>().unwrap(), v as f64))
            .sorted_by(|a, b| a.partial_cmp(b).unwrap())
            .collect();

        let correct_chars_at_whole_sec_intervals = correct_chars_per_sec
            .iter()
            .enumerate()
            .filter(|&(i, _)| i < correct_chars_per_sec.len() - 1)
            .map(|(_, x)| x.1)
            .collect::<Vec<f64>>();

        if !correct_chars_at_whole_sec_intervals.is_empty() {
            self.std_dev = std_dev(&correct_chars_at_whole_sec_intervals).unwrap();
        } else {
            self.std_dev = 0.0;
        }

        let mut correct_chars_pressed_until_now = 0.0;

        for x in correct_chars_per_sec {
            correct_chars_pressed_until_now += x.1;
            self.wpm_coords
                .push((x.0, ((60.00 / x.0) * correct_chars_pressed_until_now) / 5.0))
        }

        if !self.wpm_coords.is_empty() {
            self.wpm = self.wpm_coords.last().unwrap().1.ceil();
        } else {
            self.wpm = 0.0;
        }
        self.accuracy = ((correct_chars.len() as f64 / self.input.len() as f64) * 100.0).round();

        let _ = self.save_results();
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            self.input.remove(self.cursor_pos - 1);
            self.decrement_cursor();
        }
    }

    pub fn start(&mut self) {
        self.started_at = Some(SystemTime::now());
    }

    pub fn on_keypress_start(&mut self) {
        self.keypress_start_time = Some(SystemTime::now());
    }

    pub fn write(&mut self, c: char) {
        let idx = self.input.len();
        if idx == 0 && self.started_at.is_none() {
            self.start();
        }

        let now = SystemTime::now();
        let expected_char = self.get_expected_char(idx);
        let outcome = if c == expected_char {
            Outcome::Correct
        } else {
            Outcome::Incorrect
        };

        // Calculate time to press if we have a start time
        let time_to_press_ms = if let Some(start_time) = self.keypress_start_time {
            time_diff_ms(start_time, now)
        } else {
            0
        };

        // Record character statistics if database is available
        if let Some(ref stats_db) = self.stats_db {
            let (context_before, context_after) = extract_context(&self.prompt, idx, 3);
            
            let char_stat = CharStat {
                character: expected_char,
                time_to_press_ms,
                was_correct: outcome == Outcome::Correct,
                timestamp: Local::now(),
                context_before,
                context_after,
            };

            let _ = stats_db.record_char_stat(&char_stat);
        }

        self.input.insert(
            self.cursor_pos,
            Input {
                char: c,
                outcome,
                timestamp: now,
                keypress_start: self.keypress_start_time,
            },
        );
        
        // Reset keypress start time for next character
        self.keypress_start_time = None;
        self.increment_cursor();
    }

    pub fn has_started(&self) -> bool {
        self.started_at.is_some()
    }

    pub fn has_finished(&self) -> bool {
        (self.input.len() == self.prompt.len())
            || (self.seconds_remaining.is_some() && self.seconds_remaining.unwrap() <= 0.0)
    }

    pub fn save_results(&self) -> io::Result<()> {
        if let Some(proj_dirs) = ProjectDirs::from("", "", "thokr") {
            let config_dir = proj_dirs.config_dir();
            let log_path = config_dir.join("log.csv");

            std::fs::create_dir_all(config_dir)?;

            // If the config file doesn't exist, we need to emit a header
            let needs_header = !log_path.exists();

            let mut log_file = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(log_path)?;

            if needs_header {
                writeln!(
                    log_file,
                    "date,num_words,num_secs,elapsed_secs,wpm,accuracy,std_dev"
                )?;
            }

            let elapsed_secs = self.started_at.unwrap().elapsed().unwrap().as_secs_f64();

            writeln!(
                log_file,
                "{},{},{},{:.2},{},{},{:.2}",
                Local::now().format("%c"),
                self.number_of_words,
                self.number_of_secs
                    .map_or(String::from(""), |ns| format!("{:.2}", ns)),
                elapsed_secs,
                self.wpm,      // already rounded, no need to round to two decimal places
                self.accuracy, // already rounded, no need to round to two decimal places
                self.std_dev,
            )?;
        }

        Ok(())
    }

    /// Get character statistics for analysis
    pub fn get_char_stats(&self, character: char) -> Option<Vec<crate::stats::CharStat>> {
        if let Some(ref stats_db) = self.stats_db {
            stats_db.get_char_stats(character).ok()
        } else {
            None
        }
    }

    /// Get average time to press for a character
    pub fn get_avg_time_to_press(&self, character: char) -> Option<f64> {
        if let Some(ref stats_db) = self.stats_db {
            stats_db.get_avg_time_to_press(character).ok().flatten()
        } else {
            None
        }
    }

    /// Get miss rate for a character
    pub fn get_miss_rate(&self, character: char) -> Option<f64> {
        if let Some(ref stats_db) = self.stats_db {
            stats_db.get_miss_rate(character).ok()
        } else {
            None
        }
    }

    /// Get summary of all character statistics
    pub fn get_all_char_summary(&self) -> Option<Vec<(char, f64, f64, i64)>> {
        if let Some(ref stats_db) = self.stats_db {
            stats_db.get_all_char_summary().ok()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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
        let thok = Thok::new("hello world".to_string(), 2, None);

        assert_eq!(thok.prompt, "hello world");
        assert_eq!(thok.number_of_words, 2);
        assert_eq!(thok.number_of_secs, None);
        assert_eq!(thok.input.len(), 0);
        assert_eq!(thok.cursor_pos, 0);
        assert_eq!(thok.wpm, 0.0);
        assert_eq!(thok.accuracy, 0.0);
        assert_eq!(thok.std_dev, 0.0);
        assert!(!thok.has_started());
        assert!(!thok.has_finished());
    }

    #[test]
    fn test_thok_new_with_time_limit() {
        let thok = Thok::new("test".to_string(), 1, Some(30.0));

        assert_eq!(thok.number_of_secs, Some(30.0));
        assert_eq!(thok.seconds_remaining, Some(30.0));
    }

    #[test]
    fn test_get_expected_char() {
        let thok = Thok::new("hello".to_string(), 1, None);

        assert_eq!(thok.get_expected_char(0), 'h');
        assert_eq!(thok.get_expected_char(1), 'e');
        assert_eq!(thok.get_expected_char(4), 'o');
    }

    #[test]
    fn test_write_correct_char() {
        let mut thok = Thok::new("test".to_string(), 1, None);

        thok.write('t');

        assert_eq!(thok.input.len(), 1);
        assert_eq!(thok.input[0].char, 't');
        assert_eq!(thok.input[0].outcome, Outcome::Correct);
        assert_eq!(thok.cursor_pos, 1);
        assert!(thok.has_started());
    }

    #[test]
    fn test_write_incorrect_char() {
        let mut thok = Thok::new("test".to_string(), 1, None);

        thok.write('x');

        assert_eq!(thok.input.len(), 1);
        assert_eq!(thok.input[0].char, 'x');
        assert_eq!(thok.input[0].outcome, Outcome::Incorrect);
        assert_eq!(thok.cursor_pos, 1);
    }

    #[test]
    fn test_backspace() {
        let mut thok = Thok::new("test".to_string(), 1, None);

        thok.write('t');
        thok.write('e');
        assert_eq!(thok.input.len(), 2);
        assert_eq!(thok.cursor_pos, 2);

        thok.backspace();
        assert_eq!(thok.input.len(), 1);
        assert_eq!(thok.cursor_pos, 1);

        thok.backspace();
        assert_eq!(thok.input.len(), 0);
        assert_eq!(thok.cursor_pos, 0);
    }

    #[test]
    fn test_backspace_at_start() {
        let mut thok = Thok::new("test".to_string(), 1, None);

        thok.backspace();
        assert_eq!(thok.input.len(), 0);
        assert_eq!(thok.cursor_pos, 0);
    }

    #[test]
    fn test_increment_cursor() {
        let mut thok = Thok::new("test".to_string(), 1, None);
        thok.write('t');

        let initial_pos = thok.cursor_pos;
        thok.increment_cursor();

        assert_eq!(thok.cursor_pos, initial_pos);
    }

    #[test]
    fn test_decrement_cursor() {
        let mut thok = Thok::new("test".to_string(), 1, None);
        thok.write('t');

        let initial_pos = thok.cursor_pos;
        thok.decrement_cursor();

        assert_eq!(thok.cursor_pos, initial_pos - 1);
    }

    #[test]
    fn test_has_finished_by_completion() {
        let mut thok = Thok::new("hi".to_string(), 1, None);

        assert!(!thok.has_finished());

        thok.write('h');
        assert!(!thok.has_finished());

        thok.write('i');
        assert!(thok.has_finished());
    }

    #[test]
    fn test_has_finished_by_time() {
        let mut thok = Thok::new("test".to_string(), 1, Some(1.0));

        assert!(!thok.has_finished());

        thok.seconds_remaining = Some(0.0);
        assert!(thok.has_finished());

        thok.seconds_remaining = Some(-1.0);
        assert!(thok.has_finished());
    }

    #[test]
    fn test_on_tick() {
        let mut thok = Thok::new("test".to_string(), 1, Some(10.0));
        let initial_time = thok.seconds_remaining.unwrap();

        thok.on_tick();

        let expected_time = initial_time - (TICK_RATE_MS as f64 / 1000.0);
        assert_eq!(thok.seconds_remaining.unwrap(), expected_time);
    }

    #[test]
    fn test_calc_results_basic() {
        let mut thok = Thok::new("test".to_string(), 1, None);
        thok.start();

        thread::sleep(Duration::from_millis(100));

        thok.write('t');
        thok.write('e');
        thok.write('s');
        thok.write('t');

        thok.calc_results();

        assert_eq!(thok.accuracy, 100.0);
        assert!(thok.wpm > 0.0);
    }

    #[test]
    fn test_calc_results_with_errors() {
        let mut thok = Thok::new("test".to_string(), 1, None);
        thok.start();

        thread::sleep(Duration::from_millis(100));

        thok.write('t');
        thok.write('x');
        thok.write('s');
        thok.write('t');

        thok.calc_results();

        assert_eq!(thok.accuracy, 75.0);
        assert!(thok.wpm >= 0.0);
    }

    #[test]
    fn test_calc_results_empty_input() {
        let mut thok = Thok::new("test".to_string(), 1, None);
        thok.start();

        thok.calc_results();

        assert_eq!(thok.wpm, 0.0);
        assert_eq!(thok.std_dev, 0.0);
    }

    use std::thread;

    #[test]
    fn test_keypress_timing() {
        let mut thok = Thok::new("test".to_string(), 1, None);
        
        thok.on_keypress_start();
        thread::sleep(Duration::from_millis(10));
        thok.write('t');
        
        assert_eq!(thok.input.len(), 1);
        assert_eq!(thok.input[0].char, 't');
        assert_eq!(thok.input[0].outcome, Outcome::Correct);
        assert!(thok.input[0].keypress_start.is_some());
    }

    #[test]
    fn test_character_statistics_methods() {
        let thok = Thok::new("test".to_string(), 1, None);
        
        // These methods should return None if no database is available
        assert!(thok.get_char_stats('t').is_none() || thok.get_char_stats('t').is_some());
        assert!(thok.get_avg_time_to_press('t').is_none() || thok.get_avg_time_to_press('t').is_some());
        assert!(thok.get_miss_rate('t').is_none() || thok.get_miss_rate('t').is_some());
        assert!(thok.get_all_char_summary().is_none() || thok.get_all_char_summary().is_some());
    }

    #[test]
    fn test_keypress_timing_reset() {
        let mut thok = Thok::new("test".to_string(), 1, None);
        
        thok.on_keypress_start();
        assert!(thok.keypress_start_time.is_some());
        
        thok.write('t');
        assert!(thok.keypress_start_time.is_none()); // Should be reset after write
    }
}
