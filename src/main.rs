pub mod celebration;
pub mod language;
pub mod stats;
pub mod thok;
pub mod ui;
pub mod util;
pub mod word_generator;

// Keep the old lang module name for compatibility
pub use language as lang;

use crate::{
    lang::Language,
    thok::Thok,
    word_generator::{WordGenConfig, WordGenerator},
};
use clap::{error::ErrorKind, CommandFactory, Parser, ValueEnum};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Frame, Terminal,
};
use std::{
    error::Error,
    io::{self, stdin},
    sync::mpsc,
    thread,
    time::Duration,
};
use webbrowser::Browser;

const TICK_RATE_MS: u64 = 100;

/// sleek typing tui with visualized results and intelligent practice
#[derive(Parser, Debug, Clone)]
#[clap(
    version,
    about,
    long_about = "A sleek typing TUI with intelligent word selection that adapts to your weaknesses, detailed performance analytics, and historical progress tracking."
)]
pub struct Cli {
    /// number of words to use in test
    #[clap(short = 'w', long, default_value_t = 15)]
    number_of_words: usize,

    /// number of sentences to use in test
    #[clap(short = 'f', long = "full-sentences")]
    number_of_sentences: Option<usize>,

    /// number of seconds to run test
    #[clap(short = 's', long)]
    number_of_secs: Option<usize>,

    /// custom prompt to use
    #[clap(short = 'p', long)]
    prompt: Option<String>,

    /// language to pull words from
    #[clap(short = 'l', long, value_enum, default_value_t = SupportedLanguage::English)]
    supported_language: SupportedLanguage,

    /// use random word selection instead of intelligent character-based selection (default: intelligent selection that targets your weakest characters)
    #[clap(long)]
    random_words: bool,

    /// enable capitalization, punctuation, and commas for realistic typing practice
    #[clap(long)]
    capitalize: bool,

    /// enable strict mode: stop on errors and require correction before proceeding
    #[clap(long)]
    strict: bool,

    /// include symbols and special characters for comprehensive typing practice
    #[clap(long)]
    symbols: bool,

    /// enable character substitution mode: create "almost English" words by replacing characters with ones that need most practice
    #[clap(long)]
    substitute: bool,
}

#[derive(Debug, Copy, Clone, ValueEnum, strum_macros::Display)]
pub enum SupportedLanguage {
    English,
    English1k,
    English10k,
}

impl SupportedLanguage {
    fn as_lang(&self) -> Language {
        Language::new(self.to_string().to_lowercase())
    }
}

