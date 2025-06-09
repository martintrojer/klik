/// Character difficulty metrics for intelligent word selection
#[derive(Debug, Clone)]
pub struct CharacterDifficulty {
    pub miss_rate: f64,      // Percentage of incorrect attempts (0-100) for any case
    pub avg_time_ms: f64,    // Average time to type the character (any case)
    pub total_attempts: i64, // Total number of attempts for weighting
    // Uppercase-specific difficulty metrics
    pub uppercase_miss_rate: f64, // Percentage of incorrect uppercase attempts (0-100)
    pub uppercase_avg_time: f64,  // Average time for uppercase variants
    pub uppercase_attempts: i64,  // Total uppercase attempts for weighting
    pub uppercase_penalty: f64,   // Additional difficulty penalty for uppercase (0-1)
}
