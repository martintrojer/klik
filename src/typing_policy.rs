use crate::stats::{extract_context, CharStat};
use crate::thok::{Input, Outcome, Thok};
use chrono::Local;
use std::time::SystemTime;

fn prepare_input(thok: &mut Thok, c: char) -> (usize, char, Outcome, SystemTime, u64) {
    let idx = if thok.strict_mode {
        thok.cursor_pos
    } else {
        thok.input.len()
    };

    if idx == 0 && thok.started_at.is_none() {
        thok.start();
    }

    let now = SystemTime::now();
    let expected_char = thok.get_expected_char(idx);
    let outcome = if c == expected_char {
        Outcome::Correct
    } else {
        Outcome::Incorrect
    };

    let keypress_time = if let Some(start_time) = thok.keypress_start_time {
        crate::stats::time_diff_ms(start_time, now)
    } else {
        0
    };
    let inter_key_time = thok.calculate_inter_key_time(now);
    let time_to_press_ms = if inter_key_time > 0 {
        inter_key_time
    } else if keypress_time > 5 {
        keypress_time
    } else if thok.input.is_empty() && thok.started_at.is_some() {
        if let Some(start_time) = thok.started_at {
            let since_start = crate::stats::time_diff_ms(start_time, now);
            if since_start > 0 {
                since_start
            } else {
                150
            }
        } else {
            150
        }
    } else {
        150
    };

    // Record char stat
    if let Some(ref mut stats_db) = thok.stats_db {
        let (context_before, context_after) = extract_context(&thok.prompt, idx, 3);
        let stat = CharStat {
            character: expected_char.to_lowercase().next().unwrap_or(expected_char),
            time_to_press_ms,
            was_correct: outcome == Outcome::Correct,
            was_uppercase: expected_char.is_uppercase(),
            timestamp: Local::now(),
            context_before,
            context_after,
        };
        let _ = stats_db.record_char_stat(&stat);
    }

    (idx, expected_char, outcome, now, time_to_press_ms)
}

pub fn write_normal(thok: &mut Thok, c: char) {
    let (_idx, _expected, outcome, now, _ttp) = prepare_input(thok, c);
    // Always insert and advance
    thok.input.insert(
        thok.cursor_pos,
        Input {
            char: c,
            outcome,
            timestamp: now,
            keypress_start: thok.keypress_start_time,
        },
    );
    thok.increment_cursor();
    thok.keypress_start_time = None;
}

pub fn write_strict(thok: &mut Thok, c: char) {
    let (idx, expected, outcome, now, _ttp) = prepare_input(thok, c);

    if outcome == Outcome::Correct {
        let had_error = thok.cursor_pos < thok.input.len()
            && thok.input[thok.cursor_pos].outcome == Outcome::Incorrect;
        if had_error {
            thok.corrected_positions.insert(thok.cursor_pos);
        }
        if thok.cursor_pos < thok.input.len() {
            thok.input[thok.cursor_pos] = Input {
                char: c,
                outcome,
                timestamp: now,
                keypress_start: thok.keypress_start_time,
            };
        } else {
            thok.input.push(Input {
                char: c,
                outcome,
                timestamp: now,
                keypress_start: thok.keypress_start_time,
            });
        }
        thok.increment_cursor();
    } else if thok.cursor_pos < thok.input.len() {
        thok.input[thok.cursor_pos] = Input {
            char: c,
            outcome,
            timestamp: now,
            keypress_start: thok.keypress_start_time,
        };
    } else {
        thok.input.push(Input {
            char: c,
            outcome,
            timestamp: now,
            keypress_start: thok.keypress_start_time,
        });
        // Cursor stays for retry
    }

    // Avoid unused warnings for variables we kept for parity
    let _ = (idx, expected);
    thok.keypress_start_time = None;
}

pub fn apply_write(thok: &mut Thok, c: char) {
    if thok.strict_mode {
        write_strict(thok, c)
    } else {
        write_normal(thok, c)
    }
}