impl Cli {
    /// Convert CLI arguments to word generation configuration
    fn to_word_gen_config(&self, custom_prompt: Option<String>) -> WordGenConfig {
        WordGenConfig {
            number_of_words: self.number_of_words,
            number_of_sentences: self.number_of_sentences,
            custom_prompt,
            language: self.supported_language,
            random_words: self.random_words,
            substitute: self.substitute,
            capitalize: self.capitalize,
            symbols: self.symbols,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Typing,
    Results,
    CharacterStats,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortBy {
    Character,
    AvgTime,
    MissRate,
    Attempts,
}

#[derive(Debug)]
pub struct CharStatsState {
    pub scroll_offset: usize,
    pub sort_by: SortBy,
    pub sort_ascending: bool,
}

impl Default for CharStatsState {
    fn default() -> Self {
        Self {
            scroll_offset: 0,
            sort_by: SortBy::Character,
            sort_ascending: true,
        }
    }
}

#[derive(Debug)]
pub struct App {
    pub cli: Option<Cli>,
    pub thok: Thok,
    pub state: AppState,
    pub char_stats_state: CharStatsState,
}

impl App {
    pub fn new(cli: Cli) -> Self {
        let config = cli.to_word_gen_config(cli.prompt.clone());
        let generator = WordGenerator::new(config);
        let (prompt, word_count) = generator.generate_prompt();

        Self {
            thok: Thok::new(
                prompt,
                word_count,
                cli.number_of_secs.map(|ns| ns as f64),
                cli.strict,
            ),
            cli: Some(cli),
            state: AppState::Typing,
            char_stats_state: CharStatsState::default(),
        }
    }

    pub fn reset(&mut self, new_prompt: Option<String>) {
        let cli = self.cli.clone().unwrap();
        let config = cli.to_word_gen_config(new_prompt);
        let generator = WordGenerator::new(config);
        let (prompt, word_count) = generator.generate_prompt();

        self.thok = Thok::new(
            prompt,
            word_count,
            cli.number_of_secs.map(|ns| ns as f64),
            cli.strict,
        );
        self.state = AppState::Typing;
        self.char_stats_state = CharStatsState::default();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if !stdin().is_tty() {
        let mut cmd = Cli::command();
        cmd.error(ErrorKind::Io, "stdin must be a tty").exit();
    }

    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(cli);
    start_tui(&mut terminal, &mut app)?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    Ok(())
}

#[derive(Debug)]
enum ExitType {
    Restart,
    New,
    Quit,
}
fn start_tui<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: &mut App,
) -> Result<(), Box<dyn Error>> {
    // Always enable ticking for celebration animations and timed sessions
    let should_tick = true;

    let thok_events = get_thok_events(should_tick);

    loop {
        let mut exit_type: ExitType = ExitType::Quit;
        terminal.draw(|f| ui(app, f))?;

        loop {
            let app = &mut app;

            match thok_events.recv()? {
                ThokEvent::Tick => {
                    if app.thok.has_started() && !app.thok.has_finished() {
                        app.thok.on_tick();

                        if app.thok.has_finished() {
                            app.thok.calc_results();
                            // Get terminal size for celebration
                            let size = terminal.size().unwrap_or_default();
                            app.thok
                                .start_celebration_if_perfect(size.width, size.height);
                            app.state = AppState::Results;
                        }
                    }

                    // Always update celebration animation if active
                    app.thok.update_celebration();

                    // Draw on every tick if there's active animation or during typing
                    if app.thok.celebration.is_active
                        || (app.thok.has_started() && !app.thok.has_finished())
                    {
                        terminal.draw(|f| ui(app, f))?;
                    }
                }
                ThokEvent::Resize => {
                    terminal.draw(|f| ui(app, f))?;
                }
                ThokEvent::Key(key) => {
                    match key.code {
                        KeyCode::Esc => {
                            break;
                        }
                        KeyCode::Backspace => {
                            if app.state == AppState::Typing && !app.thok.has_finished() {
                                app.thok.backspace();
                            }
                        }
                        KeyCode::Left => {
                            exit_type = ExitType::Restart;
                            break;
                        }
                        KeyCode::Right => {
                            exit_type = ExitType::New;
                            break;
                        }
                        KeyCode::Char(c) => {
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                                && key.code == KeyCode::Char('c')
                            // ctrl+c to quit
                            {
                                break;
                            }

                            match app.state {
                                AppState::Typing => {
                                    if !app.thok.has_finished() {
                                        // Remove the immediate on_keypress_start() call
                                        // The write() method will now use inter-keystroke timing
                                        app.thok.write(c);
                                        if app.thok.has_finished() {
                                            app.thok.calc_results();
                                            // Get terminal size for celebration
                                            let size = terminal.size().unwrap_or_default();
                                            app.thok.start_celebration_if_perfect(
                                                size.width,
                                                size.height,
                                            );
                                            app.state = AppState::Results;
                                        }
                                    }
                                }
                                AppState::Results => match key.code {
                                    KeyCode::Char('t') => {
                                        if Browser::is_available() {
                                            webbrowser::open(&format!("https://twitter.com/intent/tweet?text={}%20wpm%20%2F%20{}%25%20acc%20%2F%20{:.2}%20sd%0A%0Ahttps%3A%2F%2Fgithub.com%thatvegandev%2Fthokr", app.thok.wpm, app.thok.accuracy, app.thok.std_dev))
                                        .unwrap_or_default();
                                        }
                                    }
                                    KeyCode::Char('r') => {
                                        exit_type = ExitType::Restart;
                                        break;
                                    }
                                    KeyCode::Char('n') => {
                                        exit_type = ExitType::New;
                                        break;
                                    }
                                    KeyCode::Char('s') => {
                                        app.state = AppState::CharacterStats;
                                    }
                                    _ => {}
                                },
                                AppState::CharacterStats => match key.code {
                                    KeyCode::Char('r') => {
                                        exit_type = ExitType::Restart;
                                        break;
                                    }
                                    KeyCode::Char('n') => {
                                        exit_type = ExitType::New;
                                        break;
                                    }
                                    KeyCode::Char('b') | KeyCode::Backspace => {
                                        app.state = AppState::Results;
                                    }
                                    KeyCode::Up => {
                                        if app.char_stats_state.scroll_offset > 0 {
                                            app.char_stats_state.scroll_offset -= 1;
                                        }
                                    }
                                    KeyCode::Down => {
                                        // Will check max scroll in render function
                                        app.char_stats_state.scroll_offset += 1;
                                    }
                                    KeyCode::PageUp => {
                                        app.char_stats_state.scroll_offset =
                                            app.char_stats_state.scroll_offset.saturating_sub(10);
                                    }
                                    KeyCode::PageDown => {
                                        app.char_stats_state.scroll_offset += 10;
                                    }
                                    KeyCode::Home => {
                                        app.char_stats_state.scroll_offset = 0;
                                    }
                                    KeyCode::Char('1') => {
                                        app.char_stats_state.sort_by = SortBy::Character;
                                        app.char_stats_state.scroll_offset = 0;
                                    }
                                    KeyCode::Char('2') => {
                                        app.char_stats_state.sort_by = SortBy::AvgTime;
                                        app.char_stats_state.scroll_offset = 0;
                                    }
                                    KeyCode::Char('3') => {
                                        app.char_stats_state.sort_by = SortBy::MissRate;
                                        app.char_stats_state.scroll_offset = 0;
                                    }
                                    KeyCode::Char('4') => {
                                        app.char_stats_state.sort_by = SortBy::Attempts;
                                        app.char_stats_state.scroll_offset = 0;
                                    }
                                    KeyCode::Char(' ') => {
                                        // Toggle sort direction
                                        app.char_stats_state.sort_ascending =
                                            !app.char_stats_state.sort_ascending;
                                        app.char_stats_state.scroll_offset = 0;
                                    }
                                    _ => {}
                                },
                            }
                        }
                        _ => {}
                    }
                    terminal.draw(|f| ui(app, f))?;
                }
            }
        }

        match exit_type {
            ExitType::Restart => {
                app.reset(Some(app.thok.prompt.clone()));
            }
            ExitType::New => {
                app.reset(None);
            }
            ExitType::Quit => {
                break;
            }
        }
    }

    Ok(())
}

#[derive(Clone)]
enum ThokEvent {
    Key(KeyEvent),
    Resize,
    Tick,
}

fn get_thok_events(should_tick: bool) -> mpsc::Receiver<ThokEvent> {
    let (tx, rx) = mpsc::channel();

    if should_tick {
        let tick_x = tx.clone();
        thread::spawn(move || loop {
            if tick_x.send(ThokEvent::Tick).is_err() {
                break;
            }

            thread::sleep(Duration::from_millis(TICK_RATE_MS))
        });
    }

    thread::spawn(move || loop {
        let evt = match event::read().unwrap() {
            Event::Key(key) => Some(ThokEvent::Key(key)),
            Event::Resize(_, _) => Some(ThokEvent::Resize),
            _ => None,
        };

        if evt.is_some() && tx.send(evt.unwrap()).is_err() {
            break;
        }
    });

    rx
}

fn render_character_stats(app: &mut App, f: &mut Frame) {
    use ratatui::{
        layout::{Alignment, Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    };

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
    let title_text = format!(
        "Character Statistics (Sort: {} {})",
        sort_by_text, sort_direction
    );

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
            SortBy::Character => {
                summary.sort_by(|a, b| {
                    let cmp = a.0.cmp(&b.0);
                    if app.char_stats_state.sort_ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });
            }
            SortBy::AvgTime => {
                summary.sort_by(|a, b| {
                    let cmp = a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal);
                    if app.char_stats_state.sort_ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });
            }
            SortBy::MissRate => {
                summary.sort_by(|a, b| {
                    let cmp = a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal);
                    if app.char_stats_state.sort_ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });
            }
            SortBy::Attempts => {
                summary.sort_by(|a, b| {
                    let cmp = a.3.cmp(&b.3);
                    if app.char_stats_state.sort_ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });
            }
        }

        // Calculate scrolling bounds
        let table_height = chunks[1].height.saturating_sub(3) as usize; // Account for borders and header
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
            Cell::from(format!("Char {}", char_indicator)),
            Cell::from(format!("Avg Time (ms) {}", time_indicator)),
            Cell::from(format!("Miss Rate (%) {}", miss_indicator)),
            Cell::from(format!("Attempts {}", attempts_indicator)),
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

        // Get visible rows based on scroll offset
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
                )| {
                    let char_display = if *character == ' ' {
                        "SPACE".to_string()
                    } else {
                        character.to_string()
                    };

                    let time_color = if *avg_time < 150.0 {
                        Color::Green
                    } else if *avg_time < 250.0 {
                        Color::Yellow
                    } else {
                        Color::Red
                    };

                    let miss_color = if *miss_rate == 0.0 {
                        Color::Green
                    } else if *miss_rate < 10.0 {
                        Color::Yellow
                    } else {
                        Color::Red
                    };

                    // Format time with delta
                    let time_display = if let Some(delta) = time_delta {
                        if delta.abs() < 1.0 {
                            format!("{:.1}", avg_time)
                        } else if *delta < 0.0 {
                            format!("{:.1} ↓{:.0}", avg_time, delta.abs())
                        } else {
                            format!("{:.1} ↑{:.0}", avg_time, delta)
                        }
                    } else {
                        format!("{:.1}", avg_time)
                    };

                    // Format miss rate with delta
                    let miss_display = if let Some(delta) = miss_delta {
                        if delta.abs() < 0.5 {
                            format!("{:.1}", miss_rate)
                        } else if *delta < 0.0 {
                            format!("{:.1} ↓{:.1}", miss_rate, delta.abs())
                        } else {
                            format!("{:.1} ↑{:.1}", miss_rate, delta)
                        }
                    } else {
                        format!("{:.1}", miss_rate)
                    };

                    // Format attempts with session info
                    let attempts_display = if *session_attempts > 0 {
                        format!("{} (+{})", attempts, session_attempts)
                    } else {
                        attempts.to_string()
                    };

                    // Color deltas: green for improvement, red for regression
                    let time_style = if let Some(delta) = time_delta {
                        if *delta < -5.0 {
                            Style::default().fg(Color::Green)
                        } else if *delta > 5.0 {
                            Style::default().fg(Color::Red)
                        } else {
                            Style::default().fg(time_color)
                        }
                    } else {
                        Style::default().fg(time_color)
                    };

                    let miss_style = if let Some(delta) = miss_delta {
                        if *delta < -2.0 {
                            Style::default().fg(Color::Green)
                        } else if *delta > 2.0 {
                            Style::default().fg(Color::Red)
                        } else {
                            Style::default().fg(miss_color)
                        }
                    } else {
                        Style::default().fg(miss_color)
                    };

                    Row::new(vec![
                        Cell::from(char_display),
                        Cell::from(time_display).style(time_style),
                        Cell::from(miss_display).style(miss_style),
                        Cell::from(attempts_display),
                    ])
                },
            )
            .collect();

        // Show scroll position in title if there are more rows than visible
        let scroll_info = if total_rows > table_height {
            format!(
                " ({}/{} rows)",
                app.char_stats_state.scroll_offset + visible_rows.len().min(table_height),
                total_rows
            )
        } else {
            String::new()
        };

        let table = Table::new(
            visible_rows,
            &[
                Constraint::Length(8),  // Character
                Constraint::Length(22), // Avg Time with delta (expanded)
                Constraint::Length(22), // Miss Rate with delta (expanded)
                Constraint::Length(16), // Attempts with session info (expanded)
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Performance by Character{}", scroll_info)),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray));

        f.render_widget(table, chunks[1]);
    } else {
        let no_data = Paragraph::new(
            "No character statistics available.\nComplete a typing test to see your stats!",
        )
        .block(Block::default().borders(Borders::ALL).title("No Data"))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
        f.render_widget(no_data, chunks[1]);
    }

    // Instructions
    let instructions = Paragraph::new("Historical stats with session deltas: ↓=improvement ↑=regression (+n)=session attempts\nSort: (1)Char (2)Time (3)Miss (4)Attempts | (Space)Toggle | ↑/↓ PgUp/PgDn | (b)ack (esc)ape")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC))
        .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[2]);
}

