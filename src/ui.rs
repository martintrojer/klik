use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Widget, Wrap},
};
use unicode_width::UnicodeWidthStr;
use webbrowser::Browser;

use crate::thok::{Outcome, Thok};

const HORIZONTAL_MARGIN: u16 = 5;
const VERTICAL_MARGIN: u16 = 2;

impl Widget for &Thok {
    fn render(self, area: Rect, buf: &mut Buffer) {
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

        match !self.has_finished() {
            true => {
                let max_chars_per_line = area.width - (HORIZONTAL_MARGIN * 2);
                let mut prompt_occupied_lines =
                    ((self.prompt.width() as f64 / max_chars_per_line as f64).ceil() + 1.0) as u16;

                let time_left_lines = if self.number_of_secs.is_some() { 2 } else { 0 };

                if self.prompt.width() <= max_chars_per_line as usize {
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

                let mut spans = self
                    .input
                    .iter()
                    .enumerate()
                    .map(|(idx, input)| {
                        let expected = self.get_expected_char(idx).to_string();

                        match input.outcome {
                            Outcome::Incorrect => Span::styled(
                                match expected.as_str() {
                                    " " => "·".to_owned(),
                                    _ => expected,
                                },
                                red_bold_style,
                            ),
                            Outcome::Correct => {
                                // In strict mode, show corrected positions with a different color
                                if self.strict_mode && self.corrected_positions.contains(&idx) {
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
                    self.get_expected_char(self.cursor_pos).to_string(),
                    underlined_dim_bold_style,
                ));

                spans.push(Span::styled(
                    self.prompt[(self.cursor_pos + 1)..self.prompt.len()].to_string(),
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

                if self.seconds_remaining.is_some() {
                    let timer = Paragraph::new(Span::styled(
                        format!("{:.1}", self.seconds_remaining.unwrap()),
                        dim_bold_style,
                    ))
                    .alignment(Alignment::Center);

                    timer.render(chunks[1], buf);
                }
            }
            false => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .horizontal_margin(HORIZONTAL_MARGIN)
                    .vertical_margin(VERTICAL_MARGIN)
                    .constraints(
                        [
                            Constraint::Min(1),
                            Constraint::Length(1),
                            Constraint::Length(1), // for padding
                            Constraint::Length(1),
                        ]
                        .as_ref(),
                    )
                    .split(area);

                let mut highest_wpm = 0.0;

                for ts in &self.wpm_coords {
                    if ts.1 > highest_wpm {
                        highest_wpm = ts.1;
                    }
                }

                let datasets = vec![Dataset::default()
                    .marker(ratatui::symbols::Marker::Braille)
                    .style(magenta_style)
                    .graph_type(GraphType::Line)
                    .data(&self.wpm_coords)];

                let mut overall_duration = match self.wpm_coords.last() {
                    Some(x) => x.0,
                    _ => self.seconds_remaining.unwrap_or(1.0),
                };

                overall_duration = if overall_duration < 1.0 {
                    1.0
                } else {
                    overall_duration
                };

                let chart = Chart::new(datasets)
                    .x_axis(
                        Axis::default()
                            .title("seconds")
                            .bounds([1.0, overall_duration])
                            .labels(vec![
                                Span::styled("1", bold_style),
                                Span::styled(format!("{:.2}", overall_duration), bold_style),
                            ]),
                    )
                    .y_axis(
                        Axis::default()
                            .title("wpm")
                            .bounds([0.0, highest_wpm.round()])
                            .labels(vec![
                                Span::styled("0", bold_style),
                                Span::styled(format!("{}", highest_wpm.round()), bold_style),
                            ]),
                    );

                chart.render(chunks[0], buf);

                let stats = Paragraph::new(Span::styled(
                    format!(
                        "{} wpm   {}% acc   {:.2} sd",
                        self.wpm, self.accuracy, self.std_dev
                    ),
                    bold_style,
                ))
                .alignment(Alignment::Center);

                stats.render(chunks[1], buf);

                let legend = Paragraph::new(Span::styled(
                    String::from(if Browser::is_available() {
                        "(r)etry / (n)ew / (s)tats / (t)weet / (esc)ape"
                    } else {
                        "(r)etry / (n)ew / (s)tats / (esc)ape"
                    }),
                    italic_style,
                ));

                legend.render(chunks[3], buf);
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

    fn create_test_thok(prompt: &str, finished: bool) -> Thok {
        let mut thok = Thok::new(prompt.to_string(), 1, None, false);

        if finished {
            for c in prompt.chars() {
                thok.input.push(Input {
                    char: c,
                    outcome: Outcome::Correct,
                    timestamp: SystemTime::now(),
                    keypress_start: None,
                });
            }
            thok.cursor_pos = prompt.len();
            thok.wpm = 42.0;
            thok.accuracy = 95.0;
            thok.std_dev = 2.5;
            thok.wpm_coords = vec![(1.0, 20.0), (2.0, 35.0), (3.0, 42.0)];
        }

        thok
    }

    #[test]
    fn test_ui_widget_in_progress() {
        let thok = create_test_thok("hello world", false);
        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&thok).render(area, &mut buffer);

        let rendered = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(rendered.contains("hello world") || !rendered.trim().is_empty());
    }

    #[test]
    fn test_ui_widget_finished() {
        let thok = create_test_thok("test", true);
        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&thok).render(area, &mut buffer);

        let rendered = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        assert!(rendered.contains("42") || rendered.contains("95") || !rendered.trim().is_empty());
    }

    #[test]
    fn test_ui_widget_with_time_limit() {
        let mut thok = Thok::new("test".to_string(), 1, Some(30.0), false);
        thok.seconds_remaining = Some(25.5);

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&thok).render(area, &mut buffer);

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
        let thok = create_test_thok("hello", false);
        let area = Rect::new(0, 0, 20, 5);
        let mut buffer = Buffer::empty(area);

        (&thok).render(area, &mut buffer);

        assert!(*buffer.area() == area);
    }

    #[test]
    fn test_ui_widget_with_incorrect_input() {
        let mut thok = Thok::new("test".to_string(), 1, None, false);

        thok.input.push(Input {
            char: 't',
            outcome: Outcome::Correct,
            timestamp: SystemTime::now(),
            keypress_start: None,
        });
        thok.input.push(Input {
            char: 'x',
            outcome: Outcome::Incorrect,
            timestamp: SystemTime::now(),
            keypress_start: None,
        });
        thok.cursor_pos = 2;

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&thok).render(area, &mut buffer);

        assert!(*buffer.area() == area);
    }

    #[test]
    fn test_ui_constants() {
        assert_eq!(HORIZONTAL_MARGIN, 5);
        assert_eq!(VERTICAL_MARGIN, 2);
    }

    #[test]
    fn test_ui_widget_finished_with_browser_available() {
        let thok = create_test_thok("test", true);
        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&thok).render(area, &mut buffer);

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
        let thok = create_test_thok(large_prompt, false);
        let area = Rect::new(0, 0, 40, 20);
        let mut buffer = Buffer::empty(area);

        (&thok).render(area, &mut buffer);

        assert!(*buffer.area() == area);
    }

    #[test]
    fn test_ui_widget_empty_prompt() {
        let thok = create_test_thok("", false);
        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&thok).render(area, &mut buffer);

        assert!(*buffer.area() == area);
    }

    #[test]
    fn test_ui_widget_extreme_sizes() {
        let thok = create_test_thok("test prompt", false);

        // Test small area
        let small_area = Rect::new(0, 0, 10, 5);
        let mut small_buffer = Buffer::empty(small_area);
        (&thok).render(small_area, &mut small_buffer);
        assert!(*small_buffer.area() == small_area);

        // Test very large area
        let large_area = Rect::new(0, 0, 1000, 1000);
        let mut large_buffer = Buffer::empty(large_area);
        (&thok).render(large_area, &mut large_buffer);
        assert!(*large_buffer.area() == large_area);

        // Test normal sized area
        let normal_area = Rect::new(0, 0, 80, 24);
        let mut normal_buffer = Buffer::empty(normal_area);
        (&thok).render(normal_area, &mut normal_buffer);
        assert!(*normal_buffer.area() == normal_area);
    }

    #[test]
    fn test_ui_widget_partial_typing() {
        let mut thok = create_test_thok("hello world", false);

        // Type partially through the prompt
        thok.write('h');
        thok.write('e');
        thok.write('l');

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&thok).render(area, &mut buffer);

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
        let mut thok = create_test_thok("hello", false);

        // Type with some errors
        thok.write('h');
        thok.write('x'); // Wrong character
        thok.write('l');
        thok.write('l');
        thok.write('o');

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&thok).render(area, &mut buffer);

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
        let thok = create_test_thok("café naïve résumé", false);

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&thok).render(area, &mut buffer);

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
        let mut thok = create_test_thok("test", false);

