pub mod character_stats;
pub mod charting;
pub mod screen;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Widget, Wrap},
};
use unicode_width::UnicodeWidthStr;
use webbrowser::Browser;

use crate::{thok::Outcome, App, AppState};

const HORIZONTAL_MARGIN: u16 = 5;
const VERTICAL_MARGIN: u16 = 2;

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let thok = &self.thok;
        // styles
        let bold_style = Style::default().add_modifier(Modifier::BOLD);

        let green_bold_style = Style::default().patch(bold_style).fg(Color::Green);
        let red_bold_style = Style::default().patch(bold_style).fg(Color::Red);

        let dim_bold_style = Style::default()
            .patch(bold_style)
            .add_modifier(Modifier::DIM);

        let underlined_dim_bold_style = Style::default()
            .patch(dim_bold_style)
            .add_modifier(Modifier::UNDERLINED);

        let italic_style = Style::default().add_modifier(Modifier::ITALIC);

        let magenta_style = Style::default().fg(Color::Magenta);

        match (!thok.has_finished(), thok.is_idle()) {
            (true, true) => {
                // Idle state - show idle message
                let idle_message = Paragraph::new(Span::styled(
                    "IDLE - Press any key to continue typing",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD | Modifier::ITALIC),
                ))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });

                idle_message.render(area, buf);
            }
            (true, false) => {
                let max_chars_per_line = area.width - (HORIZONTAL_MARGIN * 2);
                let mut prompt_occupied_lines =
                    ((thok.session.prompt.width() as f64 / max_chars_per_line as f64).ceil() + 1.0)
                        as u16;

                let time_left_lines = if thok.session.config.number_of_secs.is_some() {
                    2
                } else {
                    0
                };

                if thok.session.prompt.width() <= max_chars_per_line as usize {
                    prompt_occupied_lines = 1;
                }

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .horizontal_margin(HORIZONTAL_MARGIN)
                    .constraints(
                        [
                            Constraint::Length(
                                ((area.height as f64 - prompt_occupied_lines as f64) / 2.0) as u16,
                            ),
                            Constraint::Length(time_left_lines),
                            Constraint::Length(prompt_occupied_lines),
                            Constraint::Length(
                                ((area.height as f64 - prompt_occupied_lines as f64) / 2.0) as u16,
                            ),
                        ]
                        .as_ref(),
                    )
                    .split(area);

                // Preallocate spans vector to avoid reallocations
                let input_len = thok.input().len();
                let mut spans = Vec::with_capacity(input_len + 2); // +2 for cursor and remaining

                // Build spans for typed input, avoiding repeated string conversions
                for (idx, input) in thok.input().iter().enumerate() {
                    match input.outcome {
                        Outcome::Incorrect => {
                            let char_str = if input.char == ' ' {
                                "·"
                            } else {
                                // For single chars, convert to String once
                                spans.push(Span::styled(input.char.to_string(), red_bold_style));
                                continue;
                            };
                            spans.push(Span::styled(char_str, red_bold_style));
                        }
                        Outcome::Correct => {
                            let expected = thok.get_expected_char(idx);
                            let style = if thok.session.config.strict
                                && thok.corrected_positions().contains(&idx)
                            {
                                // Show corrected errors with orange color (much more distinct from green)
                                Style::default()
                                    .patch(bold_style)
                                    .fg(Color::Rgb(255, 165, 0))
                            } else {
                                green_bold_style
                            };
                            spans.push(Span::styled(expected.to_string(), style));
                        }
                    }
                }

                spans.push(Span::styled(
                    thok.get_expected_char(thok.cursor_pos()).to_string(),
                    underlined_dim_bold_style,
                ));

                // Append the remaining prompt after the cursor using character indexing to avoid
                // slicing by byte indices (handles Unicode safely)
                // Preallocate remaining string with approximate capacity
                let cursor_pos = thok.cursor_pos();
                let remaining_len = thok
                    .session
                    .prompt
                    .chars()
                    .count()
                    .saturating_sub(cursor_pos + 1);
                let mut remaining = String::with_capacity(remaining_len);
                remaining.extend(thok.session.prompt.chars().skip(cursor_pos + 1));
                spans.push(Span::styled(remaining, dim_bold_style));

                let widget = Paragraph::new(Line::from(spans))
                    .alignment(if prompt_occupied_lines == 1 {
                        // when the prompt is small enough to fit on one line
                        // centering the text gives a nice zen feeling
                        Alignment::Center
                    } else {
                        Alignment::Left
                    })
                    .wrap(Wrap { trim: true });

                widget.render(chunks[2], buf);

                if thok.seconds_remaining().is_some() {
                    let timer = Paragraph::new(Span::styled(
                        format!("{:.1}", thok.seconds_remaining().unwrap()),
                        dim_bold_style,
                    ))
                    .alignment(Alignment::Center);

                    timer.render(chunks[1], buf);
                }
            }
            (false, _) => {
                // Check if we're in the Results state to show settings
                let show_settings = matches!(self.state, AppState::Results);

                let constraints = if show_settings {
                    vec![
                        Constraint::Min(1),    // chart
                        Constraint::Length(1), // stats
                        Constraint::Length(1), // session delta summary
                        Constraint::Length(3), // settings info box
                        Constraint::Length(1), // padding
                        Constraint::Length(1), // legend
                    ]
                } else {
                    vec![
                        Constraint::Min(1),
                        Constraint::Length(1),
                        Constraint::Length(1), // for session delta summary
                        Constraint::Length(1), // for padding
                        Constraint::Length(1),
                    ]
                };

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .horizontal_margin(HORIZONTAL_MARGIN)
                    .vertical_margin(VERTICAL_MARGIN)
                    .constraints(constraints.as_slice())
                    .split(area);

                let (overall_duration, highest_wpm) = crate::ui::charting::compute_chart_params(
                    thok.wpm_coords(),
                    thok.seconds_remaining(),
                );

                let tuples: Vec<(f64, f64)> =
                    thok.wpm_coords().iter().map(|p| (p.t, p.wpm)).collect();
                let datasets = vec![Dataset::default()
                    .marker(ratatui::symbols::Marker::Braille)
                    .style(magenta_style)
                    .graph_type(GraphType::Line)
                    .data(&tuples)];

                let chart = Chart::new(datasets)
                    .x_axis(
                        Axis::default()
                            .title("seconds")
                            .bounds([1.0, overall_duration])
                            .labels(vec![
                                Span::styled("1", bold_style),
                                Span::styled(
                                    crate::ui::charting::format_label(overall_duration),
                                    bold_style,
                                ),
                            ]),
                    )
                    .y_axis(
                        Axis::default()
                            .title("wpm")
                            .bounds([0.0, highest_wpm])
                            .labels(vec![
                                Span::styled("0", bold_style),
                                Span::styled(
                                    crate::ui::charting::format_label(highest_wpm),
                                    bold_style,
                                ),
                            ]),
                    );

                chart.render(chunks[0], buf);

                let stats = Paragraph::new(Span::styled(
                    format!(
                        "{} wpm   {}% acc   {:.2} sd",
                        thok.wpm(),
                        thok.accuracy(),
                        thok.std_dev()
                    ),
                    bold_style,
                ))
                .alignment(Alignment::Center);

                stats.render(chunks[1], buf);

                // Render session delta summary
                let delta_summary = thok.get_session_delta_summary();
                let delta_widget = Paragraph::new(Span::styled(
                    delta_summary,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::ITALIC),
                ))
                .alignment(Alignment::Center);

                delta_widget.render(chunks[2], buf);

                // Render settings info box if in Results state
                if show_settings {
                    let settings_text = format!(
                        "Settings: Words: {} | Lang: {} | Random: {} | Caps: {} | Strict: {} | Symbols: {} | Subst: {}\n(w) Words (l) Language (1) Random (2) Caps (3) Strict (4) Symbols (5) Substitute",
                        self.runtime_settings.number_of_words,
                        self.runtime_settings.supported_language,
                        if self.runtime_settings.random_words { "ON" } else { "OFF" },
                        if self.runtime_settings.capitalize { "ON" } else { "OFF" },
                        if self.runtime_settings.strict { "ON" } else { "OFF" },
                        if self.runtime_settings.symbols { "ON" } else { "OFF" },
                        if self.runtime_settings.substitute { "ON" } else { "OFF" }
                    );

                    let settings_widget = Paragraph::new(settings_text)
                        .style(
                            Style::default()
                                .fg(Color::Gray)
                                .add_modifier(Modifier::ITALIC),
                        )
                        .alignment(Alignment::Center)
                        .wrap(Wrap { trim: true });

                    settings_widget.render(chunks[3], buf);
                }

                let legend_chunk_index = if show_settings { 5 } else { 4 };
                let legend = Paragraph::new(Span::styled(
                    String::from(if Browser::is_available() {
                        "(r)etry / (n)ew / (s)tats / (t)weet / (esc)ape"
                    } else {
                        "(r)etry / (n)ew / (s)tats / (esc)ape"
                    }),
                    italic_style,
                ));

                legend.render(chunks[legend_chunk_index], buf);

                // Render celebration animation if active
                if thok.celebration.is_active {
                    render_celebration_particles(&thok.celebration, area, buf);
                }
            }
        }
    }
}

