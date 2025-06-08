pub mod lang;
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

/// sleek typing tui with visualized results and historical logging
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about= None)]
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

#[derive(Debug)]
pub struct App {
    pub cli: Option<Cli>,
    pub thok: Thok,
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

            language.get_random(cli.number_of_words).join(" ")
        };
        if cli.number_of_sentences.is_some() {
            Self {
                thok: Thok::new(prompt, count, cli.number_of_secs.map(|ns| ns as f64)),
                cli: Some(cli),
            }
        } else {
            Self {
                thok: Thok::new(
                    prompt,
                    cli.number_of_words,
                    cli.number_of_secs.map(|ns| ns as f64),
                ),
                cli: Some(cli),
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
                    language.get_random(cli.number_of_words).join(" ")
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
                            if !app.thok.has_finished() {
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

                            match app.thok.has_finished() {
                                false => {
                                    app.thok.write(c);
                                    if app.thok.has_finished() {
                                        app.thok.calc_results();
                                    }
                                }
                                true => match key.code {
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

fn ui(app: &mut App, f: &mut Frame) {
    f.render_widget(&app.thok, f.area());
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
        };

        let app = App::new(cli.clone());

        assert_eq!(app.thok.number_of_words, 10);
        assert_eq!(app.thok.number_of_secs, None);
        assert!(app.cli.is_some());
        assert!(!app.thok.prompt.is_empty());
    }

    #[test]
    fn test_app_new_with_custom_prompt() {
        let cli = Cli {
            number_of_words: 10,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: Some("custom test prompt".to_string()),
            supported_language: SupportedLanguage::English,
        };

        let app = App::new(cli);

        assert_eq!(app.thok.prompt, "custom test prompt");
        assert_eq!(app.thok.number_of_words, 10);
    }

    #[test]
    fn test_app_new_with_sentences() {
        let cli = Cli {
            number_of_words: 10,
            number_of_sentences: Some(2),
            number_of_secs: None,
            prompt: None,
            supported_language: SupportedLanguage::English,
        };

        let app = App::new(cli);

        assert!(app.thok.number_of_words > 0);
        assert!(!app.thok.prompt.is_empty());
    }

    #[test]
    fn test_app_new_with_time_limit() {
        let cli = Cli {
            number_of_words: 10,
            number_of_sentences: None,
            number_of_secs: Some(60),
            prompt: None,
            supported_language: SupportedLanguage::English,
        };

        let app = App::new(cli);

        assert_eq!(app.thok.number_of_secs, Some(60.0));
        assert_eq!(app.thok.seconds_remaining, Some(60.0));
    }

    #[test]
    fn test_app_reset_with_new_prompt() {
        let cli = Cli {
            number_of_words: 5,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: None,
            supported_language: SupportedLanguage::English,
        };

        let mut app = App::new(cli);
        let original_prompt = app.thok.prompt.clone();

        app.reset(Some("new test prompt".to_string()));

        assert_eq!(app.thok.prompt, "new test prompt");
        assert_ne!(app.thok.prompt, original_prompt);
        assert_eq!(app.thok.input.len(), 0);
        assert_eq!(app.thok.cursor_pos, 0);
    }

    #[test]
    fn test_app_reset_without_new_prompt() {
        let cli = Cli {
            number_of_words: 5,
            number_of_sentences: None,
            number_of_secs: None,
            prompt: None,
            supported_language: SupportedLanguage::English,
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
}
