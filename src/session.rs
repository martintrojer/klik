#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub number_of_words: usize,
    pub number_of_secs: Option<f64>,
    pub strict: bool,
}

// SessionResult was part of planned split; removed until needed

#[derive(Debug, Clone)]
pub struct SessionState {
    pub started_at: Option<std::time::SystemTime>,
    pub seconds_remaining: Option<f64>,
    pub last_activity: Option<std::time::SystemTime>,
    pub is_idle: bool,
    pub idle_timeout_secs: f64,
    pub keypress_start_time: Option<std::time::SystemTime>,
    // Typing state
    pub cursor_pos: usize,
    pub input: Vec<crate::thok::Input>,
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
