//! dispatch 접수 성공 시점과 publisher 결과 수신 시점을 order_id로 상관해 명령별 지연을 집계한다.
//!
//! 같은 order_id가 한 워크로드 안에서 재사용되므로(place→cancel, place→amend→cancel)
//! order_id별 FIFO(VecDeque)로 매칭한다. 형제 구현은 `integration_stress/metrics.rs`다
//! (백분위 항목과 id 타입이 달라 공유하지 않는다).
//!
//! 송신 알림은 runner 스레드가 unbounded channel로 보내고, correlator는 result handler
//! 스레드가 `on_recv`마다 drain한다. runner는 페이즈 종료 후 `take_summary`로 1회만 잠근다.
//! 표본은 u64(µs) 1개당 8바이트로, 1M cmd/s × 30s ≈ 240MB까지 커질 수 있다.

use std::{
    collections::{HashMap, VecDeque},
    mem,
    sync::{Arc, Mutex},
    time::Instant,
};

use crossbeam::channel::{Receiver, Sender};
use uuid::Uuid;

/// runner가 쓰는 송신 recorder와 publisher가 쓰는 correlator를 한데 묶는다.
pub struct LatencyTracker {
    recorder: LatencyRecorder,
    correlator: Arc<Mutex<LatencyCorrelator>>,
}

impl LatencyTracker {
    pub fn new() -> Self {
        let (recorder, correlator) = LatencyCorrelator::channel();
        Self {
            recorder,
            correlator: Arc::new(Mutex::new(correlator)),
        }
    }

    pub fn recorder(&self) -> &LatencyRecorder {
        &self.recorder
    }

    /// publisher에 전달할 correlator 핸들.
    pub fn correlator(&self) -> Arc<Mutex<LatencyCorrelator>> {
        Arc::clone(&self.correlator)
    }

    /// 누적 표본으로 요약을 만들고 상태를 비운다(페이즈 경계 격리).
    pub fn take_summary(&self) -> LatencySummary {
        self.correlator.lock().unwrap().take_summary()
    }
}

impl Default for LatencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LatencyRecorder {
    tx: Sender<(Uuid, Instant)>,
}

impl LatencyRecorder {
    pub fn record(&self, order_id: Uuid, at: Instant) {
        let _ = self.tx.send((order_id, at));
    }
}

pub struct LatencyCorrelator {
    rx: Receiver<(Uuid, Instant)>,
    pending_sends: HashMap<Uuid, VecDeque<Instant>>,
    pending_recvs: HashMap<Uuid, VecDeque<Instant>>,
    latencies_us: Vec<u64>,
}

impl LatencyCorrelator {
    pub fn channel() -> (LatencyRecorder, LatencyCorrelator) {
        let (tx, rx) = crossbeam::channel::unbounded();
        (
            LatencyRecorder { tx },
            LatencyCorrelator {
                rx,
                pending_sends: HashMap::new(),
                pending_recvs: HashMap::new(),
                latencies_us: Vec::new(),
            },
        )
    }

    pub fn on_recv(&mut self, order_id: Uuid, at: Instant) {
        self.drain_sent_notices();

        if let Some(send_at) = pop_front(&mut self.pending_sends, &order_id) {
            self.record_latency(send_at, at);
            return;
        }

        self.pending_recvs
            .entry(order_id)
            .or_default()
            .push_back(at);
    }

    /// 누적 표본으로 요약을 만들고 상태를 비운다(페이즈 경계 격리).
    pub fn take_summary(&mut self) -> LatencySummary {
        self.drain_sent_notices();

        let mut latencies_us = mem::take(&mut self.latencies_us);
        latencies_us.sort_unstable();

        self.pending_sends.clear();
        self.pending_recvs.clear();

        LatencySummary::from_sorted(&latencies_us)
    }

    fn drain_sent_notices(&mut self) {
        while let Ok((order_id, send_at)) = self.rx.try_recv() {
            if let Some(recv_at) = pop_front(&mut self.pending_recvs, &order_id) {
                self.record_latency(send_at, recv_at);
                continue;
            }

            self.pending_sends
                .entry(order_id)
                .or_default()
                .push_back(send_at);
        }
    }

