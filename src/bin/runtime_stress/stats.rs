use std::{
    ops::Sub,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DispatchStats {
    pub accepted: usize,
    pub channel_full: usize,
    pub unknown_symbol: usize,
    pub engine_stopped: usize,
}

impl DispatchStats {
    pub fn rejected(self) -> usize {
        self.channel_full + self.unknown_symbol + self.engine_stopped
    }
}

#[derive(Default)]
pub struct ResultStats {
    published: AtomicUsize,
    place_results: AtomicUsize,
    cancel_results: AtomicUsize,
    amend_results: AtomicUsize,
    trades: AtomicUsize,
    maker_updates: AtomicUsize,
}

impl ResultStats {
    pub fn add_published(&self) {
        self.published.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_place_result(&self, trades: usize, maker_updates: usize) {
        self.place_results.fetch_add(1, Ordering::Relaxed);
        self.trades.fetch_add(trades, Ordering::Relaxed);
        self.maker_updates
            .fetch_add(maker_updates, Ordering::Relaxed);
    }

    pub fn add_cancel_result(&self) {
        self.cancel_results.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_amend_result(&self) {
        self.amend_results.fetch_add(1, Ordering::Relaxed);
    }

    pub fn published(&self) -> usize {
        self.published.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> ResultStatsSnapshot {
        ResultStatsSnapshot {
            published: self.published.load(Ordering::Relaxed),
            place_results: self.place_results.load(Ordering::Relaxed),
            cancel_results: self.cancel_results.load(Ordering::Relaxed),
            amend_results: self.amend_results.load(Ordering::Relaxed),
            trades: self.trades.load(Ordering::Relaxed),
            maker_updates: self.maker_updates.load(Ordering::Relaxed),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResultStatsSnapshot {
    pub published: usize,
    pub place_results: usize,
    pub cancel_results: usize,
    pub amend_results: usize,
    pub trades: usize,
    pub maker_updates: usize,
}

impl Sub for ResultStatsSnapshot {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            published: self.published.saturating_sub(rhs.published),
            place_results: self.place_results.saturating_sub(rhs.place_results),
            cancel_results: self.cancel_results.saturating_sub(rhs.cancel_results),
            amend_results: self.amend_results.saturating_sub(rhs.amend_results),
            trades: self.trades.saturating_sub(rhs.trades),
            maker_updates: self.maker_updates.saturating_sub(rhs.maker_updates),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhaseStats {
    pub attempted_commands: usize,
    pub dispatch: DispatchStats,
    pub result: ResultStatsSnapshot,
    pub completed: bool,
    pub dispatch_elapsed: Duration,
    pub total_elapsed: Duration,
    pub pacing_lag_events: usize,
    pub pacing_lag: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_stats_rejected_sums_rejection_types() {
        let stats = DispatchStats {
            accepted: 1,
            channel_full: 2,
            unknown_symbol: 3,
            engine_stopped: 4,
        };

        assert_eq!(stats.rejected(), 9);
    }

    #[test]
    fn result_snapshot_subtracts_counters() {
        let left = ResultStatsSnapshot {
            published: 10,
            place_results: 9,
            cancel_results: 8,
            amend_results: 7,
            trades: 6,
            maker_updates: 5,
        };
        let right = ResultStatsSnapshot {
            published: 1,
            place_results: 2,
            cancel_results: 3,
            amend_results: 4,
            trades: 5,
            maker_updates: 6,
        };

        assert_eq!(
            left - right,
            ResultStatsSnapshot {
                published: 9,
                place_results: 7,
                cancel_results: 5,
                amend_results: 3,
                trades: 1,
                maker_updates: 0,
            }
        );
    }
}
