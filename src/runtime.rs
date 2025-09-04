use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

use crossterm::event::{self, Event as CtEvent, KeyEvent};

/// Unified event type consumed by the app runner
#[derive(Clone, Debug)]
pub enum ThokEvent {
    Key(KeyEvent),
    Resize,
    Tick,
}

/// Source of terminal events (keyboard, resize, etc.)
pub trait ThokEventSource: Send + 'static {
    /// Block for up to `timeout` waiting for an event.
    /// Returns Ok(event) if an event arrives before the timeout, or Err(Timeout) if it expires.
    fn recv_timeout(&self, timeout: Duration) -> Result<ThokEvent, RecvTimeoutError>;
}

/// Production event source using crossterm
pub struct CrosstermEventSource {
    rx: Receiver<ThokEvent>,
}

impl CrosstermEventSource {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || loop {
            match event::read() {
                Ok(CtEvent::Key(key)) => {
                    if tx.send(ThokEvent::Key(key)).is_err() {
                        break;
                    }
                }
                Ok(CtEvent::Resize(_, _)) => {
                    if tx.send(ThokEvent::Resize).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        });

        Self { rx }
    }
}

impl Default for CrosstermEventSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ThokEventSource for CrosstermEventSource {
    fn recv_timeout(&self, timeout: Duration) -> Result<ThokEvent, RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }
}

/// Configurable ticker interface
pub trait Ticker: Send + Sync + 'static {
    fn interval(&self) -> Duration;
}

/// Fixed interval ticker
#[derive(Clone, Copy, Debug)]
pub struct FixedTicker {
    interval: Duration,
}

impl FixedTicker {
    pub fn new(interval: Duration) -> Self {
        Self { interval }
    }
}

impl Ticker for FixedTicker {
    fn interval(&self) -> Duration {
        self.interval
    }
}

/// Test event source for unit tests
pub struct TestEventSource {
    rx: Receiver<ThokEvent>,
}

impl TestEventSource {
    pub fn new(rx: Receiver<ThokEvent>) -> Self {
        Self { rx }
    }
}

impl ThokEventSource for TestEventSource {
    fn recv_timeout(&self, timeout: Duration) -> Result<ThokEvent, RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }
}

/// Runner that advances the application one event/tick at a time
pub struct Runner<E: ThokEventSource, T: Ticker> {
    event_source: E,
    ticker: T,
}

impl<E: ThokEventSource, T: Ticker> Runner<E, T> {
    pub fn new(event_source: E, ticker: T) -> Self {
        Self {
            event_source,
            ticker,
        }
    }

    /// Blocks up to tick interval and returns the next event, or Tick on timeout
    pub fn step(&self) -> ThokEvent {
        match self.event_source.recv_timeout(self.ticker.interval()) {
            Ok(ev) => ev,
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => ThokEvent::Tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn step_returns_tick_on_timeout() {
        let (_tx, rx) = mpsc::channel();
        let es = TestEventSource::new(rx);
        let ticker = FixedTicker::new(Duration::from_millis(1));
        let runner = Runner::new(es, ticker);

        // With no events available, step should yield Tick
        let ev = runner.step();
        match ev {
            ThokEvent::Tick => {}
            _ => panic!("expected Tick on timeout"),
        }
    }

    #[test]
    fn step_passes_through_events() {
        let (tx, rx) = mpsc::channel();
        tx.send(ThokEvent::Resize).unwrap();
        let es = TestEventSource::new(rx);
        let ticker = FixedTicker::new(Duration::from_millis(10));
        let runner = Runner::new(es, ticker);

        match runner.step() {
            ThokEvent::Resize => {}
            _ => panic!("expected Resize event"),
        }
    }
}