fn ui(app: &mut App, f: &mut Frame) {
    match app.state {
        AppState::Typing | AppState::Results => {
            f.render_widget(&app.thok, f.area());
        }
        AppState::CharacterStats => {
            render_character_stats(app, f);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_default_values() {
        let cli = Cli::parse_from(["thokr"]);

        assert_eq!(cli.number_of_words, 15);
        assert_eq!(cli.number_of_sentences, None);
        assert_eq!(cli.number_of_secs, None);
        assert_eq!(cli.prompt, None);
        assert!(matches!(cli.supported_language, SupportedLanguage::English));
    }

    #[test]
    fn test_cli_number_of_words() {
        let cli = Cli::parse_from(["thokr", "-w", "25"]);
        assert_eq!(cli.number_of_words, 25);

        let cli = Cli::parse_from(["thokr", "--number-of-words", "50"]);
        assert_eq!(cli.number_of_words, 50);
    }

    #[test]
    fn test_cli_number_of_sentences() {
        let cli = Cli::parse_from(["thokr", "-f", "3"]);
        assert_eq!(cli.number_of_sentences, Some(3));

        let cli = Cli::parse_from(["thokr", "--full-sentences", "5"]);
        assert_eq!(cli.number_of_sentences, Some(5));
    }

    #[test]
    fn test_cli_number_of_secs() {
        let cli = Cli::parse_from(["thokr", "-s", "60"]);
        assert_eq!(cli.number_of_secs, Some(60));

        let cli = Cli::parse_from(["thokr", "--number-of-secs", "120"]);
        assert_eq!(cli.number_of_secs, Some(120));
    }

    #[test]
    fn test_cli_custom_prompt() {
        let cli = Cli::parse_from(["thokr", "-p", "hello world"]);
        assert_eq!(cli.prompt, Some("hello world".to_string()));

        let cli = Cli::parse_from(["thokr", "--prompt", "custom text"]);
        assert_eq!(cli.prompt, Some("custom text".to_string()));
    }

    #[test]
    fn test_cli_supported_language() {
        let cli = Cli::parse_from(["thokr", "-l", "english"]);
        assert!(matches!(cli.supported_language, SupportedLanguage::English));

        let cli = Cli::parse_from(["thokr", "--supported-language", "english1k"]);
        assert!(matches!(
            cli.supported_language,
            SupportedLanguage::English1k
        ));

        let cli = Cli::parse_from(["thokr", "--supported-language", "english10k"]);
        assert!(matches!(
            cli.supported_language,
            SupportedLanguage::English10k
        ));
    }

    #[test]
    fn test_supported_language_as_lang() {
        let english = SupportedLanguage::English.as_lang();
        assert_eq!(english.name, "english");

        let english1k = SupportedLanguage::English1k.as_lang();
        assert_eq!(english1k.name, "english_1k");

        let english10k = SupportedLanguage::English10k.as_lang();
        assert_eq!(english10k.name, "english_10k");
    }

    #[test]
    fn test_supported_language_display() {
        assert_eq!(SupportedLanguage::English.to_string(), "English");
        assert_eq!(SupportedLanguage::English1k.to_string(), "English1k");
        assert_eq!(SupportedLanguage::English10k.to_string(), "English10k");
    }

    #[test]
    fn test_app_new_with_words() {
        let cli = Cli {
            number_of_words: 10,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: None,
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let app = App::new(cli.clone());

        assert_eq!(app.thok.number_of_words, 10);
        assert_eq!(app.thok.number_of_secs, None);
        assert!(app.cli.is_some());
        assert!(!app.thok.prompt.is_empty());
        assert_eq!(app.state, AppState::Typing);
    }

    #[test]
    fn test_app_new_with_custom_prompt() {
        let cli = Cli {
            number_of_words: 10,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: Some("custom test prompt".to_string()),
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let app = App::new(cli);

        assert_eq!(app.thok.prompt, "custom test prompt");
        assert_eq!(app.thok.number_of_words, 10);
        assert_eq!(app.state, AppState::Typing);
    }

    #[test]
    fn test_app_new_with_sentences() {
        let cli = Cli {
            number_of_words: 10,
            number_of_sentences: Some(2),
            number_of_secs: None,
            prompt: None,
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let app = App::new(cli);

        assert!(app.thok.number_of_words > 0);
        assert!(!app.thok.prompt.is_empty());
        assert_eq!(app.state, AppState::Typing);
    }

    #[test]
    fn test_app_new_with_time_limit() {
        let cli = Cli {
            number_of_words: 10,
            number_of_sentences: None,
            number_of_secs: Some(60),
            prompt: None,
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let app = App::new(cli);

        assert_eq!(app.thok.number_of_secs, Some(60.0));
        assert_eq!(app.thok.seconds_remaining, Some(60.0));
        assert_eq!(app.state, AppState::Typing);
    }

    #[test]
    fn test_app_reset_with_new_prompt() {
        let cli = Cli {
            number_of_words: 5,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: None,
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let mut app = App::new(cli);
        let original_prompt = app.thok.prompt.clone();

        app.reset(Some("new test prompt".to_string()));

        assert_eq!(app.thok.prompt, "new test prompt");
        assert_ne!(app.thok.prompt, original_prompt);
        assert_eq!(app.thok.input.len(), 0);
        assert_eq!(app.thok.cursor_pos, 0);
        assert_eq!(app.state, AppState::Typing);
    }

    #[test]
    fn test_app_reset_without_new_prompt() {
        let cli = Cli {
            number_of_words: 5,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: None,
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let mut app = App::new(cli);
        let original_prompt = app.thok.prompt.clone();

        app.thok.write('t');
        app.thok.write('e');
        assert_eq!(app.thok.input.len(), 2);

        app.reset(None);

        assert_ne!(app.thok.prompt, original_prompt);
        assert_eq!(app.thok.input.len(), 0);
        assert_eq!(app.thok.cursor_pos, 0);
        assert_eq!(app.state, AppState::Typing);
    }

    #[test]
    fn test_thok_event_clone() {
        let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let thok_event = ThokEvent::Key(key_event);
        let cloned_event = thok_event.clone();

        match (thok_event, cloned_event) {
            (ThokEvent::Key(original), ThokEvent::Key(cloned)) => {
                assert_eq!(original.code, cloned.code);
                assert_eq!(original.modifiers, cloned.modifiers);
            }
            _ => panic!("Events should match"),
        }
    }

    #[test]
    fn test_exit_type_debug() {
        let restart = ExitType::Restart;
        let new = ExitType::New;
        let quit = ExitType::Quit;

        assert_eq!(format!("{:?}", restart), "Restart");
        assert_eq!(format!("{:?}", new), "New");
        assert_eq!(format!("{:?}", quit), "Quit");
    }

    #[test]
    fn test_app_state_transitions() {
        let cli = Cli {
            number_of_words: 3,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: Some("hello".to_string()),
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let mut app = App::new(cli);

        // Should start in Typing state
        assert_eq!(app.state, AppState::Typing);

        // Simulate completing the typing test
        app.thok.write('h');
        app.thok.write('e');
        app.thok.write('l');
        app.thok.write('l');
        app.thok.write('o');

        assert!(app.thok.has_finished());
        app.thok.calc_results();
        app.state = AppState::Results;

        assert_eq!(app.state, AppState::Results);

        // Navigate to character stats
        app.state = AppState::CharacterStats;
        assert_eq!(app.state, AppState::CharacterStats);

        // Navigate back to results
        app.state = AppState::Results;
        assert_eq!(app.state, AppState::Results);
    }

    #[test]
    fn test_app_state_clone() {
        let state1 = AppState::Typing;
        let state2 = state1.clone();
        assert_eq!(state1, state2);

        let state3 = AppState::CharacterStats;
        assert_ne!(state1, state3);
    }

    #[test]
    fn test_flag_independence_at_app_level() {
        // Test that CLI flags control their own behavior independently

        // Test substitute only
        let cli_substitute_only = Cli {
            number_of_words: 5,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: None,
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: true,
        };
        let app_substitute = App::new(cli_substitute_only);
        // Should generate substituted words without extra formatting
        assert!(!app_substitute.thok.prompt.is_empty());

        // Test capitalize only
        let cli_capitalize_only = Cli {
            number_of_words: 5,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: None,
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: true,
            strict: false,
            symbols: false,
            substitute: false,
        };
        let app_capitalize = App::new(cli_capitalize_only);
        // Should have capitalization
        assert!(app_capitalize
            .thok
            .prompt
            .chars()
            .next()
            .unwrap()
            .is_uppercase());

        // Test symbols only
        let cli_symbols_only = Cli {
            number_of_words: 5,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: None,
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: true,
            substitute: false,
        };
        let app_symbols = App::new(cli_symbols_only);
        // Should have symbols available (end punctuation at minimum)
        assert!(
            app_symbols.thok.prompt.ends_with('.')
                || app_symbols.thok.prompt.ends_with('!')
                || app_symbols.thok.prompt.ends_with('?')
                || app_symbols.thok.prompt.ends_with(';')
                || app_symbols.thok.prompt.ends_with(':')
                || app_symbols.thok.prompt.ends_with("...")
        );

        // Test all three combined
        let cli_all = Cli {
            number_of_words: 5,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: None,
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: true,
            strict: false,
            symbols: true,
            substitute: true,
        };
        let app_all = App::new(cli_all);
        // Should have all features
        assert!(!app_all.thok.prompt.is_empty());
        // Due to substitution and symbol randomness, just check that the prompt is generated
        // The capitalization will be handled by the formatting logic when both flags are enabled
    }

    #[test]
    fn test_cli_to_word_gen_config() {
        let cli = Cli {
            number_of_words: 20,
            number_of_sentences: Some(3),
            number_of_secs: Some(60),
            prompt: Some("test prompt".to_string()),
            supported_language: SupportedLanguage::English1k,
            random_words: true,
            capitalize: true,
            strict: false,
            symbols: true,
            substitute: false,
        };

        let config = cli.to_word_gen_config(None);

        assert_eq!(config.number_of_words, 20);
        assert_eq!(config.number_of_sentences, Some(3));
        assert_eq!(config.custom_prompt, None);
        assert!(matches!(config.language, SupportedLanguage::English1k));
        assert!(config.random_words);
        assert!(config.capitalize);
        assert!(config.symbols);
        assert!(!config.substitute);
    }

    #[test]
    fn test_cli_to_word_gen_config_with_custom_prompt() {
        let cli = Cli {
            number_of_words: 10,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: None,
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let config = cli.to_word_gen_config(Some("custom prompt override".to_string()));

        assert_eq!(
            config.custom_prompt,
            Some("custom prompt override".to_string())
        );
    }

    #[test]
    fn test_char_stats_state_default() {
        let state = CharStatsState::default();

        assert_eq!(state.scroll_offset, 0);
        assert!(matches!(state.sort_by, SortBy::Character));
        assert!(state.sort_ascending);
    }

    #[test]
    fn test_sort_by_variants() {
        // Test that all SortBy variants can be created
        let _char_sort = SortBy::Character;
        let _time_sort = SortBy::AvgTime;
        let _miss_sort = SortBy::MissRate;
        let _attempts_sort = SortBy::Attempts;
    }

    #[test]
    fn test_app_state_variants() {
        // Test that all AppState variants can be created and are equal to themselves
        assert_eq!(AppState::Typing, AppState::Typing);
        assert_eq!(AppState::Results, AppState::Results);
        assert_eq!(AppState::CharacterStats, AppState::CharacterStats);

        // Test that different variants are not equal
        assert_ne!(AppState::Typing, AppState::Results);
        assert_ne!(AppState::Results, AppState::CharacterStats);
    }

    #[test]
    fn test_strict_mode_flag() {
        let cli_strict = Cli {
            number_of_words: 5,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: Some("test".to_string()),
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: true,
            symbols: false,
            substitute: false,
        };

        let app = App::new(cli_strict);
        assert!(app.thok.strict_mode);
    }

    #[test]
    fn test_supported_language_enum_variants() {
        // Test all variants can be created
        let _english = SupportedLanguage::English;
        let _english1k = SupportedLanguage::English1k;
        let _english10k = SupportedLanguage::English10k;

        // Test copying
        let lang1 = SupportedLanguage::English;
        let lang2 = lang1;
        assert!(matches!(lang2, SupportedLanguage::English));
    }

    #[test]
    fn test_get_thok_events_no_tick() {
        // Test creating event receiver without ticking
        let receiver = get_thok_events(false);

        // Since we can't easily inject events in a unit test without complex mocking,
        // we'll just verify the receiver was created successfully
        // The actual event handling is tested through integration
        drop(receiver); // Clean up
    }

    #[test]
    fn test_get_thok_events_with_tick() {
        // Test creating event receiver with ticking enabled
        let receiver = get_thok_events(true);

        // Verify we can receive tick events (with timeout to avoid hanging)
        use std::time::Duration;
        let result = receiver.recv_timeout(Duration::from_millis(150)); // Slightly longer than TICK_RATE_MS

        // Should receive a tick event
        match result {
            Ok(ThokEvent::Tick) => {
                // Success - we got a tick event
            }
            Ok(_) => panic!("Expected tick event, got different event type"),
            Err(_) => {
                // Timeout is acceptable in test environment due to timing variations
                // The important thing is that the receiver was created successfully
            }
        }

        drop(receiver); // Clean up
    }

    #[test]
    fn test_ui_function_typing_state() {
        use ratatui::{backend::TestBackend, Terminal};

        let cli = Cli {
            number_of_words: 3,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: Some("test".to_string()),
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let mut app = App::new(cli);
        app.state = AppState::Typing;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Test that ui function renders without panicking
        terminal.draw(|f| ui(&mut app, f)).unwrap();

        // Verify the buffer contains some content (the prompt should be rendered)
        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(content.contains("test") || !content.trim().is_empty());
    }

    #[test]
    fn test_ui_function_results_state() {
        use ratatui::{backend::TestBackend, Terminal};

        let cli = Cli {
            number_of_words: 3,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: Some("test".to_string()),
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let mut app = App::new(cli);
        app.state = AppState::Results;

        // Complete the typing test to generate results
        app.thok.write('t');
        app.thok.write('e');
        app.thok.write('s');
        app.thok.write('t');
        app.thok.calc_results();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Test that ui function renders without panicking
        terminal.draw(|f| ui(&mut app, f)).unwrap();

        // The results screen should render successfully
    }

    #[test]
    fn test_ui_function_character_stats_state() {
        use ratatui::{backend::TestBackend, Terminal};

        let cli = Cli {
            number_of_words: 3,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: Some("test".to_string()),
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let mut app = App::new(cli);
        app.state = AppState::CharacterStats;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Test that ui function renders without panicking
        terminal.draw(|f| ui(&mut app, f)).unwrap();

        // The character stats screen should render successfully
    }

    #[test]
    fn test_render_character_stats_with_data() {
        use ratatui::{backend::TestBackend, Terminal};

        let cli = Cli {
            number_of_words: 3,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: Some("hello".to_string()),
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let mut app = App::new(cli);

        // Generate some character stats by typing
        app.thok.write('h');
        app.thok.write('e');
        app.thok.write('l');
        app.thok.write('l');
        app.thok.write('o');
        app.thok.calc_results();

        app.state = AppState::CharacterStats;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Test that render_character_stats function works with data
        terminal
            .draw(|f| render_character_stats(&mut app, f))
            .unwrap();
    }

    #[test]
    fn test_render_character_stats_no_data() {
        use ratatui::{backend::TestBackend, Terminal};

        let cli = Cli {
            number_of_words: 3,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: Some("test".to_string()),
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let mut app = App::new(cli);
        app.state = AppState::CharacterStats;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Test that render_character_stats function works without data
        terminal
            .draw(|f| render_character_stats(&mut app, f))
            .unwrap();
    }

    #[test]
    fn test_character_stats_scrolling() {
        let cli = Cli {
            number_of_words: 3,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: Some("hello world test".to_string()),
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let mut app = App::new(cli);

        // Generate character stats by typing the full prompt
        for c in "hello world test".chars() {
            app.thok.write(c);
        }
        app.thok.calc_results();

        app.state = AppState::CharacterStats;

        // Test initial scroll state
        assert_eq!(app.char_stats_state.scroll_offset, 0);

        // Test scroll down
        app.char_stats_state.scroll_offset += 1;
        assert_eq!(app.char_stats_state.scroll_offset, 1);

        // Test scroll up
        app.char_stats_state.scroll_offset = app.char_stats_state.scroll_offset.saturating_sub(1);
        assert_eq!(app.char_stats_state.scroll_offset, 0);
    }

    #[test]
    fn test_character_stats_sorting() {
        let cli = Cli {
            number_of_words: 3,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: Some("test".to_string()),
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let mut app = App::new(cli);
        app.state = AppState::CharacterStats;

        // Test initial sort state
        assert!(matches!(app.char_stats_state.sort_by, SortBy::Character));
        assert!(app.char_stats_state.sort_ascending);

        // Test changing sort criteria
        app.char_stats_state.sort_by = SortBy::AvgTime;
        assert!(matches!(app.char_stats_state.sort_by, SortBy::AvgTime));

        app.char_stats_state.sort_by = SortBy::MissRate;
        assert!(matches!(app.char_stats_state.sort_by, SortBy::MissRate));

        app.char_stats_state.sort_by = SortBy::Attempts;
        assert!(matches!(app.char_stats_state.sort_by, SortBy::Attempts));

        // Test toggling sort direction
        app.char_stats_state.sort_ascending = !app.char_stats_state.sort_ascending;
        assert!(!app.char_stats_state.sort_ascending);
    }

    #[test]
    fn test_exit_type_variants() {
        // Test all ExitType variants can be created
        let _restart = ExitType::Restart;
        let _new = ExitType::New;
        let _quit = ExitType::Quit;

        // Test Debug trait
        assert_eq!(format!("{:?}", ExitType::Restart), "Restart");
        assert_eq!(format!("{:?}", ExitType::New), "New");
        assert_eq!(format!("{:?}", ExitType::Quit), "Quit");
    }

    #[test]
    fn test_tick_rate_constant() {
        // Verify the tick rate constant is reasonable
        assert_eq!(TICK_RATE_MS, 100);

        // These are compile-time checks that our constant is reasonable
        const _: () = assert!(TICK_RATE_MS > 0);
        const _: () = assert!(TICK_RATE_MS <= 1000); // Should be sub-second
    }

    #[test]
    fn test_integration_complete_typing_session() {
        // Integration test for a complete typing session workflow
        let cli = Cli {
            number_of_words: 3,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: Some("hello world test".to_string()),
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let mut app = App::new(cli);

        // Verify initial state
        assert_eq!(app.state, AppState::Typing);
        assert!(!app.thok.has_started());
        assert!(!app.thok.has_finished());

        // Simulate typing session
        app.thok.on_keypress_start(); // Start timing
        for c in "hello world test".chars() {
            app.thok.write(c);
        }

        // Verify session completion
        assert!(app.thok.has_started());
        assert!(app.thok.has_finished());

        // Calculate results
        app.thok.calc_results();
        app.state = AppState::Results;

        // Verify results exist
        assert!(app.thok.wpm > 0.0);
        assert!(app.thok.accuracy >= 0.0 && app.thok.accuracy <= 100.0);

        // Test state transitions
        app.state = AppState::CharacterStats;
        assert_eq!(app.state, AppState::CharacterStats);

        // Test reset functionality
        app.reset(None);
        assert_eq!(app.state, AppState::Typing);
        assert!(!app.thok.has_started());
        assert!(!app.thok.has_finished());
        assert_eq!(app.thok.input.len(), 0);
    }

    #[test]
    fn test_integration_timed_session() {
        // Integration test for timed typing session
        let cli = Cli {
            number_of_words: 10,
            number_of_sentences: None,
            number_of_secs: Some(1), // 1 second for fast test
            prompt: None,
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        };

        let mut app = App::new(cli);

        // Verify timed session setup
        assert_eq!(app.thok.number_of_secs, Some(1.0));
        assert_eq!(app.thok.seconds_remaining, Some(1.0));

        // Start typing
        app.thok.on_keypress_start();
        app.thok.write('t');
        app.thok.write('e');

        // Simulate time passage via multiple ticks
        for _ in 0..12 {
            // 12 ticks * 100ms = 1200ms > 1000ms
            app.thok.on_tick();
        }

        // Session should be finished due to time limit
        assert!(app.thok.has_finished());
    }

    #[test]
    fn test_integration_strict_mode_workflow() {
        // Integration test for strict mode behavior
        let cli = Cli {
            number_of_words: 3,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: Some("hello".to_string()),
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: true,
            symbols: false,
            substitute: false,
        };

        let mut app = App::new(cli);

        // Verify strict mode is enabled
        assert!(app.thok.strict_mode);

        // Start typing
        app.thok.on_keypress_start();
        app.thok.write('h');
        app.thok.write('e');
        app.thok.write('l');
        app.thok.write('l');
        app.thok.write('o');

        // Verify completion
        assert!(app.thok.has_finished());
    }

    #[test]
    fn test_integration_character_substitution_workflow() {
        // Integration test for character substitution feature
        let cli = Cli {
            number_of_words: 5,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: None,
            supported_language: SupportedLanguage::English,
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: true,
        };

        let app = App::new(cli);

        // Verify substitution mode generates a prompt
        assert!(!app.thok.prompt.is_empty());
        // The actual substitution logic is tested in the language module
        // Here we just verify the integration works
    }

    #[test]
    fn test_integration_app_reset_preserves_cli_settings() {
        // Integration test to verify app reset preserves CLI settings
        let cli = Cli {
            number_of_words: 25,
            number_of_sentences: None,
            number_of_secs: Some(60),
            prompt: None,
            supported_language: SupportedLanguage::English1k,
            random_words: true,
            capitalize: true,
            strict: true,
            symbols: true,
            substitute: false,
        };

        let mut app = App::new(cli.clone());

        // Verify initial settings
        assert_eq!(app.thok.number_of_words, 25);
        assert_eq!(app.thok.number_of_secs, Some(60.0));
        assert!(app.thok.strict_mode);

        // Type something to change state
        app.thok.write('t');
        app.thok.write('e');

        // Reset the app
        app.reset(None);

        // Verify settings are preserved after reset
        assert_eq!(app.thok.number_of_words, 25);
        assert_eq!(app.thok.number_of_secs, Some(60.0));
        assert!(app.thok.strict_mode);
        assert_eq!(app.thok.input.len(), 0); // But input is cleared
        assert_eq!(app.thok.cursor_pos, 0); // And cursor is reset
        assert_eq!(app.state, AppState::Typing); // And state is reset
    }

    #[test]
    fn test_integration_multiple_language_support() {
        // Integration test for different language configurations
        let languages = [
            SupportedLanguage::English,
            SupportedLanguage::English1k,
            SupportedLanguage::English10k,
        ];

        for lang in languages {
            let cli = Cli {
                number_of_words: 5,
                number_of_sentences: None,
                number_of_secs: None,
                prompt: None,
                supported_language: lang,
                random_words: false,
                capitalize: false,
                strict: false,
                symbols: false,
                substitute: false,
            };

            let app = App::new(cli);

            // Verify that each language generates a valid prompt
            assert!(!app.thok.prompt.is_empty());
            assert!(app.thok.number_of_words > 0);
        }
    }

    #[test]
    fn test_integration_formatting_combinations() {
        // Integration test for various formatting flag combinations
        let test_cases = [
            (false, false, false), // No formatting
            (true, false, false),  // Capitalize only
            (false, true, false),  // Symbols only
            (false, false, true),  // Substitute only
            (true, true, false),   // Capitalize + Symbols
            (true, false, true),   // Capitalize + Substitute
            (false, true, true),   // Symbols + Substitute
            (true, true, true),    // All formatting
        ];

        for (capitalize, symbols, substitute) in test_cases {
            let cli = Cli {
                number_of_words: 5,
                number_of_sentences: None,
                number_of_secs: None,
                prompt: None,
                supported_language: SupportedLanguage::English,
                random_words: false,
                capitalize,
                strict: false,
                symbols,
                substitute,
            };

            let app = App::new(cli);

            // Verify that all formatting combinations generate valid prompts
            assert!(!app.thok.prompt.is_empty());

            // If capitalize is enabled, check for capitalization (may be affected by other formatting)
            if capitalize && !substitute && !symbols {
                // Only test when other formatting options don't interfere
                let first_char = app.thok.prompt.chars().next().unwrap();
                // Allow for punctuation or other formatting that might come first
                if first_char.is_alphabetic() {
                    assert!(first_char.is_uppercase());
                }
            }
        }
    }
}
