use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// Headless integration using the internal runtime + Thok without a TTY
// Verifies that a minimal typing flow completes via Runner/TestEventSource.
#[test]
fn headless_typing_flow_completes() {
    // Arrange: build a Thok with a simple prompt
    let mut thok = klik::thok::Thok::new("hi".to_string(), 1, None, false);

    // Channel for the test event source
    let (tx, rx) = mpsc::channel();

    // Create TestEventSource and Runner with a small tick interval
    let es = klik::runtime::TestEventSource::new(rx);
    let ticker = klik::runtime::FixedTicker::new(Duration::from_millis(5));
    let runner = klik::runtime::Runner::new(es, ticker);

    // Producer: send the keystrokes for the prompt
    tx.send(klik::runtime::ThokEvent::Key(KeyEvent::new(
        KeyCode::Char('h'),
        KeyModifiers::NONE,
    )))
    .unwrap();
    tx.send(klik::runtime::ThokEvent::Key(KeyEvent::new(
        KeyCode::Char('i'),
        KeyModifiers::NONE,
    )))
    .unwrap();

    // Act: drive a tiny event loop until finished (or bounded steps)
    for _ in 0..100u32 {
        match runner.step() {
            klik::runtime::ThokEvent::Tick => thok.on_tick(),
            klik::runtime::ThokEvent::Resize => {}
            klik::runtime::ThokEvent::Key(key) => {
                if let KeyCode::Char(c) = key.code {
                    thok.write(c);
                    if thok.has_finished() {
                        break;
                    }
                }
            }
        }
    }

    // Assert: finished and results computable
    assert!(thok.has_finished(), "thok should have finished typing");
    thok.calc_results();
    assert!(thok.session_state.wpm >= 0.0);
    assert!(thok.session_state.accuracy >= 0.0);
}

#[test]
fn headless_strict_mode_flow() {
    // Strict mode: wrong char should not advance cursor
    let mut thok = klik::thok::Thok::new("ab".to_string(), 1, None, true);

    // Type wrong char first
    thok.write('x');
    assert_eq!(thok.session_state.cursor_pos, 0);

    // Type correct sequence
    thok.write('a');
    assert_eq!(thok.session_state.cursor_pos, 1);
    thok.write('b');
    assert!(thok.has_finished());
}

#[test]
fn headless_timed_session_finishes_by_time() {
    // Timed session: tick until time elapses
    let mut thok = klik::thok::Thok::new("hello".to_string(), 1, Some(0.2), false); // ~200ms

    let (_tx, rx) = std::sync::mpsc::channel();
    let es = klik::runtime::TestEventSource::new(rx);
    let ticker = klik::runtime::FixedTicker::new(Duration::from_millis(10));
    let runner = klik::runtime::Runner::new(es, ticker);

    for _ in 0..50u32 {
        // up to ~500ms
        if let klik::runtime::ThokEvent::Tick = runner.step() {
            thok.on_tick();
        }
        if thok.has_finished() {
            break;
        }
    }

    assert!(
        thok.has_finished(),
        "timed session should finish by timeout"
    );
}