/// Render celebration particles on top of the results screen
fn render_celebration_particles(
    celebration: &crate::celebration::CelebrationAnimation,
    area: Rect,
    buf: &mut Buffer,
) {
    let colors = [
        Color::Yellow,
        Color::Magenta,
        Color::Cyan,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::LightYellow,
    ];

    for particle in &celebration.particles {
        let x = particle.x as u16;
        let y = particle.y as u16;

        // Check bounds
        if x < area.width && y < area.height {
            let color = colors[particle.color_index % colors.len()];

            // Calculate alpha based on particle age for fade effect
            let alpha = 1.0 - (particle.age / particle.max_age);

            let style = if particle.is_text {
                // Text particles are always bold and bright
                if alpha > 0.4 {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color)
                }
            } else {
                // Regular decorative particles with normal fade
                if alpha > 0.7 {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                } else if alpha > 0.3 {
                    Style::default().fg(color)
                } else {
                    Style::default().fg(color).add_modifier(Modifier::DIM)
                }
            };

            // Set the particle character in the buffer
            if let Some(cell) = buf.cell_mut((area.x + x, area.y + y)) {
                cell.set_symbol(&particle.symbol.to_string());
                cell.set_style(style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thok::{Input, Outcome, Thok};
    use ratatui::{buffer::Buffer, layout::Rect};
    use std::time::SystemTime;

    fn create_test_app(prompt: &str, finished: bool) -> App {
        use crate::{RuntimeSettings, SupportedLanguage};
        let mut thok = Thok::new(prompt.to_string(), 1, None, false);

        if finished {
            for c in prompt.chars() {
                thok.session.state.input.push(Input {
                    char: c,
                    outcome: Outcome::Correct,
                    timestamp: SystemTime::now(),
                    keypress_start: None,
                });
            }
            thok.session.state.cursor_pos = prompt.len();
            thok.session.state.wpm = 42.0;
            thok.session.state.accuracy = 95.0;
            thok.session.state.std_dev = 2.5;
            thok.session.state.wpm_coords = vec![
                crate::time_series::TimeSeriesPoint::new(1.0, 20.0),
                crate::time_series::TimeSeriesPoint::new(2.0, 35.0),
                crate::time_series::TimeSeriesPoint::new(3.0, 42.0),
            ];
        }

        App {
            cli: None,
            thok,
            state: if finished {
                crate::AppState::Results
            } else {
                crate::AppState::Typing
            },
            char_stats_state: crate::CharStatsState::default(),
            runtime_settings: RuntimeSettings {
                number_of_words: 15,
                number_of_sentences: None,
                number_of_secs: None,
                supported_language: SupportedLanguage::English,
                random_words: false,
                capitalize: false,
                strict: false,
                symbols: false,
                substitute: false,
            },
            config_store: Box::new(crate::config::FileConfigStore::default()),
        }
    }

    fn render_to_string(app: &App, area: Rect) -> String {
        let mut buffer = Buffer::empty(area);
        app.render(area, &mut buffer);
        buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>()
    }

    const STD_AREA: Rect = Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 24,
    };

    // -- Rendering doesn't panic for various sizes and edge cases --

    #[test]
    fn test_render_various_sizes() {
        let areas = [
            Rect::new(0, 0, 10, 5),     // tiny
            Rect::new(0, 0, 20, 50),    // narrow tall
            Rect::new(0, 0, 200, 5),    // wide short
            Rect::new(0, 0, 80, 24),    // standard
            Rect::new(0, 0, 40, 20),    // medium
            Rect::new(0, 0, 1000, 100), // huge
        ];
        for area in areas {
            let app = create_test_app("test prompt", false);
            let mut buffer = Buffer::empty(area);
            (&app).render(area, &mut buffer);
        }
    }

    #[test]
    fn test_render_edge_case_prompts() {
        for prompt in [
            "",
            "x",
            "line one\nline two\nline three",
            &"word ".repeat(1000),
        ] {
            let app = create_test_app(prompt, false);
            let mut buffer = Buffer::empty(STD_AREA);
            (&app).render(STD_AREA, &mut buffer);
        }
    }

    // -- In-progress rendering --

    #[test]
    fn test_in_progress_shows_prompt() {
        let rendered = render_to_string(&create_test_app("hello world", false), STD_AREA);
        assert!(rendered.contains("hello world"));
    }

    #[test]
    fn test_in_progress_with_timer() {
        let mut app = create_test_app("test", false);
        app.thok.session.state.seconds_remaining = Some(25.5);
        app.thok.session.config.number_of_secs = Some(30.0);

        let rendered = render_to_string(&app, STD_AREA);
        assert!(rendered.contains("25.5"));
    }

    #[test]
    fn test_partial_typing_shows_prompt() {
        let mut app = create_test_app("hello world", false);
        app.thok.write('h');
        app.thok.write('e');
        app.thok.write('l');

        let rendered = render_to_string(&app, STD_AREA);
        assert!(rendered.contains("lo world"));
    }

    #[test]
    fn test_unicode_prompt_renders() {
        let rendered = render_to_string(
            &create_test_app("cafe\u{301} nai\u{308}ve", false),
            STD_AREA,
        );
        assert!(!rendered.trim().is_empty());
    }

    #[test]
    fn test_incorrect_input_renders() {
        let mut app = create_test_app("test", false);
        app.thok.session.state.input.push(Input {
            char: 't',
            outcome: Outcome::Correct,
            timestamp: SystemTime::now(),
            keypress_start: None,
        });
        app.thok.session.state.input.push(Input {
            char: 'x',
            outcome: Outcome::Incorrect,
            timestamp: SystemTime::now(),
            keypress_start: None,
        });
        app.thok.session.state.cursor_pos = 2;

        let rendered = render_to_string(&app, STD_AREA);
        assert!(!rendered.trim().is_empty());
    }

    // -- Finished/results rendering --

    #[test]
    fn test_finished_shows_stats() {
        let rendered = render_to_string(&create_test_app("test", true), STD_AREA);
        assert!(rendered.contains("42")); // wpm
        assert!(rendered.contains("95")); // accuracy
    }

    #[test]
    fn test_finished_shows_legend() {
        let rendered = render_to_string(&create_test_app("test", true), STD_AREA);
        assert!(rendered.contains("(r)etry"));
        assert!(rendered.contains("(n)ew"));
        assert!(rendered.contains("(esc)ape"));
    }

    // -- State progression --

    #[test]
    fn test_render_changes_after_typing() {
        let mut app = create_test_app("hello", false);
        let mut buf_before = Buffer::empty(STD_AREA);
        (&app).render(STD_AREA, &mut buf_before);

        app.thok.write('h');
        let mut buf_after = Buffer::empty(STD_AREA);
        (&app).render(STD_AREA, &mut buf_after);

        // Content text is the same but styles differ (green vs underlined)
        assert_ne!(buf_before, buf_after);
    }

    // -- Celebration --

    #[test]
    fn test_celebration_renders() {
        let mut app = create_test_app("test", true);
        app.thok.celebration = crate::celebration::CelebrationAnimation::default();
        app.thok.celebration.start(80, 24);
        assert!(app.thok.celebration.is_active);

        let mut buffer = Buffer::empty(STD_AREA);
        (&app).render(STD_AREA, &mut buffer);
    }
}
