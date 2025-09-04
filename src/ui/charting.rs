/// Compute X (seconds) and Y (WPM) bounds for the results chart
pub fn compute_chart_params(
    wpm_coords: &[(f64, f64)],
    seconds_remaining: Option<f64>,
) -> (f64, f64) {
    let mut highest_wpm = 0.0;
    for &(_, wpm) in wpm_coords {
        if wpm > highest_wpm {
            highest_wpm = wpm;
        }
    }

    let mut overall_duration = match wpm_coords.last() {
        Some(x) => x.0,
        None => seconds_remaining.unwrap_or(1.0),
    };
    if overall_duration < 1.0 {
        overall_duration = 1.0;
    }

    (overall_duration, highest_wpm.round())
}

/// Format a simple numeric label consistently
pub fn format_label(val: f64) -> String {
    if (val - val.round()).abs() < f64::EPSILON {
        format!("{}", val.round())
    } else {
        format!("{val:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_chart_params_empty() {
        let (x, y) = compute_chart_params(&[], Some(5.0));
        assert_eq!(x, 5.0);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn test_format_label() {
        assert_eq!(format_label(1.0), "1");
        assert_eq!(format_label(1.2345), "1.23");
    }
}
