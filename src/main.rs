pub mod lang;
pub mod stats;
pub mod thok;
pub mod ui;
pub mod util;

use crate::{lang::Language, thok::Thok};
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
#[clap(version, about, long_about= "A sleek typing TUI with intelligent word selection that adapts to your weaknesses, detailed performance analytics, and historical progress tracking.")]
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
        let mut count = 0;
        let prompt = if cli.prompt.is_some() {
            cli.prompt.clone().unwrap()
        } else if cli.number_of_sentences.is_some() {
            let language = cli.supported_language.as_lang();
            let (s, count_tmp) = language.get_random_sentence(cli.number_of_sentences.unwrap());
            count = count_tmp;
            // sets the word count for the sentence.
            s.join("")
        } else {
            let language = cli.supported_language.as_lang();
            
            if cli.random_words {
                // Legacy random word selection
                language.get_random(cli.number_of_words).join(" ")
            } else {
                // Intelligent word selection based on character statistics
                use crate::stats::StatsDb;
                let words = if let Ok(stats_db) = StatsDb::new() {
                    if let Ok(char_difficulties) = stats_db.get_character_difficulties() {
                        language.get_intelligent(cli.number_of_words, &char_difficulties)
                    } else {
                        // Fall back to random if stats query fails
                        language.get_random(cli.number_of_words)
                    }
                } else {
                    // Fall back to random if database unavailable
                    language.get_random(cli.number_of_words)
                };
                words.join(" ")
            }
        };
        if cli.number_of_sentences.is_some() {
            Self {
                thok: Thok::new(prompt, count, cli.number_of_secs.map(|ns| ns as f64)),
                cli: Some(cli),
                state: AppState::Typing,
                char_stats_state: CharStatsState::default(),
            }
        } else {
            Self {
                thok: Thok::new(
                    prompt,
                    cli.number_of_words,
                    cli.number_of_secs.map(|ns| ns as f64),
                ),
                cli: Some(cli),
                state: AppState::Typing,
                char_stats_state: CharStatsState::default(),
            }
        }
    }

    pub fn reset(&mut self, new_prompt: Option<String>) {
        let cli = self.cli.clone().unwrap();
        let mut count = 0;
        let prompt = match new_prompt {
            Some(_) => new_prompt.unwrap(),
            _ => match cli.number_of_sentences {
                Some(t) => {
                    let language = cli.supported_language.as_lang();
                    let (s, count_tmp) = language.get_random_sentence(t);
                    count = count_tmp;
                    // sets the word count for the sentence
                    s.join("")
                }
                _ => {
                    let language = cli.supported_language.as_lang();
                    
                    if cli.random_words {
                        // Legacy random word selection
                        language.get_random(cli.number_of_words).join(" ")
                    } else {
                        // Intelligent word selection based on character statistics
                        use crate::stats::StatsDb;
                        let words = if let Ok(stats_db) = StatsDb::new() {
                            if let Ok(char_difficulties) = stats_db.get_character_difficulties() {
                                language.get_intelligent(cli.number_of_words, &char_difficulties)
                            } else {
                                // Fall back to random if stats query fails
                                language.get_random(cli.number_of_words)
                            }
                        } else {
                            // Fall back to random if database unavailable
                            language.get_random(cli.number_of_words)
                        };
                        words.join(" ")
                    }
                }
            },
        };
        if cli.number_of_sentences.is_some() {
            self.thok = Thok::new(prompt, count, cli.number_of_secs.map(|ns| ns as f64));
        } else {
            self.thok = Thok::new(
                prompt,
                cli.number_of_words,
                cli.number_of_secs.map(|ns| ns as f64),
            );
        }
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
    let cli = app.cli.clone();

    let should_tick = cli.unwrap().number_of_secs.unwrap_or(0) > 0;

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
                            app.state = AppState::Results;
                        }
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
                                        app.thok.on_keypress_start();
                                        app.thok.write(c);
                                        if app.thok.has_finished() {
                                            app.thok.calc_results();
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
                                        app.char_stats_state.sort_ascending = !app.char_stats_state.sort_ascending;
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
        widgets::{Block, Borders, Paragraph, Table, Row, Cell},
    };

    let area = f.area();
    
    // Create layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(0),     // Stats table
            Constraint::Length(4),  // Instructions
        ])
        .split(area);

    // Title with sort indicator
    let sort_direction = if app.char_stats_state.sort_ascending { "↑" } else { "↓" };
    let sort_by_text = match app.char_stats_state.sort_by {
        SortBy::Character => "Character",
        SortBy::AvgTime => "Avg Time",
        SortBy::MissRate => "Miss Rate",
        SortBy::Attempts => "Attempts",
    };
    let title_text = format!("Character Statistics (Sort: {} {})", sort_by_text, sort_direction);
    
    let title = Paragraph::new(title_text)
        .block(Block::default().borders(Borders::ALL).title("Stats"))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Get character statistics
    if let Some(mut summary) = app.thok.get_all_char_summary() {
        // Sort the data based on current sort criteria
        match app.char_stats_state.sort_by {
            SortBy::Character => {
                summary.sort_by(|a, b| {
                    let cmp = a.0.cmp(&b.0);
                    if app.char_stats_state.sort_ascending { cmp } else { cmp.reverse() }
                });
            }
            SortBy::AvgTime => {
                summary.sort_by(|a, b| {
                    let cmp = a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal);
                    if app.char_stats_state.sort_ascending { cmp } else { cmp.reverse() }
                });
            }
            SortBy::MissRate => {
                summary.sort_by(|a, b| {
                    let cmp = a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal);
                    if app.char_stats_state.sort_ascending { cmp } else { cmp.reverse() }
                });
            }
            SortBy::Attempts => {
                summary.sort_by(|a, b| {
                    let cmp = a.3.cmp(&b.3);
                    if app.char_stats_state.sort_ascending { cmp } else { cmp.reverse() }
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
        let char_indicator = if matches!(app.char_stats_state.sort_by, SortBy::Character) { sort_direction } else { "" };
        let time_indicator = if matches!(app.char_stats_state.sort_by, SortBy::AvgTime) { sort_direction } else { "" };
        let miss_indicator = if matches!(app.char_stats_state.sort_by, SortBy::MissRate) { sort_direction } else { "" };
        let attempts_indicator = if matches!(app.char_stats_state.sort_by, SortBy::Attempts) { sort_direction } else { "" };

        let header = Row::new(vec![
            Cell::from(format!("Char {}", char_indicator)),
            Cell::from(format!("Avg Time (ms) {}", time_indicator)),
            Cell::from(format!("Miss Rate (%) {}", miss_indicator)),
            Cell::from(format!("Attempts {}", attempts_indicator)),
        ])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        // Get visible rows based on scroll offset
        let visible_rows: Vec<Row> = summary
            .iter()
            .skip(app.char_stats_state.scroll_offset)
            .take(table_height)
            .map(|(character, avg_time, miss_rate, attempts)| {
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

                Row::new(vec![
                    Cell::from(char_display),
                    Cell::from(format!("{:.1}", avg_time)).style(Style::default().fg(time_color)),
                    Cell::from(format!("{:.1}", miss_rate)).style(Style::default().fg(miss_color)),
                    Cell::from(attempts.to_string()),
                ])
            })
            .collect();

        // Show scroll position in title if there are more rows than visible
        let scroll_info = if total_rows > table_height {
            format!(" ({}/{} rows)", app.char_stats_state.scroll_offset + visible_rows.len().min(table_height), total_rows)
        } else {
            String::new()
        };

        let table = Table::new(visible_rows, &[
            Constraint::Length(8),
            Constraint::Length(18),
            Constraint::Length(18),
            Constraint::Length(12),
        ])
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(format!("Performance by Character{}", scroll_info)))
        .row_highlight_style(Style::default().bg(Color::DarkGray));

        f.render_widget(table, chunks[1]);
    } else {
        let no_data = Paragraph::new("No character statistics available.\nComplete a typing test to see your stats!")
            .block(Block::default().borders(Borders::ALL).title("No Data"))
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(no_data, chunks[1]);
    }

    // Instructions
    let instructions = Paragraph::new("Sort: (1)Char (2)Time (3)Miss (4)Attempts | (Space)Toggle direction\nScroll: ↑/↓ PgUp/PgDn Home | (b)ack (r)etry (n)ew (esc)ape")
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
        let cli = Cli::parse_from(&["thokr"]);

        assert_eq!(cli.number_of_words, 15);
        assert_eq!(cli.number_of_sentences, None);
        assert_eq!(cli.number_of_secs, None);
        assert_eq!(cli.prompt, None);
        assert!(matches!(cli.supported_language, SupportedLanguage::English));
    }

    #[test]
    fn test_cli_number_of_words() {
        let cli = Cli::parse_from(&["thokr", "-w", "25"]);
        assert_eq!(cli.number_of_words, 25);

        let cli = Cli::parse_from(&["thokr", "--number-of-words", "50"]);
        assert_eq!(cli.number_of_words, 50);
    }

    #[test]
    fn test_cli_number_of_sentences() {
        let cli = Cli::parse_from(&["thokr", "-f", "3"]);
        assert_eq!(cli.number_of_sentences, Some(3));

        let cli = Cli::parse_from(&["thokr", "--full-sentences", "5"]);
        assert_eq!(cli.number_of_sentences, Some(5));
    }

    #[test]
    fn test_cli_number_of_secs() {
        let cli = Cli::parse_from(&["thokr", "-s", "60"]);
        assert_eq!(cli.number_of_secs, Some(60));

        let cli = Cli::parse_from(&["thokr", "--number-of-secs", "120"]);
        assert_eq!(cli.number_of_secs, Some(120));
    }

    #[test]
    fn test_cli_custom_prompt() {
        let cli = Cli::parse_from(&["thokr", "-p", "hello world"]);
        assert_eq!(cli.prompt, Some("hello world".to_string()));

        let cli = Cli::parse_from(&["thokr", "--prompt", "custom text"]);
        assert_eq!(cli.prompt, Some("custom text".to_string()));
    }

    #[test]
    fn test_cli_supported_language() {
        let cli = Cli::parse_from(&["thokr", "-l", "english"]);
        assert!(matches!(cli.supported_language, SupportedLanguage::English));

        let cli = Cli::parse_from(&["thokr", "--supported-language", "english1k"]);
        assert!(matches!(
            cli.supported_language,
            SupportedLanguage::English1k
        ));

        let cli = Cli::parse_from(&["thokr", "--supported-language", "english10k"]);
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
}
