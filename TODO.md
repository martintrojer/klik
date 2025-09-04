# TODO: Abstraction and Architecture Improvements

This document tracks proposed refactors to improve separation of concerns, testability, and maintainability.

## Events & Runtime
- [x] Introduce `ThokEventSource` trait (recv -> ThokEvent)
  - [x] Impl: `CrosstermEventSource` (prod)
  - [x] Impl: `TestEventSource` (tests)
- [x] Add `Ticker`/`Clock` trait for ticks (configurable rate)
- [x] Extract a `Runner` with `step()` to advance one event/tick

## UI Boundaries
- [ ] Define `Screen` trait: `render(&self, app, frame)` and `on_key(&mut self, key, app)`
- [ ] Implement screens: Typing, Results, CharacterStats
- [x] Move character stats rendering out of `main.rs` into `ui` (e.g. `ui/character_stats.rs`)
- [x] Add a presenter for character stats rows (pure function) to simplify the widget

## Domain vs Persistence
- [x] Create `StatsStore` trait (get/put/flush/summary APIs)
  - [x] Impl: `SqliteStatsStore` (current DB via `StatsDb`)
  - [x] Impl: `NoopStatsStore` (tests)
  - [x] Impl: `InMemoryStatsStore` (bench/tests)
- [x] Introduce `AppDirs` service for config/log/db paths
- [x] Inject `StatsStore` into `Thok` instead of owning DB directly

## Typing Logic (Thok)
- [x] Extract `TypingPolicy` strategies (strict vs normal) from `write()`
- [x] Add `SessionConfig` and `SessionResult` scaffolding (to evolve into full split)
- [ ] Split session into `Session` (config), `SessionState` (mutable), `SessionResult`
- [ ] Replace raw `(f64,f64)` WPM points with `TimeSeriesPoint { t, wpm }`

## Language & Formatting
- [ ] Consolidate `language/formatter.rs` and `language/formatting.rs`
  - Keep `TextFormatter` + `CompositeFormatter` strategy
  - Remove/redirect `Language::apply_advanced_formatting` to formatter
- [ ] Ensure a single source of truth for word selectors (avoid duplication with `selection.rs`)
- [ ] Make `WordGenerator` depend only on formatter trait for formatting step

## Configuration
- [ ] Add persisted `Config` (user prefs: theme, defaults, language)
- [ ] Create `ConfigStore` trait (file-backed impl later)

## Rendering Helpers
- [x] Extract `charting` helpers (bounds, label formatting) used by UI

## Logging/Diagnostics
- [ ] Replace `println!` in tests with `log` macros behind a `test` feature or `RUST_LOG`

## Tests Organization
- [ ] Move long integration-style tests from `src/*` to `tests/` where feasible
- [ ] Keep PTY-driven E2E test (Unix) and add a headless integration test using `ThokEventSource` test impl

---

### Suggested Phasing
- Phase 1: EventSource/Ticker/Runner + move character-stats UI + chart helpers [COMPLETED]
- Phase 2: StatsStore/AppDirs injection [DONE], TypingPolicy + Session structs [NEXT]
- Phase 3: Formatter consolidation + Config/ConfigStore
- Phase 4: Test reorganization + logging cleanup

### Testing Policy for New Abstractions
- [ ] Every new abstraction (trait/struct/module) must include targeted unit tests.
- [ ] Refactor existing tests to use the new abstraction (migrate mocks/helpers accordingly).
- [ ] Prefer headless tests over PTY where possible (use `ThokEventSource`/`Ticker` test impls).
- [ ] Keep integration coverage by adding at least one end-to-end path using the new pieces.
