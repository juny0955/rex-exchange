//! gRPC 송신과 Kafka 수신을 order_id로 상관(correlate)해 E2E 지연·무손실을 집계한다.
//!
//! 같은 order_id가 여러 명령에 걸쳐 재사용되는 시나리오(place→cancel, place→amend→cancel)를
//! 위해 order_id별 FIFO(VecDeque)로 매칭한다. 심볼 키 기준 Kafka 순서가 보존되고, 한 워크로드
//! 단위 내 명령은 직렬 전송되므로 N번째 송신 ↔ N번째 수신이 성립한다.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Instant,
};

/// gRPC 송신 시각 알림(접수 성공한 명령에 대해서만 보낸다).
/// `sent_at`(T1)=요청 시작, `acked_at`(T2)=SUBMITTED 응답 수신.
pub struct SentNotice {
    pub order_id: String,
    pub sent_at: Instant,
    pub acked_at: Instant,
}

#[derive(Default)]
pub struct CorrelatorState {
    /// order_id별 (sent_at=T1, acked_at=T2) 쌍 FIFO.
    pending_sends: HashMap<String, VecDeque<(Instant, Instant)>>,
    /// order_id별 recv_at=T3 FIFO.
    pending_recvs: HashMap<String, VecDeque<Instant>>,
    seen_event_ids: HashSet<String>,
    ack_us: Vec<u64>,
    post_ack_us: Vec<u64>,
    e2e_us: Vec<u64>,
    sent: u64,
    received: u64,
    duplicates: u64,
    matched: u64,
}

impl CorrelatorState {
    pub fn on_sent(&mut self, order_id: String, sent_at: Instant, acked_at: Instant) {
        self.sent += 1;

        if let Some(recv_at) = pop_front(&mut self.pending_recvs, &order_id) {
            self.record_latency(sent_at, acked_at, recv_at);
            return;
        }

        self.pending_sends
            .entry(order_id)
            .or_default()
            .push_back((sent_at, acked_at));
    }

    pub fn is_settled(&self, expected: u64) -> bool {
        self.sent >= expected && self.matched >= expected
    }

    pub fn snapshot(&self) -> CorrelatorSummary {
        self.summary()
    }

    pub fn on_recv(&mut self, order_id: String, event_id: String, recv_at: Instant) {
        self.received += 1;

        if !self.seen_event_ids.insert(event_id) {
            self.duplicates += 1;
            return;
        }

        if let Some((sent_at, acked_at)) = pop_front(&mut self.pending_sends, &order_id) {
            self.record_latency(sent_at, acked_at, recv_at);
            return;
        }

        self.pending_recvs
            .entry(order_id)
            .or_default()
            .push_back(recv_at);
    }

    /// T1/T2/T3로부터 ack(T2−T1), post-ack(T3−T2), e2e(T3−T1) 지연을 각각 기록한다.
    fn record_latency(&mut self, sent_at: Instant, acked_at: Instant, recv_at: Instant) {
        self.ack_us
            .push(acked_at.saturating_duration_since(sent_at).as_micros() as u64);
        self.post_ack_us
            .push(recv_at.saturating_duration_since(acked_at).as_micros() as u64);
        self.e2e_us
            .push(recv_at.saturating_duration_since(sent_at).as_micros() as u64);
        self.matched += 1;
    }

    pub fn finish(self) -> CorrelatorSummary {
        self.summary()
    }

    fn summary(&self) -> CorrelatorSummary {
        let missing = self
            .pending_sends
            .values()
            .map(|q| q.len() as u64)
            .sum::<u64>();
        let unmatched_recv = self
            .pending_recvs
            .values()
            .map(|q| q.len() as u64)
            .sum::<u64>();
        CorrelatorSummary {
            sent: self.sent,
            received: self.received,
            unique: self.received - self.duplicates,
            duplicates: self.duplicates,
            matched: self.matched,
            missing,
            unmatched_recv,
            ack_latency: summarize(&self.ack_us),
            post_ack_latency: summarize(&self.post_ack_us),
            e2e_latency: summarize(&self.e2e_us),
        }
    }
}

/// 미정렬 지연 표본을 정렬해 `LatencySummary`로 요약한다.
fn summarize(samples: &[u64]) -> LatencySummary {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    LatencySummary::from_sorted(&sorted)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CorrelatorSummary {
    /// correlator가 처리한 접수 성공 송신 알림 수.
    pub sent: u64,
    /// Kafka에서 수신한 총 메시지 수(중복 포함).
    pub received: u64,
    /// 고유 event_id 수.
    pub unique: u64,
    /// 동일 event_id 재수신 수.
    pub duplicates: u64,
    /// 송신과 매칭되어 지연이 측정된 수.
    pub matched: u64,
    /// 접수했으나 끝까지 이벤트가 오지 않은 송신 수(손실 의심).
    pub missing: u64,
    /// 대응 송신을 찾지 못한 수신 수(이전 실행 잔여 등; latest 오프셋에서는 0이어야 함).
    pub unmatched_recv: u64,
    /// gRPC ack 지연(T2−T1): 송신→SUBMITTED.
    pub ack_latency: LatencySummary,
    /// post-ack 지연(T3−T2): SUBMITTED→Kafka 수신.
    pub post_ack_latency: LatencySummary,
    /// E2E 지연(T3−T1): 송신→Kafka 수신.
    pub e2e_latency: LatencySummary,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LatencySummary {
    pub count: u64,
    pub min_us: u64,
    pub p50_us: u64,
    pub p90_us: u64,
    pub p99_us: u64,
    pub p999_us: u64,
    pub max_us: u64,
    pub mean_us: u64,
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
            p50_us: percentile(sorted, 50.0),
            p90_us: percentile(sorted, 90.0),
            p99_us: percentile(sorted, 99.0),
            p999_us: percentile(sorted, 99.9),
            max_us: sorted[sorted.len() - 1],
            mean_us: (sum / count as u128) as u64,
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
fn pop_front<T>(map: &mut HashMap<String, VecDeque<T>>, order_id: &str) -> Option<T> {
    let queue = map.get_mut(order_id)?;
    let value = queue.pop_front();
    if queue.is_empty() {
        map.remove(order_id);
    }
    value
}
