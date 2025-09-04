use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::{App, SortBy};

pub struct CharRowData {
    pub character: char,
    pub avg_time: f64,
    pub miss_rate: f64,
    pub attempts: i64,
    pub time_delta: Option<f64>,
    pub miss_delta: Option<f64>,
    pub session_attempts: i64,
    pub latest_datetime: Option<String>,
}

/// Pure presenter for a single character stats row
/// Returns a Row given the raw tuple from Thok summary
pub fn present_row(data: &CharRowData) -> Row<'static> {
    let char_display = if data.character == ' ' {
        "SPACE".to_string()
    } else {
        data.character.to_string()
    };

    let time_color = if data.avg_time < 150.0 {
        Color::Green
    } else if data.avg_time < 250.0 {
        Color::Yellow
    } else {
        Color::Red
    };

    let miss_color = if data.miss_rate == 0.0 {
        Color::Green
    } else if data.miss_rate < 10.0 {
        Color::Yellow
    } else {
        Color::Red
    };

    // Format time with delta
    let time_display = if let Some(delta) = data.time_delta {
        if delta.abs() < 1.0 {
            format!("{:.1}", data.avg_time)
        } else if delta < 0.0 {
            format!("{:.1} ↓{:.0}", data.avg_time, delta.abs())
        } else {
            format!("{:.1} ↑{:.0}", data.avg_time, delta)
        }
    } else if data.session_attempts > 0 {
        format!("{:.1} •", data.avg_time) // new character this session
    } else {
        format!("{:.1}", data.avg_time)
    };

    // Format miss rate with delta
    let miss_display = if let Some(delta) = data.miss_delta {
        if delta.abs() < 0.5 {
            format!("{:.1}", data.miss_rate)
        } else if delta < 0.0 {
            format!("{:.1} ↓{:.1}", data.miss_rate, delta.abs())
        } else {
            format!("{:.1} ↑{:.1}", data.miss_rate, delta)
        }
    } else if data.session_attempts > 0 {
        format!("{:.1} •", data.miss_rate)
    } else {
        format!("{:.1}", data.miss_rate)
    };

    // Format attempts with session info
    let attempts_display = if data.session_attempts > 0 {
        format!("{} (+{})", data.attempts, data.session_attempts)
    } else {
        data.attempts.to_string()
    };

    // Color deltas: green for improvement, red for regression
    let time_style = if let Some(delta) = data.time_delta {
        if delta < -5.0 {
            Style::default().fg(Color::Green)
        } else if delta > 5.0 {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        }
    } else {
        Style::default().fg(time_color)
    };

    let miss_style = if let Some(delta) = data.miss_delta {
        if delta < -1.0 {
            Style::default().fg(Color::Green)
        } else if delta > 1.0 {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        }
    } else {
        Style::default().fg(miss_color)
    };

    Row::new(vec![
        Cell::from(char_display).style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from(time_display).style(time_style),
        Cell::from(miss_display).style(miss_style),
        Cell::from(attempts_display),
        Cell::from(
            data.latest_datetime
                .clone()
                .unwrap_or_else(|| "—".to_string()),
        ),
    ])
}

