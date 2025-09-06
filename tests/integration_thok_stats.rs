use klik::thok::Thok;

#[test]
fn char_summary_with_deltas_integration() {
    let mut thok = Thok::new("hello".to_string(), 1, None, false);

    for c in "hello".chars() {
        thok.write(c);
    }
    assert!(thok.has_finished());
    thok.calc_results();

    if let Some(summary_with_deltas) = thok.get_char_summary_with_deltas() {
        assert!(!summary_with_deltas.is_empty());
        for (
            _character,
            _hist_avg,
            _hist_miss,
            _hist_attempts,
            _time_delta,
            _miss_delta,
            session_attempts,
            _latest_datetime,
        ) in &summary_with_deltas
        {
            assert!(*session_attempts >= 0);
        }
    }
}

#[test]
fn session_delta_summary_smoke() {
    let mut thok = Thok::new("test".to_string(), 1, None, false);
    for c in "test".chars() {
        thok.write(c);
    }
    assert!(thok.has_finished());
    let summary = thok.get_session_delta_summary();
    assert!(!summary.is_empty());
}

#[test]
fn auto_compaction_integration_smoke() {
    let mut thok = Thok::new("test".to_string(), 1, None, false);
    thok.session_state.started_at = Some(std::time::SystemTime::now());
    for c in "test".chars() {
        thok.write(c);
    }
    thok.calc_results();
    assert!(thok.has_finished());
    assert!(thok.session_state.wpm >= 0.0);
}
