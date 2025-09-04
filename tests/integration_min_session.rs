// Minimal integration test that drives the compiled binary through a PTY.
// This exercises the real event loop and crossterm input handling across
// the main boundaries without relying on internal modules.
//
// Notes:
// - Requires a TTY; uses expectrl which allocates a pseudo terminal.
// - Marked Unix-only and ignored by default to avoid CI/platform issues.
// - Run manually via: `cargo test --test integration_min_session -- --ignored`.

#![cfg(unix)]

use std::time::Duration;

use expectrl::{spawn, Eof};

#[test]
#[ignore]
fn minimal_session_completes_and_exits() -> Result<(), Box<dyn std::error::Error>> {
    // Resolve path to compiled binary (debug build during tests)
    let bin = assert_cmd::cargo::cargo_bin("klik");
    let cmd = format!("{} -p hi", bin.display());

    // Spawn the TUI inside a pseudo terminal
    let mut p = spawn(cmd)?;

    // Give the app a moment to initialize the terminal/alternate screen
    std::thread::sleep(Duration::from_millis(200));

    // Type the custom prompt characters to finish the minimal session
    p.send("hi")?;

    // Small delay to allow processing and results transition
    std::thread::sleep(Duration::from_millis(200));

    // Send ESC to exit from the app (handled in both typing and results states)
    p.send("\x1b")?; // ESC

    // Wait for the program to terminate cleanly
    p.expect(Eof)?;
    Ok(())
}
