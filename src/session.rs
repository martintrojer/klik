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
    pub wpm_coords: Vec<(f64, f64)>,
}
