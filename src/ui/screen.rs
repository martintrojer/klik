use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::{ui::character_stats::render_character_stats, App, AppState};

/// A UI Screen boundary: responsible for rendering and optional key handling
pub trait Screen {
    fn render(&self, app: &mut App, f: &mut Frame);
    /// Optional per-screen key handling. Returns true if the key was handled.
    fn on_key(&mut self, _key: KeyEvent, _app: &mut App) -> bool {
        false
    }
}

/// Typing screen - renders the typing UI using the existing App widget
pub struct TypingScreen;

impl Screen for TypingScreen {
    fn render(&self, app: &mut App, f: &mut Frame) {
        f.render_widget(&*app, f.area());
    }
}

/// Results screen - renders the results UI using the existing App widget
pub struct ResultsScreen;

impl Screen for ResultsScreen {
    fn render(&self, app: &mut App, f: &mut Frame) {
        f.render_widget(&*app, f.area());
    }
}

/// Character stats screen - uses dedicated renderer
pub struct CharacterStatsScreen;

impl Screen for CharacterStatsScreen {
    fn render(&self, app: &mut App, f: &mut Frame) {
        // Delegate to the extracted function
        render_character_stats(app, f);
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
