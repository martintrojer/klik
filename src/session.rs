use crate::stats::time_diff_ms;
use crate::thok::{Input, Outcome, TICK_RATE_MS};
use crate::util::std_dev;
use itertools::Itertools;
use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub number_of_words: usize,
    pub number_of_secs: Option<f64>,
    pub strict: bool,
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub started_at: Option<SystemTime>,
    pub seconds_remaining: Option<f64>,
    pub last_activity: Option<SystemTime>,
    pub is_idle: bool,
    pub idle_timeout_secs: f64,
    pub keypress_start_time: Option<SystemTime>,
    // Typing state
    pub cursor_pos: usize,
    pub input: Vec<Input>,
    pub corrected_positions: std::collections::HashSet<usize>,
    // Results
    pub wpm: f64,
    pub accuracy: f64,
    pub std_dev: f64,
    pub wpm_coords: Vec<crate::time_series::TimeSeriesPoint>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            started_at: None,
            seconds_remaining: None,
            last_activity: None,
            is_idle: false,
            idle_timeout_secs: 30.0,
            keypress_start_time: None,
            cursor_pos: 0,
            input: Vec::new(),
            corrected_positions: std::collections::HashSet::new(),
            wpm: 0.0,
            accuracy: 0.0,
            std_dev: 0.0,
            wpm_coords: Vec::new(),
        }
    }
}

/// A typing session: prompt text + configuration + mutable state.
/// Contains pure typing logic with no persistence or animation concerns.
#[derive(Debug)]
pub struct Session {
    pub prompt: String,
    pub config: SessionConfig,
    pub state: SessionState,
}

impl Session {
    pub fn new(
        prompt: String,
        number_of_words: usize,
        number_of_secs: Option<f64>,
        strict_mode: bool,
    ) -> Self {
        Self {
            prompt,
            config: SessionConfig {
                number_of_words,
                number_of_secs,
                strict: strict_mode,
            },
            state: SessionState {
                seconds_remaining: number_of_secs,
                ..Default::default()
            },
        }
    }

    pub fn on_tick(&mut self) {
        if let Some(remaining) = self.state.seconds_remaining {
            let next = remaining - (TICK_RATE_MS as f64 / 1000_f64);
            self.state.seconds_remaining = Some(next.max(0.0));
        }
        self.check_idle_timeout();
    }

