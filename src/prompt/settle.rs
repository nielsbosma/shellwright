use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time::sleep;

/// Double-settle completion detector.
///
/// Implements a two-pass settle approach: after output stops for `settle_time`,
/// a first settle is recorded. After another `confirm_time` of continued silence,
/// the detection is confirmed. This eliminates false positives from brief pauses
/// in output (e.g., between progress bar updates).
#[derive(Debug)]
pub struct SettleDetector {
    settle_time: Duration,
    confirm_time: Duration,
    settled: Arc<AtomicBool>,
    confirmed: Arc<AtomicBool>,
    activity: Arc<Notify>,
}

impl SettleDetector {
    pub fn new(settle_time_ms: u64, confirm_time_ms: u64) -> Self {
        Self {
            settle_time: Duration::from_millis(settle_time_ms),
            confirm_time: Duration::from_millis(confirm_time_ms),
            settled: Arc::new(AtomicBool::new(false)),
            confirmed: Arc::new(AtomicBool::new(false)),
            activity: Arc::new(Notify::new()),
        }
    }

    /// Signal that new output was received. Resets the settle state.
    pub fn on_activity(&self) {
        self.settled.store(false, Ordering::SeqCst);
        self.confirmed.store(false, Ordering::SeqCst);
        self.activity.notify_one();
    }

    /// Returns true if the output has fully settled (double-confirm).
    pub fn is_confirmed(&self) -> bool {
        self.confirmed.load(Ordering::SeqCst)
    }

    /// Returns true if the first settle phase has completed.
    pub fn is_settled(&self) -> bool {
        self.settled.load(Ordering::SeqCst)
    }

    /// Wait for the output to settle with double confirmation.
    /// Returns `true` if settled, `false` if the timeout expired without settling.
    pub async fn wait_for_settle(&self, timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            if tokio::time::Instant::now() >= deadline {
                return false;
            }

            // First settle: must observe full settle_time of silence
            let remaining = deadline - tokio::time::Instant::now();
            if remaining < self.settle_time {
                // Not enough time left for a full settle period
                sleep(remaining).await;
                return false;
            }

            tokio::select! {
                _ = sleep(self.settle_time) => {
                    self.settled.store(true, Ordering::SeqCst);
                }
                _ = self.activity.notified() => {
                    self.settled.store(false, Ordering::SeqCst);
                    self.confirmed.store(false, Ordering::SeqCst);
                    continue;
                }
            }

            if !self.settled.load(Ordering::SeqCst) {
                continue;
            }

            // Second settle: confirm the silence holds for confirm_time
            let remaining = deadline - tokio::time::Instant::now();
            if remaining < self.confirm_time {
                sleep(remaining).await;
                return false;
            }

            tokio::select! {
                _ = sleep(self.confirm_time) => {
                    self.confirmed.store(true, Ordering::SeqCst);
                    return true;
                }
                _ = self.activity.notified() => {
                    self.settled.store(false, Ordering::SeqCst);
                    self.confirmed.store(false, Ordering::SeqCst);
                    continue;
                }
            }
        }
    }

    /// Reset the detector state.
    pub fn reset(&self) {
        self.settled.store(false, Ordering::SeqCst);
        self.confirmed.store(false, Ordering::SeqCst);
    }
}

impl Clone for SettleDetector {
    fn clone(&self) -> Self {
        Self {
            settle_time: self.settle_time,
            confirm_time: self.confirm_time,
            settled: Arc::clone(&self.settled),
            confirmed: Arc::clone(&self.confirmed),
            activity: Arc::clone(&self.activity),
        }
    }
}
