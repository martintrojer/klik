# TODO

## Architecture

- [ ] Consider a distinct `Session` type owning `SessionConfig/SessionState` to reduce `Thok` responsibilities.
- [ ] Batch DB writes: buffer stats and flush less frequently (currently per char plus flush at end).

## Tests

- [ ] **Un-ignore integration test**: `integration_min_session` is `#[ignore]` — PTY spawning is environment-dependent, keep ignored but consider CI step with `--ignored` allowed to fail.
