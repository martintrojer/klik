#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub number_of_words: usize,
    pub number_of_secs: Option<f64>,
    pub strict: bool,
}

#[derive(Debug, Clone)]
pub struct SessionResult {
    pub wpm: f64,
    pub accuracy: f64,
    pub std_dev: f64,
    pub wpm_coords: Vec<crate::time_series::TimeSeriesPoint>,
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub started_at: Option<std::time::SystemTime>,
    pub seconds_remaining: Option<f64>,
    pub last_activity: Option<std::time::SystemTime>,
    pub is_idle: bool,
    pub idle_timeout_secs: f64,
    pub keypress_start_time: Option<std::time::SystemTime>,
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
        }
    }
}
