use crate::stats::{extract_context, CharStat};
use crate::thok::{Input, Outcome, Thok};
use chrono::Local;
use std::time::SystemTime;

const DEFAULT_KEYPRESS_MS: u64 = 150;

fn calculate_time_to_press(thok: &Thok, now: SystemTime) -> u64 {
    let inter_key_time = thok.calculate_inter_key_time(now);
    if inter_key_time > 0 {
        return inter_key_time;
    }

    let keypress_time = thok
        .session
        .state
        .keypress_start_time
        .map(|start| crate::stats::time_diff_ms(start, now))
        .unwrap_or(0);
    if keypress_time > 5 {
        return keypress_time;
    }

    if thok.session.state.input.is_empty() {
        if let Some(start_time) = thok.session.state.started_at {
            let since_start = crate::stats::time_diff_ms(start_time, now);
            if since_start > 0 {
                return since_start;
            }
        }
    }

    DEFAULT_KEYPRESS_MS
}

struct PreparedInput {
    outcome: Outcome,
    now: SystemTime,
}

fn prepare_input(thok: &mut Thok, c: char) -> Option<PreparedInput> {
    if thok.has_finished() {
        return None;
    }

    let idx = if thok.session.config.strict {
        thok.session.state.cursor_pos
    } else {
        thok.session.state.input.len()
    };

    if idx == 0 && thok.session.state.started_at.is_none() {
        thok.start();
    }

    let now = SystemTime::now();
    let expected_char = thok.get_expected_char(idx);
    let outcome = if c == expected_char {
        Outcome::Correct
    } else {
        Outcome::Incorrect
    };

    let time_to_press_ms = calculate_time_to_press(thok, now);

    // Record char stat
    if let Some(ref mut stats_db) = thok.stats_db {
        let (context_before, context_after) = extract_context(&thok.session.prompt, idx, 3);
        let stat = CharStat {
            character: expected_char.to_lowercase().next().unwrap_or(expected_char),
            time_to_press_ms,
            was_correct: outcome == Outcome::Correct,
            was_uppercase: expected_char.is_uppercase(),
            timestamp: Local::now(),
            context_before,
            context_after,
        };
        if let Err(e) = stats_db.record_char_stat(&stat) {
            #[cfg(any(debug_assertions, test))]
            eprintln!("Failed to record char stat: {}", e);
        }
    }

    Some(PreparedInput { outcome, now })
}

pub fn write_normal(thok: &mut Thok, c: char) {
    let Some(prepared) = prepare_input(thok, c) else {
        return;
    };
    thok.session.state.input.insert(
        thok.session.state.cursor_pos,
        Input {
            char: c,
            outcome: prepared.outcome,
            timestamp: prepared.now,
            keypress_start: thok.session.state.keypress_start_time,
        },
    );
    thok.increment_cursor();
    thok.session.state.keypress_start_time = None;
}

pub fn write_strict(thok: &mut Thok, c: char) {
    let Some(prepared) = prepare_input(thok, c) else {
        return;
    };
    let input = Input {
        char: c,
        outcome: prepared.outcome,
        timestamp: prepared.now,
        keypress_start: thok.session.state.keypress_start_time,
    };

    if prepared.outcome == Outcome::Correct {
        let had_error = thok.session.state.cursor_pos < thok.session.state.input.len()
            && thok.session.state.input[thok.session.state.cursor_pos].outcome
                == Outcome::Incorrect;
        if had_error {
            thok.session
                .state
                .corrected_positions
                .insert(thok.session.state.cursor_pos);
        }
        if thok.session.state.cursor_pos < thok.session.state.input.len() {
            thok.session.state.input[thok.session.state.cursor_pos] = input;
        } else {
            thok.session.state.input.push(input);
        }
        thok.increment_cursor();
    } else if thok.session.state.cursor_pos < thok.session.state.input.len() {
        thok.session.state.input[thok.session.state.cursor_pos] = input;
    } else {
        thok.session.state.input.push(input);
    }

    thok.session.state.keypress_start_time = None;
}

pub fn apply_write(thok: &mut Thok, c: char) {
    if thok.session.config.strict {
        write_strict(thok, c)
    } else {
        write_normal(thok, c)
    }
}
