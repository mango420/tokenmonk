//! The limits worker thread (docs/ARCHITECTURE.md, "Threads" and "Limits").
//!
//! One thread calls [`LimitSource::fetch`] on a fixed interval and sends each
//! outcome over a channel to the ingest worker. The channel is unbounded, so
//! sending never blocks, and a slow or failing source only delays its own
//! thread, never log ingestion.
//!
//! Release builds use `panic = "abort"`, so a panic cannot be caught here:
//! [`LimitSource`] implementations must return a `LimitError` for every
//! failure and never panic.

use std::io;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::contracts::limits::{LimitError, LimitReading, LimitSource};

/// Poll every 5 minutes while the app runs.
pub const POLL_INTERVAL: Duration = Duration::from_secs(5 * 60);
/// A refresh request (popup opened) is ignored if the last poll is younger.
pub const MIN_REFRESH_GAP: Duration = Duration::from_secs(60);

/// Timing of the worker; tests use short values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerConfig {
    pub interval: Duration,
    pub min_refresh_gap: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            interval: POLL_INTERVAL,
            min_refresh_gap: MIN_REFRESH_GAP,
        }
    }
}

/// One poll outcome, as received by the ingest worker.
#[derive(Debug, Clone, PartialEq)]
pub struct LimitPoll {
    /// [`LimitSource::id`] of the source that was asked.
    pub source: &'static str,
    pub result: Result<LimitReading, LimitError>,
}

enum Control {
    Refresh,
    Stop,
}

/// Handle to the running worker. Keep it in Tauri managed state: the popup
/// then asks for a fresh reading when it opens (`window::show`).
#[derive(Debug)]
pub struct LimitsHandle {
    control: Sender<Control>,
}

impl LimitsHandle {
    /// Asks for a poll now. Ignored by the worker if the last poll is
    /// younger than [`WorkerConfig::min_refresh_gap`]. Never blocks.
    pub fn refresh(&self) {
        let _ = self.control.send(Control::Refresh);
    }

    /// Asks the worker to stop after its current fetch. Never blocks.
    pub fn stop(&self) {
        let _ = self.control.send(Control::Stop);
    }
}

impl Drop for LimitsHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Starts the worker with the default timing. The first poll runs
/// immediately; results go to `out`. The worker ends when the handle is
/// dropped or stopped, or when the receiving side of `out` is gone.
pub fn spawn(source: Arc<dyn LimitSource>, out: Sender<LimitPoll>) -> io::Result<LimitsHandle> {
    spawn_with(source, out, WorkerConfig::default())
}

/// Like [`spawn`] with explicit timing.
pub fn spawn_with(
    source: Arc<dyn LimitSource>,
    out: Sender<LimitPoll>,
    config: WorkerConfig,
) -> io::Result<LimitsHandle> {
    let (control, inbox) = channel();
    thread::Builder::new()
        .name("limits-worker".into())
        .spawn(move || {
            let mut last_poll: Option<Instant> = None;
            let mut poll = true;
            loop {
                if poll {
                    last_poll = Some(Instant::now());
                    if out.send(poll_once(source.as_ref())).is_err() {
                        return;
                    }
                }
                let wait = config
                    .interval
                    .saturating_sub(last_poll.map_or(Duration::ZERO, |t| t.elapsed()));
                poll = match inbox.recv_timeout(wait) {
                    Ok(Control::Refresh) => {
                        last_poll.is_none_or(|t| t.elapsed() >= config.min_refresh_gap)
                    }
                    Ok(Control::Stop) | Err(RecvTimeoutError::Disconnected) => return,
                    Err(RecvTimeoutError::Timeout) => true,
                };
            }
        })?;
    Ok(LimitsHandle { control })
}

/// Calls the source once.
fn poll_once(source: &dyn LimitSource) -> LimitPoll {
    LimitPoll {
        source: source.id(),
        result: source.fetch(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::limits::LimitReason;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::Receiver;

    const WAIT: Duration = Duration::from_secs(5);

    struct Counting {
        calls: AtomicUsize,
    }

    impl LimitSource for Counting {
        fn id(&self) -> &'static str {
            "test"
        }
        fn fetch(&self) -> Result<LimitReading, LimitError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(LimitReading {
                five_hour: None,
                weekly: None,
                fetched_at: n as i64,
            })
        }
    }

    struct Failing;

    impl LimitSource for Failing {
        fn id(&self) -> &'static str {
            "failing"
        }
        fn fetch(&self) -> Result<LimitReading, LimitError> {
            Err(LimitError {
                reason: LimitReason::Network,
                detail: "offline".into(),
            })
        }
    }

    fn counting() -> Arc<Counting> {
        Arc::new(Counting {
            calls: AtomicUsize::new(0),
        })
    }

    fn start(
        source: Arc<dyn LimitSource>,
        interval_ms: u64,
        gap_ms: u64,
    ) -> (LimitsHandle, Receiver<LimitPoll>) {
        let (tx, rx) = channel();
        let handle = spawn_with(
            source,
            tx,
            WorkerConfig {
                interval: Duration::from_millis(interval_ms),
                min_refresh_gap: Duration::from_millis(gap_ms),
            },
        )
        .unwrap();
        (handle, rx)
    }

    #[test]
    fn polls_immediately_and_then_on_the_interval() {
        let (_handle, rx) = start(counting(), 30, 0);
        let first = rx.recv_timeout(WAIT).unwrap();
        assert_eq!(first.source, "test");
        assert_eq!(first.result.unwrap().fetched_at, 0);
        assert_eq!(rx.recv_timeout(WAIT).unwrap().result.unwrap().fetched_at, 1);
        assert_eq!(rx.recv_timeout(WAIT).unwrap().result.unwrap().fetched_at, 2);
    }

    #[test]
    fn refresh_polls_early_once_the_gap_has_passed() {
        let (handle, rx) = start(counting(), 60_000, 0);
        rx.recv_timeout(WAIT).unwrap();
        handle.refresh();
        let second = rx.recv_timeout(WAIT).unwrap();
        assert_eq!(second.result.unwrap().fetched_at, 1);
    }

    #[test]
    fn refresh_inside_the_gap_is_ignored() {
        let (handle, rx) = start(counting(), 60_000, 60_000);
        rx.recv_timeout(WAIT).unwrap();
        handle.refresh();
        assert!(rx.recv_timeout(Duration::from_millis(150)).is_err());
    }

    #[test]
    fn failing_source_keeps_the_worker_alive() {
        let (_handle, rx) = start(Arc::new(Failing), 20, 0);
        for _ in 0..3 {
            let poll = rx.recv_timeout(WAIT).unwrap();
            assert_eq!(poll.source, "failing");
            assert_eq!(poll.result.unwrap_err().reason, LimitReason::Network);
        }
    }

    #[test]
    fn stop_ends_the_worker() {
        let (handle, rx) = start(counting(), 60_000, 0);
        rx.recv_timeout(WAIT).unwrap();
        handle.stop();
        // The thread drops its sender on exit, which disconnects the channel.
        assert_eq!(
            rx.recv_timeout(WAIT),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
        );
    }

    #[test]
    fn dropping_the_receiver_ends_the_worker() {
        let source = counting();
        let (handle, rx) = start(source.clone(), 10, 0);
        rx.recv_timeout(WAIT).unwrap();
        drop(rx);
        thread::sleep(Duration::from_millis(100));
        let calls = source.calls.load(Ordering::SeqCst);
        thread::sleep(Duration::from_millis(100));
        assert_eq!(source.calls.load(Ordering::SeqCst), calls);
        drop(handle);
    }
}