    fn check_idle_timeout(&mut self) {
        if let Some(last_activity) = self.state.last_activity {
            let now = SystemTime::now();
            if let Ok(duration) = now.duration_since(last_activity) {
                let idle_duration = duration.as_secs_f64();
                if idle_duration >= self.state.idle_timeout_secs && !self.state.is_idle {
                    self.state.is_idle = true;
                    if self.has_started() && !self.has_finished() {
                        if let Some(started_at) = self.state.started_at {
                            if let Ok(elapsed) = last_activity.duration_since(started_at) {
                                let adj = Some(now.checked_sub(elapsed).unwrap_or(now));
                                self.state.started_at = adj;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Mark activity and exit idle state if necessary.
    /// Returns true if we were exiting idle state.
    pub fn mark_activity(&mut self) -> bool {
        let now = SystemTime::now();
        let was_idle = self.state.is_idle;

        if self.state.is_idle {
            self.state.is_idle = false;
            if self.has_started() && !self.has_finished() {
                if let Some(started_at) = self.state.started_at {
                    if let Some(last_activity) = self.state.last_activity {
                        if let Ok(elapsed_before_idle) = last_activity.duration_since(started_at) {
                            self.state.started_at =
                                Some(now.checked_sub(elapsed_before_idle).unwrap_or(started_at));
                        }
                    }
                }
                self.state.seconds_remaining = self.config.number_of_secs;
            }
        }

        self.state.last_activity = Some(now);
        was_idle
    }

    pub fn get_expected_char(&self, idx: usize) -> char {
        self.prompt.chars().nth(idx).unwrap_or(' ')
    }

    pub fn increment_cursor(&mut self) {
        if self.state.cursor_pos < self.state.input.len() {
            self.state.cursor_pos += 1;
        }
    }

    pub fn decrement_cursor(&mut self) {
        if self.state.cursor_pos > 0 {
            self.state.cursor_pos -= 1;
        }
    }

    pub fn backspace(&mut self) {
        let _ = self.mark_activity();

        if self.config.strict {
            if self.state.cursor_pos > 0 {
                self.decrement_cursor();
                if self.state.cursor_pos < self.state.input.len() {
                    self.state.input.remove(self.state.cursor_pos);
                }
            }
        } else if self.state.cursor_pos > 0 {
            self.state.input.remove(self.state.cursor_pos - 1);
            self.decrement_cursor();
        }
    }

    pub fn start(&mut self) {
        self.state.started_at = Some(SystemTime::now());
    }

    pub fn on_keypress_start(&mut self) {
        self.state.keypress_start_time = Some(SystemTime::now());
    }

    pub fn calculate_inter_key_time(&self, now: SystemTime) -> u64 {
        if let Some(last_input) = self.state.input.last() {
            time_diff_ms(last_input.timestamp, now)
        } else {
            0
        }
    }

    pub fn has_started(&self) -> bool {
        self.state.started_at.is_some()
    }

    pub fn has_finished(&self) -> bool {
        let prompt_chars = self.prompt.chars().count();
        (self.state.input.len() == prompt_chars)
            || (self.state.seconds_remaining.is_some()
                && self.state.seconds_remaining.unwrap() <= 0.0)
    }

    /// Calculate WPM, accuracy, and standard deviation from the current input.
    pub fn calc_results(&mut self) {
        let correct_chars: Vec<&Input> = self
            .state
            .input
            .iter()
            .filter(|i| i.outcome == Outcome::Correct)
            .collect();

        let started_at = self.state.started_at.unwrap_or_else(SystemTime::now);
        let elapsed_secs = started_at.elapsed().unwrap_or_default().as_millis() as f64;
        let whole_second_limit = elapsed_secs.floor();

        let mut char_counts: HashMap<String, u32> = HashMap::new();
        for input in &correct_chars {
            let mut num_secs = input
                .timestamp
                .duration_since(started_at)
                .unwrap_or_default()
                .as_secs_f64();

            if num_secs == 0.0 {
                num_secs = 1.0;
            } else if num_secs.ceil() <= whole_second_limit {
                if num_secs > 0.0 && num_secs < 1.0 {
                    num_secs = 1.0;
                } else {
                    num_secs = num_secs.ceil();
                }
            } else {
                num_secs = elapsed_secs;
            }

            *char_counts.entry(num_secs.to_string()).or_insert(0) += 1;
        }

        let correct_chars_per_sec: Vec<(f64, f64)> = char_counts
            .into_iter()
            .map(|(k, v)| (k.parse::<f64>().unwrap(), v as f64))
            .sorted_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .collect();

        let correct_chars_at_whole_sec_intervals: Vec<f64> = correct_chars_per_sec
            .iter()
            .take(correct_chars_per_sec.len().saturating_sub(1))
            .map(|(_, count)| *count)
            .collect();

        self.state.std_dev = std_dev(&correct_chars_at_whole_sec_intervals).unwrap_or(0.0);

        let mut correct_chars_pressed_until_now = 0.0;

        self.state.wpm_coords.clear();
        for x in correct_chars_per_sec {
            correct_chars_pressed_until_now += x.1;
            self.state
                .wpm_coords
                .push(crate::time_series::TimeSeriesPoint::new(
                    x.0,
                    ((60.00 / x.0) * correct_chars_pressed_until_now) / 5.0,
                ))
        }

        if let Some(last) = self.state.wpm_coords.last() {
            self.state.wpm = last.wpm.ceil();
        } else {
            self.state.wpm = 0.0;
        }
        self.state.accuracy = if self.state.input.is_empty() {
            0.0
        } else {
            ((correct_chars.len() as f64 / self.state.input.len() as f64) * 100.0).round()
        };
    }
}
