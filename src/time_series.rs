#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeSeriesPoint {
    pub t: f64,
    pub wpm: f64,
}

impl TimeSeriesPoint {
    pub fn new(t: f64, wpm: f64) -> Self {
        Self { t, wpm }
    }
}

impl From<(f64, f64)> for TimeSeriesPoint {
    fn from(v: (f64, f64)) -> Self {
        TimeSeriesPoint { t: v.0, wpm: v.1 }
    }
}

impl From<TimeSeriesPoint> for (f64, f64) {
    fn from(p: TimeSeriesPoint) -> Self {
        (p.t, p.wpm)
    }
}