/// Render the Character Statistics screen
pub fn render_character_stats(app: &mut App, f: &mut Frame) {
    let area = f.area();

    // Create layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Stats table
            Constraint::Length(4), // Instructions
        ])
        .split(area);

    // Title with sort indicator
    let sort_direction = if app.char_stats_state.sort_ascending {
        "↑"
    } else {
        "↓"
    };
    let sort_by_text = match app.char_stats_state.sort_by {
        SortBy::Character => "Character",
        SortBy::AvgTime => "Avg Time",
        SortBy::MissRate => "Miss Rate",
        SortBy::Attempts => "Attempts",
    };
    let title_text = format!("Character Statistics (Sort: {sort_by_text} {sort_direction})");

    let title = Paragraph::new(title_text)
        .block(Block::default().borders(Borders::ALL).title("Stats"))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Get character statistics with session deltas
    if let Some(mut summary) = app.thok.get_char_summary_with_deltas() {
        // Sort the data based on current sort criteria
        match app.char_stats_state.sort_by {
            SortBy::Character => summary.sort_by(|a, b| {
                let cmp = a.0.cmp(&b.0);
                if app.char_stats_state.sort_ascending {
                    cmp
                } else {
                    cmp.reverse()
                }
            }),
            SortBy::AvgTime => summary.sort_by(|a, b| {
                let cmp = a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal);
                if app.char_stats_state.sort_ascending {
                    cmp
                } else {
                    cmp.reverse()
                }
            }),
            SortBy::MissRate => summary.sort_by(|a, b| {
                let cmp = a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal);
                if app.char_stats_state.sort_ascending {
                    cmp
                } else {
                    cmp.reverse()
                }
            }),
            SortBy::Attempts => summary.sort_by(|a, b| {
                let cmp = a.3.cmp(&b.3);
                if app.char_stats_state.sort_ascending {
                    cmp
                } else {
                    cmp.reverse()
                }
            }),
        }

        // Calculate scrolling bounds
        let table_height = chunks[1].height.saturating_sub(3) as usize; // borders + header
        let total_rows = summary.len();
        let max_scroll = total_rows.saturating_sub(table_height);

        // Clamp scroll offset
        if app.char_stats_state.scroll_offset > max_scroll {
            app.char_stats_state.scroll_offset = max_scroll;
        }

        // Create header with sort indicators
        let char_indicator = if matches!(app.char_stats_state.sort_by, SortBy::Character) {
            sort_direction
        } else {
            ""
        };
        let time_indicator = if matches!(app.char_stats_state.sort_by, SortBy::AvgTime) {
            sort_direction
        } else {
            ""
        };
        let miss_indicator = if matches!(app.char_stats_state.sort_by, SortBy::MissRate) {
            sort_direction
        } else {
            ""
        };
        let attempts_indicator = if matches!(app.char_stats_state.sort_by, SortBy::Attempts) {
            sort_direction
        } else {
            ""
        };

        let header = Row::new(vec![
            Cell::from(format!("Char {char_indicator}")),
            Cell::from(format!("Avg Time (ms) {time_indicator}")),
            Cell::from(format!("Miss Rate (%) {miss_indicator}")),
            Cell::from(format!("Attempts {attempts_indicator}")),
            Cell::from("Last Typed"),
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

        // Visible rows
        let visible_rows: Vec<Row> = summary
            .iter()
            .skip(app.char_stats_state.scroll_offset)
            .take(table_height)
            .map(
                |(
                    character,
                    avg_time,
                    miss_rate,
                    attempts,
                    time_delta,
                    miss_delta,
                    session_attempts,
                    latest_datetime,
                )| {
                    let data = CharRowData {
                        character: *character,
                        avg_time: *avg_time,
                        miss_rate: *miss_rate,
                        attempts: *attempts,
                        time_delta: *time_delta,
                        miss_delta: *miss_delta,
                        session_attempts: *session_attempts,
                        latest_datetime: latest_datetime.clone(),
                    };
                    present_row(&data)
                },
            )
            .collect();

        // Create the table
        let widths = [
            Constraint::Length(8),  // Char
            Constraint::Length(18), // Avg Time
            Constraint::Length(18), // Miss Rate
            Constraint::Length(12), // Attempts
            Constraint::Min(10),    // Last Typed
        ];

        let table = Table::new(visible_rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Character Stats"),
            )
            .column_spacing(2);

        f.render_widget(table, chunks[1]);
    } else {
        // No data state
        let no_data =
            Paragraph::new("No character statistics available yet. Type to collect data.")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Gray));
        f.render_widget(no_data, chunks[1]);
    }

    // Instructions
    let instructions = Paragraph::new(
        "(↑/↓) scroll  (PgUp/PgDn) page  (Home) top  (1-4) sort  (b/backspace) back  (n) new  (r) retry",
    )
    .alignment(Alignment::Center)
    .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(instructions, chunks[2]);
}
