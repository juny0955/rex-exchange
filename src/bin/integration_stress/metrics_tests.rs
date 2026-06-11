use std::time::{Duration, Instant};

use crate::integration_stress::metrics::{CorrelatorState, LatencySummary};

#[test]
fn 송신후_수신이면_매칭되고_지연이_측정된다() {
    let mut state = CorrelatorState::default();
    let t0 = Instant::now();

    state.on_sent("o1".into(), t0);
    state.on_recv("o1".into(), "e1".into(), t0 + Duration::from_millis(5));

    let summary = state.finish();
    assert_eq!(summary.sent, 1);
    assert_eq!(summary.matched, 1);
    assert_eq!(summary.missing, 0);
    assert_eq!(summary.unmatched_recv, 0);
    assert_eq!(summary.latency.count, 1);
    assert_eq!(summary.latency.p50_us, 5_000);
}

#[test]
fn 수신이_먼저_와도_나중_송신과_매칭된다() {
    let mut state = CorrelatorState::default();
    let t0 = Instant::now();

    // 레이스: Kafka 수신이 송신 보고보다 먼저 correlator에 도달
    state.on_recv("o1".into(), "e1".into(), t0 + Duration::from_millis(3));
    state.on_sent("o1".into(), t0);

    let summary = state.finish();
    assert_eq!(summary.sent, 1);
    assert_eq!(summary.matched, 1);
    assert_eq!(summary.missing, 0);
    assert_eq!(summary.unmatched_recv, 0);
    assert_eq!(summary.latency.p50_us, 3_000);
}

#[test]
fn 동일_order_id_재사용은_fifo로_매칭된다() {
    let mut state = CorrelatorState::default();
    let t0 = Instant::now();

    // place 후 cancel: 같은 order_id로 두 번 송신/수신
    state.on_sent("o1".into(), t0);
    state.on_sent("o1".into(), t0 + Duration::from_millis(10));
    state.on_recv("o1".into(), "place".into(), t0 + Duration::from_millis(1));
    state.on_recv("o1".into(), "cancel".into(), t0 + Duration::from_millis(12));

    let summary = state.finish();
    assert_eq!(summary.sent, 2);
    assert_eq!(summary.matched, 2);
    assert_eq!(summary.missing, 0);
    // 1번째 송신(t0)↔1번째 수신(+1ms)=1ms, 2번째 송신(+10ms)↔2번째 수신(+12ms)=2ms
    assert_eq!(summary.latency.min_us, 1_000);
    assert_eq!(summary.latency.max_us, 2_000);
}

#[test]
fn 중복_event_id는_중복으로_집계된다() {
    let mut state = CorrelatorState::default();
    let t0 = Instant::now();

    state.on_sent("o1".into(), t0);
    state.on_recv("o1".into(), "e1".into(), t0 + Duration::from_millis(1));
    state.on_recv("o1".into(), "e1".into(), t0 + Duration::from_millis(2)); // 재전송

    let summary = state.finish();
    assert_eq!(summary.sent, 1);
    assert_eq!(summary.received, 2);
    assert_eq!(summary.unique, 1);
    assert_eq!(summary.duplicates, 1);
    assert_eq!(summary.matched, 1);
}

#[test]
fn 이벤트가_안오면_누락으로_집계된다() {
    let mut state = CorrelatorState::default();
    let t0 = Instant::now();

    state.on_sent("o1".into(), t0);
    state.on_sent("o2".into(), t0);
    state.on_recv("o1".into(), "e1".into(), t0 + Duration::from_millis(1));

    let summary = state.finish();
    assert_eq!(summary.sent, 2);
    assert_eq!(summary.matched, 1);
    assert_eq!(summary.missing, 1);
}

#[test]
fn 모든_송신과_수신이_매칭되어야_정착으로_본다() {
    let mut state = CorrelatorState::default();
    let t0 = Instant::now();

    state.on_recv("o1".into(), "e1".into(), t0 + Duration::from_millis(1));

    let summary = state.snapshot();
    assert_eq!(summary.sent, 0);
    assert_eq!(summary.matched, 0);
    assert!(!state.is_settled(1));

    state.on_sent("o1".into(), t0);

    let summary = state.snapshot();
    assert_eq!(summary.sent, 1);
    assert_eq!(summary.matched, 1);
    assert!(state.is_settled(1));
}

#[test]
fn 빈_표본의_백분위는_0() {
    let summary = LatencySummary::from_sorted(&[]);
    assert_eq!(summary, LatencySummary::default());
}

#[test]
fn 백분위_계산_확인() {
    let sorted: Vec<u64> = (0..=100).collect();
    let summary = LatencySummary::from_sorted(&sorted);

    assert_eq!(summary.count, 101);
    assert_eq!(summary.min_us, 0);
    assert_eq!(summary.max_us, 100);
    assert_eq!(summary.p50_us, 50);
    assert_eq!(summary.p90_us, 90);
    assert_eq!(summary.p99_us, 99);
}
