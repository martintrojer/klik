use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;

use crate::{ui::character_stats::render_character_stats, App, AppState};

/// A UI Screen boundary: responsible for rendering and optional key handling
pub trait Screen {
    fn render(&self, app: &mut App, f: &mut Frame);
    /// Optional per-screen key handling. Return an action if handled.
    fn on_key(&mut self, _key: KeyEvent, _app: &mut App) -> Option<KeyAction> {
        None
    }
}

/// Typing screen - renders the typing UI using the existing App widget
pub struct TypingScreen;

impl Screen for TypingScreen {
    fn render(&self, app: &mut App, f: &mut Frame) {
        f.render_widget(&*app, f.area());
    }

    fn on_key(&mut self, key: KeyEvent, app: &mut App) -> Option<KeyAction> {
        match key.code {
            KeyCode::Backspace => {
                if !app.thok.has_finished() {
                    app.thok.backspace();
                }
                Some(KeyAction::Continue)
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                    return Some(KeyAction::Quit);
                }
                if !app.thok.has_finished() {
                    app.thok.write(c);
                }
                Some(KeyAction::Continue)
            }
            _ => None,
        }
    }
}

/// Results screen - renders the results UI using the existing App widget
pub struct ResultsScreen;

impl Screen for ResultsScreen {
    fn render(&self, app: &mut App, f: &mut Frame) {
        f.render_widget(&*app, f.area());
    }

    fn on_key(&mut self, key: KeyEvent, app: &mut App) -> Option<KeyAction> {
        match key.code {
            KeyCode::Char('t') => {
                if webbrowser::Browser::is_available() {
                    // Construct a minimal encoded tweet without external deps.
                    // Format: "<wpm> wpm / <acc>% acc / <sd> sd\n\nhttps://github.com/martintrojer/klik"
                    let wpm = app.thok.wpm();
                    let acc = app.thok.accuracy();
                    let sd = app.thok.std_dev();
                    let url = format!(
                        "https://twitter.com/intent/tweet?text={wpm}%20wpm%20%2F%20{acc}%25%20acc%20%2F%20{sd:.2}%20sd%0A%0Ahttps%3A%2F%2Fgithub.com%2Fmartintrojer%2Fklik"
                    );
                    let _ = webbrowser::open(&url);
                }
                Some(KeyAction::Continue)
            }
            KeyCode::Char('r') => Some(KeyAction::Restart),
            KeyCode::Char('n') => Some(KeyAction::New),
            KeyCode::Char('s') => {
                app.state = AppState::CharacterStats;
                Some(KeyAction::Continue)
            }
            // Settings toggles
            KeyCode::Char('1') => {
                app.runtime_settings.random_words = !app.runtime_settings.random_words;
                Some(KeyAction::Continue)
            }
            KeyCode::Char('2') => {
                app.runtime_settings.capitalize = !app.runtime_settings.capitalize;
                Some(KeyAction::Continue)
            }
            KeyCode::Char('3') => {
                app.runtime_settings.strict = !app.runtime_settings.strict;
                Some(KeyAction::Continue)
            }
            KeyCode::Char('4') => {
                app.runtime_settings.symbols = !app.runtime_settings.symbols;
                Some(KeyAction::Continue)
            }
            KeyCode::Char('5') => {
                app.runtime_settings.substitute = !app.runtime_settings.substitute;
                Some(KeyAction::Continue)
            }
            KeyCode::Char('w') => {
                app.runtime_settings.number_of_words = match app.runtime_settings.number_of_words {
                    15 => 25,
                    25 => 50,
                    50 => 100,
                    _ => 15,
                };
                Some(KeyAction::Continue)
            }
            KeyCode::Char('l') => {
                app.runtime_settings.supported_language =
                    match app.runtime_settings.supported_language {
                        crate::SupportedLanguage::English => crate::SupportedLanguage::English1k,
                        crate::SupportedLanguage::English1k => crate::SupportedLanguage::English10k,
                        crate::SupportedLanguage::English10k => crate::SupportedLanguage::English,
                    };
                Some(KeyAction::Continue)
            }
            _ => None,
        }
    }
}

/// Character stats screen - uses dedicated renderer
pub struct CharacterStatsScreen;

impl Screen for CharacterStatsScreen {
    fn render(&self, app: &mut App, f: &mut Frame) {
        // Delegate to the extracted function
        render_character_stats(app, f);
    }

    fn on_key(&mut self, key: KeyEvent, app: &mut App) -> Option<KeyAction> {
        match key.code {
            KeyCode::Char('r') => Some(KeyAction::Restart),
            KeyCode::Char('n') => Some(KeyAction::New),
            KeyCode::Char('b') | KeyCode::Backspace => {
                app.state = AppState::Results;
                Some(KeyAction::Continue)
            }
            KeyCode::Up => {
                if app.char_stats_state.scroll_offset > 0 {
                    app.char_stats_state.scroll_offset -= 1;
                }
                Some(KeyAction::Continue)
            }
            KeyCode::Down => {
                app.char_stats_state.scroll_offset += 1;
                Some(KeyAction::Continue)
            }
            KeyCode::PageUp => {
                app.char_stats_state.scroll_offset =
                    app.char_stats_state.scroll_offset.saturating_sub(10);
                Some(KeyAction::Continue)
            }
            KeyCode::PageDown => {
                app.char_stats_state.scroll_offset += 10;
                Some(KeyAction::Continue)
            }
            KeyCode::Home => {
                app.char_stats_state.scroll_offset = 0;
                Some(KeyAction::Continue)
            }
            KeyCode::Char('1') => {
                app.char_stats_state.sort_by = crate::SortBy::Character;
                app.char_stats_state.scroll_offset = 0;
                Some(KeyAction::Continue)
            }
            KeyCode::Char('2') => {
                app.char_stats_state.sort_by = crate::SortBy::AvgTime;
                app.char_stats_state.scroll_offset = 0;
                Some(KeyAction::Continue)
            }
            KeyCode::Char('3') => {
                app.char_stats_state.sort_by = crate::SortBy::MissRate;
                app.char_stats_state.scroll_offset = 0;
                Some(KeyAction::Continue)
            }
            KeyCode::Char('4') => {
                app.char_stats_state.sort_by = crate::SortBy::Attempts;
                app.char_stats_state.scroll_offset = 0;
                Some(KeyAction::Continue)
            }
            KeyCode::Char(' ') => {
                app.char_stats_state.sort_ascending = !app.char_stats_state.sort_ascending;
                app.char_stats_state.scroll_offset = 0;
                Some(KeyAction::Continue)
            }
            _ => None,
        }
    }
}

/// Helper to construct the appropriate screen for the current state
pub fn current_screen(state: &AppState) -> Box<dyn Screen> {
    match state {
        AppState::Typing => Box::new(TypingScreen),
        AppState::Results => Box::new(ResultsScreen),
        AppState::CharacterStats => Box::new(CharacterStatsScreen),
    }
}

/// Actions a screen can request from the main loop
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Continue,
    Restart,
    New,
    Quit,
}