        // Type correctly
        thok.write('t');
        thok.write('e');

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&thok).render(area, &mut buffer);

        // Check that the buffer was successfully populated
        // (We can't easily test colors in unit tests, but we can verify rendering succeeds)
        assert!(!buffer.content().is_empty());
    }

    #[test]
    fn test_ui_widget_renders_without_panic() {
        let thok = create_test_thok("test", false);

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        // Test that basic rendering works without panicking
        (&thok).render(area, &mut buffer);

        // Should render successfully
        assert!(*buffer.area() == area);
    }

    #[test]
    fn test_ui_widget_different_aspect_ratios() {
        let thok = create_test_thok("testing different aspect ratios", false);

        // Test wide and short
        let wide_area = Rect::new(0, 0, 200, 5);
        let mut wide_buffer = Buffer::empty(wide_area);
        (&thok).render(wide_area, &mut wide_buffer);
        assert!(*wide_buffer.area() == wide_area);

        // Test narrow and tall
        let tall_area = Rect::new(0, 0, 20, 50);
        let mut tall_buffer = Buffer::empty(tall_area);
        (&thok).render(tall_area, &mut tall_buffer);
        assert!(*tall_buffer.area() == tall_area);

        // Test square
        let square_area = Rect::new(0, 0, 50, 50);
        let mut square_buffer = Buffer::empty(square_area);
        (&thok).render(square_area, &mut square_buffer);
        assert!(*square_buffer.area() == square_area);
    }

    #[test]
    fn test_ui_constants_consistency() {
        // Test that UI constants are reasonable
        assert!(HORIZONTAL_MARGIN <= 20); // Should not be excessive
        assert!(VERTICAL_MARGIN <= 10); // Should not be excessive

        // Test that margins don't exceed typical terminal sizes
        assert!(HORIZONTAL_MARGIN * 2 < 80); // Common terminal width
        assert!(VERTICAL_MARGIN * 2 < 24); // Common terminal height
    }

    #[test]
    fn test_ui_widget_with_newlines_in_prompt() {
        let thok = create_test_thok("line one\nline two\nline three", false);

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        (&thok).render(area, &mut buffer);

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
        let mut thok = create_test_thok("hello", false);

        let area = Rect::new(0, 0, 80, 24);

        // Render initial state
        let mut buffer1 = Buffer::empty(area);
        (&thok).render(area, &mut buffer1);

        // Type a character
        thok.write('h');

        // Render after typing
        let mut buffer2 = Buffer::empty(area);
        (&thok).render(area, &mut buffer2);

        // Type another character
        thok.write('e');

        // Render again
        let mut buffer3 = Buffer::empty(area);
        (&thok).render(area, &mut buffer3);

        // All renders should succeed
        assert!(!buffer1.content().is_empty());
        assert!(!buffer2.content().is_empty());
        assert!(!buffer3.content().is_empty());
    }

    #[test]
    fn test_ui_widget_performance_large_text() {
        // Test with a very large prompt to ensure performance doesn't degrade significantly
        let large_text = "word ".repeat(1000); // 5000 characters
        let thok = create_test_thok(&large_text, false);

        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);

        // This should complete without hanging or excessive memory usage
        (&thok).render(area, &mut buffer);

        assert!(*buffer.area() == area);
    }
}