    fn record_latency(&mut self, send_at: Instant, recv_at: Instant) {
        let micros = recv_at.saturating_duration_since(send_at).as_micros();
        self.latencies_us.push(micros as u64);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LatencySummary {
    pub count: u64,
    pub min_us: u64,
    pub mean_us: u64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub max_us: u64,
}

impl LatencySummary {
    /// 오름차순 정렬된 슬라이스로부터 백분위를 계산한다.
    pub fn from_sorted(sorted: &[u64]) -> Self {
        if sorted.is_empty() {
            return Self::default();
        }

        let count = sorted.len() as u64;
        let sum: u128 = sorted.iter().map(|&v| v as u128).sum();

        Self {
            count,
            min_us: sorted[0],
            mean_us: (sum / count as u128) as u64,
            p50_us: percentile(sorted, 50.0),
            p95_us: percentile(sorted, 95.0),
            p99_us: percentile(sorted, 99.0),
            max_us: sorted[sorted.len() - 1],
        }
    }
}

/// 최근접 순위(nearest-rank) 백분위. sorted는 오름차순·비어있지 않아야 한다.
fn percentile(sorted: &[u64], p: f64) -> u64 {
    let len = sorted.len();
    let rank = (p / 100.0 * (len as f64 - 1.0)).round() as usize;
    sorted[rank.min(len - 1)]
}

/// order_id 큐에서 맨 앞 항목을 꺼내고, 비면 엔트리를 제거한다.
fn pop_front(map: &mut HashMap<Uuid, VecDeque<Instant>>, order_id: &Uuid) -> Option<Instant> {
    let queue = map.get_mut(order_id)?;
    let value = queue.pop_front();
    if queue.is_empty() {
        map.remove(order_id);
    }
    value
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn from_sorted_empty_returns_default() {
        assert_eq!(LatencySummary::from_sorted(&[]), LatencySummary::default());
    }

    #[test]
    fn from_sorted_single_sample_uses_same_value_everywhere() {
        let summary = LatencySummary::from_sorted(&[42]);

        assert_eq!(
            summary,
            LatencySummary {
                count: 1,
                min_us: 42,
                mean_us: 42,
                p50_us: 42,
                p95_us: 42,
                p99_us: 42,
                max_us: 42,
            }
        );
    }

    #[test]
    fn from_sorted_uses_nearest_rank_percentiles() {
        let sorted = (1..=100).collect::<Vec<u64>>();

        let summary = LatencySummary::from_sorted(&sorted);

        assert_eq!(summary.count, 100);
        assert_eq!(summary.min_us, 1);
        assert_eq!(summary.mean_us, 50);
        assert_eq!(summary.p50_us, 51);
        assert_eq!(summary.p95_us, 95);
        assert_eq!(summary.p99_us, 99);
        assert_eq!(summary.max_us, 100);
    }

    #[test]
    fn matches_reused_order_id_in_fifo_order() {
        let (recorder, mut correlator) = LatencyCorrelator::channel();
        let order_id = Uuid::now_v7();
        let base = Instant::now();

        for i in 0..3u64 {
            recorder.record(order_id, base + Duration::from_micros(i * 100));
        }
        for i in 0..3u64 {
            correlator.on_recv(order_id, base + Duration::from_micros(i * 100 + 10));
        }

        let summary = correlator.take_summary();

        assert_eq!(summary.count, 3);
        assert_eq!(summary.min_us, 10);
        assert_eq!(summary.max_us, 10);
    }

    #[test]
    fn matches_recv_arriving_before_sent_notice() {
        let (recorder, mut correlator) = LatencyCorrelator::channel();
        let order_id = Uuid::now_v7();
        let base = Instant::now();

        correlator.on_recv(order_id, base + Duration::from_micros(50));
        recorder.record(order_id, base);

        let summary = correlator.take_summary();

        assert_eq!(summary.count, 1);
        assert_eq!(summary.p50_us, 50);
    }

    #[test]
    fn take_summary_resets_state_between_phases() {
        let (recorder, mut correlator) = LatencyCorrelator::channel();
        let order_id = Uuid::now_v7();
        let base = Instant::now();

        recorder.record(order_id, base);
        correlator.on_recv(order_id, base + Duration::from_micros(100));
        assert_eq!(correlator.take_summary().count, 1);

        recorder.record(order_id, base);
        correlator.on_recv(order_id, base + Duration::from_micros(200));
        let second = correlator.take_summary();

        assert_eq!(second.count, 1);
        assert_eq!(second.p50_us, 200);
    }

    #[test]
    fn unmatched_send_produces_no_sample() {
        let (recorder, mut correlator) = LatencyCorrelator::channel();

        recorder.record(Uuid::now_v7(), Instant::now());

        assert_eq!(correlator.take_summary().count, 0);
    }
}
