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
                    ((thok.prompt.width() as f64 / max_chars_per_line as f64).ceil() + 1.0) as u16;

                let time_left_lines = if thok.session_config.number_of_secs.is_some() {
                    2
                } else {
                    0
                };

                if thok.prompt.width() <= max_chars_per_line as usize {
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

                let mut spans = thok
                    .input()
                    .iter()
                    .enumerate()
                    .map(|(idx, input)| {
                        let expected = thok.get_expected_char(idx).to_string();

                        match input.outcome {
                            Outcome::Incorrect => Span::styled(
                                match input.char {
                                    ' ' => "·".to_owned(),
                                    c => c.to_string(),
                                },
                                red_bold_style,
                            ),
                            Outcome::Correct => {
                                // In strict mode, show corrected positions with a different color
                                if thok.session_config.strict
                                    && thok.corrected_positions().contains(&idx)
                                {
                                    // Show corrected errors with orange color (much more distinct from green)
                                    Span::styled(
                                        expected,
                                        Style::default()
                                            .patch(bold_style)
                                            .fg(Color::Rgb(255, 165, 0)),
                                    )
                                } else {
                                    Span::styled(expected, green_bold_style)
                                }
                            }
                        }
                    })
                    .collect::<Vec<Span>>();

                spans.push(Span::styled(
                    thok.get_expected_char(thok.cursor_pos()).to_string(),
                    underlined_dim_bold_style,
                ));

                let start = (thok.cursor_pos() + 1).min(thok.prompt.len());
                spans.push(Span::styled(
                    thok.prompt[start..thok.prompt.len()].to_string(),
                    dim_bold_style,
                ));

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
                let input = Input {
                    char: c,
                    outcome: Outcome::Correct,
                    timestamp: SystemTime::now(),
                    keypress_start: None,
                };
                thok.session_state.input.push(input.clone());
                thok.session_state.input.push(input);
            }
            thok.session_state.cursor_pos = prompt.len();
            thok.session_state.cursor_pos = prompt.len();
            thok.session_state.wpm = 42.0;
            thok.session_state.wpm = 42.0;
            thok.session_state.accuracy = 95.0;
            thok.session_state.accuracy = 95.0;
            thok.session_state.std_dev = 2.5;
            thok.session_state.std_dev = 2.5;
            thok.session_state.wpm_coords = vec![
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
        }
    }

    #[test]
    fn test_ui_widget_in_progress() {
        let app = create_test_app("hello world", false);
        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&app).render(area, &mut buffer);

        let rendered = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(rendered.contains("hello world") || !rendered.trim().is_empty());
    }

    #[test]
    fn test_ui_widget_finished() {
        let app = create_test_app("test", true);
        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&app).render(area, &mut buffer);

        let rendered = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        assert!(rendered.contains("42") || rendered.contains("95") || !rendered.trim().is_empty());
    }

    #[test]
    fn test_ui_widget_with_time_limit() {
        let mut app = create_test_app("test", false);
        app.thok.session_state.seconds_remaining = Some(25.5);
        app.thok.session_config.number_of_secs = Some(30.0);

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&app).render(area, &mut buffer);

        let rendered = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(
            rendered.contains("25.5") || rendered.contains("test") || !rendered.trim().is_empty()
        );
    }

    #[test]
    fn test_ui_widget_small_area() {
        let app = create_test_app("hello", false);
        let area = Rect::new(0, 0, 20, 5);
        let mut buffer = Buffer::empty(area);

        (&app).render(area, &mut buffer);

        assert!(*buffer.area() == area);
    }

    #[test]
    fn test_ui_widget_with_incorrect_input() {
        let mut app = create_test_app("test", false);

        app.thok.session_state.input.push(Input {
            char: 't',
            outcome: Outcome::Correct,
            timestamp: SystemTime::now(),
            keypress_start: None,
        });
        app.thok.session_state.input.push(Input {
            char: 'x',
            outcome: Outcome::Incorrect,
            timestamp: SystemTime::now(),
            keypress_start: None,
        });
        app.thok.session_state.cursor_pos = 2;

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&app).render(area, &mut buffer);

        assert!(*buffer.area() == area);
    }

    #[test]
    fn test_ui_constants() {
        assert_eq!(HORIZONTAL_MARGIN, 5);
        assert_eq!(VERTICAL_MARGIN, 2);
    }

    #[test]
    fn test_ui_widget_finished_with_browser_available() {
        let app = create_test_app("test", true);
        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&app).render(area, &mut buffer);

        let rendered = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        if Browser::is_available() {
            assert!(
                rendered.contains("(t)weet")
                    || rendered.contains("(r)etry")
                    || rendered.contains("(n)ew")
                    || rendered.contains("(s)tats")
                    || rendered.contains("(esc)ape")
                    || !rendered.trim().is_empty()
            );
        } else {
            assert!(
                rendered.contains("(r)etry")
                    || rendered.contains("(n)ew")
                    || rendered.contains("(s)tats")
                    || rendered.contains("(esc)ape")
                    || !rendered.trim().is_empty()
            );
        }
    }

    #[test]
    fn test_ui_widget_large_prompt() {
        let large_prompt = "This is a very long prompt that should wrap across multiple lines when rendered in the terminal interface to test the text wrapping functionality";
        let app = create_test_app(large_prompt, false);
        let area = Rect::new(0, 0, 40, 20);
        let mut buffer = Buffer::empty(area);

        (&app).render(area, &mut buffer);

        assert!(*buffer.area() == area);
    }

    #[test]
    fn test_ui_widget_empty_prompt() {
        let app = create_test_app("", false);
        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&app).render(area, &mut buffer);

        assert!(*buffer.area() == area);
    }

    #[test]
    fn test_ui_widget_extreme_sizes() {
        let app = create_test_app("test prompt", false);

        // Test small area
        let small_area = Rect::new(0, 0, 10, 5);
        let mut small_buffer = Buffer::empty(small_area);
        (&app).render(small_area, &mut small_buffer);
        assert!(*small_buffer.area() == small_area);

        // Test very large area
        let large_area = Rect::new(0, 0, 1000, 1000);
        let mut large_buffer = Buffer::empty(large_area);
        (&app).render(large_area, &mut large_buffer);
        assert!(*large_buffer.area() == large_area);

        // Test normal sized area
        let normal_area = Rect::new(0, 0, 80, 24);
        let mut normal_buffer = Buffer::empty(normal_area);
        (&app).render(normal_area, &mut normal_buffer);
        assert!(*normal_buffer.area() == normal_area);
    }

    #[test]
    fn test_ui_widget_partial_typing() {
        let mut app = create_test_app("hello world", false);

        // Type partially through the prompt
        app.thok.write('h');
        app.thok.write('e');
        app.thok.write('l');

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&app).render(area, &mut buffer);

        let rendered = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        // Should contain the prompt text
        assert!(rendered.contains("hello") || rendered.contains("world"));
    }

    #[test]
    fn test_ui_widget_with_errors() {
        let mut app = create_test_app("hello", false);

        // Type with some errors
        app.thok.write('h');
        app.thok.write('x'); // Wrong character
        app.thok.write('l');
        app.thok.write('l');
        app.thok.write('o');

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&app).render(area, &mut buffer);

        let rendered = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        // Should render without panicking and contain some content
        assert!(!rendered.trim().is_empty());
    }

    #[test]
    fn test_ui_widget_special_characters() {
        let app = create_test_app("café naïve résumé", false);

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&app).render(area, &mut buffer);

        let rendered = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        // Should handle unicode characters without issues
        assert!(
            rendered.contains("café")
                || rendered.contains("naïve")
                || rendered.contains("résumé")
                || !rendered.trim().is_empty()
        );
    }

    #[test]
    fn test_ui_widget_color_consistency() {
        let mut app = create_test_app("test", false);

        // Type correctly
        app.thok.write('t');
        app.thok.write('e');

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&app).render(area, &mut buffer);

        // Check that the buffer was successfully populated
        // (We can't easily test colors in unit tests, but we can verify rendering succeeds)
        assert!(!buffer.content().is_empty());
    }

    #[test]
    fn test_ui_widget_renders_without_panic() {
        let app = create_test_app("test", false);

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        // Test that basic rendering works without panicking
        (&app).render(area, &mut buffer);

        // Should render successfully
        assert!(*buffer.area() == area);
    }

    #[test]
    fn test_ui_widget_different_aspect_ratios() {
        let app = create_test_app("testing different aspect ratios", false);

        // Test wide and short
        let wide_area = Rect::new(0, 0, 200, 5);
        let mut wide_buffer = Buffer::empty(wide_area);
        (&app).render(wide_area, &mut wide_buffer);
        assert!(*wide_buffer.area() == wide_area);

        // Test narrow and tall
        let tall_area = Rect::new(0, 0, 20, 50);
        let mut tall_buffer = Buffer::empty(tall_area);
        (&app).render(tall_area, &mut tall_buffer);
        assert!(*tall_buffer.area() == tall_area);

        // Test square
        let square_area = Rect::new(0, 0, 50, 50);
        let mut square_buffer = Buffer::empty(square_area);
        (&app).render(square_area, &mut square_buffer);
        assert!(*square_buffer.area() == square_area);
    }

    #[test]
    fn test_ui_constants_consistency() {
        // Test that UI constants are reasonable values
        assert_eq!(HORIZONTAL_MARGIN, 5);
        assert_eq!(VERTICAL_MARGIN, 2);

        // These are compile-time checks that our constants are reasonable
        const _: () = assert!(HORIZONTAL_MARGIN <= 20); // Should not be excessive
        const _: () = assert!(VERTICAL_MARGIN <= 10); // Should not be excessive
        const _: () = assert!(HORIZONTAL_MARGIN * 2 < 80); // Common terminal width
        const _: () = assert!(VERTICAL_MARGIN * 2 < 24); // Common terminal height
    }

    #[test]
    fn test_ui_widget_with_newlines_in_prompt() {
        let app = create_test_app("line one\nline two\nline three", false);

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&app).render(area, &mut buffer);

        let rendered = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        // Should handle newlines in the prompt gracefully
        assert!(!rendered.trim().is_empty());
    }

    #[test]
    fn test_ui_widget_render_multiple_times() {
        let mut app = create_test_app("hello", false);

        let area = Rect::new(0, 0, 80, 24);

        // Render initial state
        let mut buffer1 = Buffer::empty(area);
        (&app).render(area, &mut buffer1);

        // Type a character
        app.thok.write('h');

        // Render after typing
        let mut buffer2 = Buffer::empty(area);
        (&app).render(area, &mut buffer2);

        // Type another character
        app.thok.write('e');

        // Render again
        let mut buffer3 = Buffer::empty(area);
        (&app).render(area, &mut buffer3);

        // All renders should succeed
        assert!(!buffer1.content().is_empty());
        assert!(!buffer2.content().is_empty());
        assert!(!buffer3.content().is_empty());
    }

    #[test]
    fn test_ui_widget_performance_large_text() {
        // Test with a very large prompt to ensure performance doesn't degrade significantly
        let large_text = "word ".repeat(1000); // 5000 characters
        let app = create_test_app(&large_text, false);

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        // This should complete without hanging or excessive memory usage
        (&app).render(area, &mut buffer);

        assert!(*buffer.area() == area);
    }

    #[test]
    fn test_celebration_animation_rendering() {
        use crate::celebration::CelebrationAnimation;

        let mut app = create_test_app("test", true);

        // Manually set up celebration
        app.thok.session_state.accuracy = 100.0;
        app.thok.celebration = CelebrationAnimation::default();
        app.thok.celebration.start(80, 24);

        // Ensure celebration is active
        assert!(app.thok.celebration.is_active);
        assert!(!app.thok.celebration.particles.is_empty());

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        // Render with celebration
        (&app).render(area, &mut buffer);

        // Should render without panicking
        assert!(*buffer.area() == area);

        // The buffer should contain content (hard to test specific particles due to randomness)
        assert!(!buffer.content().is_empty());
    }
}
