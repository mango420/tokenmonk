//! End to end: status-line payload on stdin -> bridge -> `statusline.json` ->
//! file source -> limit worker / limit state -> snapshot -> halo and game.
//! Anything that needs a running Tauri app is out of scope; the bridge is
//! driven through `bridge::process`, the function `--statusline` mode runs.
//!
//! Times (UTC, 2026-10-28): the bridge runs at 10:00, the 5-hour window
//! resets at 12:00, the weekly window on the 30th at 18:00.

mod common;

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use common::pipeline::Rig;
use common::*;
use tokenmonk_lib::app::halo_percent;
use tokenmonk_lib::contracts::limits::{LimitReason, LimitSource, LimitStatus};
use tokenmonk_lib::contracts::Mode;
use tokenmonk_lib::limits::bridge::{process, FALLBACK_LINE, FILE_NAME};
use tokenmonk_lib::limits::{spawn_with, LimitPoll, StatuslineSource, WorkerConfig};
use tokenmonk_lib::watcher::SnapshotCause;

const MIN: i64 = 60_000;

fn t0() -> i64 {
    utc_ms(2026, 10, 28, 10, 0, 0)
}
fn five_reset() -> i64 {
    utc_ms(2026, 10, 28, 12, 0, 0)
}
fn week_reset() -> i64 {
    utc_ms(2026, 10, 30, 18, 0, 0)
}

/// What Claude Code pipes to the status-line command: the two rate-limit
/// windows (reset times in epoch seconds) plus fields the bridge must drop.
fn payload(five: Option<(f64, i64)>, seven: Option<(f64, i64)>) -> Vec<u8> {
    let window = |w: Option<(f64, i64)>| match w {
        Some((pct, reset_ms)) => format!(
            r#"{{"used_percentage":{pct},"resets_at":{}}}"#,
            reset_ms / 1000
        ),
        None => "null".to_owned(),
    };
    let mut limits = Vec::new();
    if five.is_some() {
        limits.push(format!(r#""five_hour":{}"#, window(five)));
    }
    if seven.is_some() {
        limits.push(format!(r#""seven_day":{}"#, window(seven)));
    }
    format!(
        r#"{{"session_id":"fake-session-id","cwd":"/Users/example/projects/synthetic","model":{{"id":"claude-sonnet-5","display_name":"Sonnet"}},"cost":{{"total_cost_usd":0.5}},"rate_limits":{{{}}}}}"#,
        limits.join(",")
    )
    .into_bytes()
}

struct Bridge {
    dir: tempfile::TempDir,
}

impl Bridge {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }
    /// Status line runs at `now`; returns the line it prints.
    fn run(&self, input: &[u8], now: i64) -> String {
        process(input, now, Some(self.dir.path()))
    }
    fn file(&self) -> PathBuf {
        self.dir.path().join(FILE_NAME)
    }
    fn source(&self) -> StatuslineSource {
        StatuslineSource::at(self.file())
    }
}

#[test]
fn a_payload_becomes_a_limit_reading_in_the_snapshot() {
    let bridge = Bridge::new();
    let line = bridge.run(
        &payload(Some((23.5, five_reset())), Some((41.2, week_reset()))),
        t0(),
    );
    assert_eq!(line, "5h 23% | 7d 41%");

    // Only the rate limits are kept.
    let stored = std::fs::read_to_string(bridge.file()).unwrap();
    for leaked in ["fake-session-id", "synthetic", "claude-sonnet-5", "cost"] {
        assert!(!stored.contains(leaked), "{leaked} leaked into {stored}");
    }

    let reading = bridge
        .source()
        .fetch_at(t0() + MIN)
        .expect("current windows");
    assert_eq!(reading.fetched_at, t0(), "the time the bridge ran");
    let five = reading.five_hour.clone().unwrap();
    assert_eq!(
        (five.utilization, five.resets_at),
        (23.5, Some(five_reset()))
    );
    let weekly = reading.weekly.clone().unwrap();
    assert_eq!(
        (weekly.utilization, weekly.resets_at),
        (41.2, Some(week_reset()))
    );

    let mut rig = Rig::new(Mode::Max);
    rig.limit(Ok(reading));
    let s = rig.snap(t0() + MIN, SnapshotCause::Limits);
    assert_eq!(s.limits.status, LimitStatus::Ok);
    assert_eq!(s.limits.source.as_deref(), Some("statusline"));
    assert_eq!(s.limits.five_hour.as_ref().unwrap().utilization, 23.5);
    assert_eq!(s.limits.weekly.as_ref().unwrap().utilization, 41.2);
    assert_eq!(halo_percent(Mode::Max, &s), Some(23.5));

    // The file only changes while Claude Code runs: 16 minutes on, the same
    // values are shown, marked as old.
    let s = rig.snap(t0() + 16 * MIN, SnapshotCause::Tick);
    assert_eq!(s.limits.status, LimitStatus::Stale);
    assert_eq!(s.limits.five_hour.as_ref().unwrap().utilization, 23.5);
}

#[test]
fn a_window_that_resets_is_dropped_and_the_next_one_takes_over() {
    let bridge = Bridge::new();
    let mut rig = Rig::new(Mode::Max);

    // The app is up (empty history, the game is evaluating).
    rig.snap(t0() - MIN, SnapshotCause::ImportDone);

    // 10:00: five-hour window at 99.4 %, close to the limit but not at it.
    bridge.run(
        &payload(Some((99.4, five_reset())), Some((41.2, week_reset()))),
        t0(),
    );
    rig.limit(bridge.source().fetch_at(t0()));
    rig.snap(t0() + MIN, SnapshotCause::Limits);
    assert!(
        rig.rec.achievements().is_empty(),
        "the window is still running"
    );

    // 12:00: the window has reset. The file is unchanged (Claude Code is
    // idle), the source drops the five-hour window and keeps the weekly one.
    let reading = bridge
        .source()
        .fetch_at(five_reset())
        .expect("weekly is left");
    assert!(reading.five_hour.is_none());
    assert_eq!(reading.weekly.as_ref().unwrap().utilization, 41.2);
    rig.limit(Ok(reading));
    let s = rig.snap(five_reset(), SnapshotCause::Tick);
    assert!(s.limits.five_hour.is_none());
    assert_eq!(s.limits.weekly.as_ref().unwrap().utilization, 41.2);
    assert_eq!(
        s.limits.status,
        LimitStatus::Stale,
        "last status line was 2 hours ago"
    );
    assert_eq!(halo_percent(Mode::Max, &s), None, "nothing to gauge");
    assert_eq!(s.burn_rate.limit_eta_minutes, None);

    // Peak 99.4 % in a finished window: So Close, announced once.
    assert_eq!(rig.rec.achievements(), ["soClose"]);

    // 12:00:10: Claude Code reruns the status line for the new window.
    let new_reset = five_reset() + 5 * 60 * MIN;
    let later = five_reset() + 10_000;
    bridge.run(
        &payload(Some((3.0, new_reset)), Some((41.5, week_reset()))),
        later,
    );
    rig.limit(bridge.source().fetch_at(later));
    let s = rig.snap(later, SnapshotCause::Limits);
    assert_eq!(s.limits.status, LimitStatus::Ok);
    let five = s.limits.five_hour.clone().unwrap();
    assert_eq!((five.utilization, five.resets_at), (3.0, Some(new_reset)));
    assert_eq!(halo_percent(Mode::Max, &s), Some(3.0));
    rig.snap(later + MIN, SnapshotCause::Tick);
    assert_eq!(rig.rec.achievements(), ["soClose"], "never again");

    // Later the new window is exhausted: Rate-Limit Rambo, once.
    let hit = five_reset() + 60 * MIN;
    bridge.run(
        &payload(Some((100.0, new_reset)), Some((43.0, week_reset()))),
        hit,
    );
    rig.limit(bridge.source().fetch_at(hit));
    let s = rig.snap(hit, SnapshotCause::Limits);
    assert_eq!(s.burn_rate.limit_eta_minutes, Some(0));
    assert_eq!(rig.rec.achievements(), ["soClose", "rateLimitRambo"]);
    rig.limit(bridge.source().fetch_at(hit + MIN));
    rig.snap(hit + MIN, SnapshotCause::Limits);
    rig.snap(hit + 2 * MIN, SnapshotCause::Tick);
    assert_eq!(rig.rec.achievements(), ["soClose", "rateLimitRambo"]);
    // 100 XP each, no token usage at all.
    assert_eq!(rig.xp_total(), 200);

    // The game survives a restart with the same readings and says nothing.
    rig.restart();
    rig.snap(hit + 3 * MIN, SnapshotCause::ImportDone);
    assert!(rig.rec.achievements().is_empty());
    assert_eq!(rig.xp_total(), 200);
}

#[test]
fn with_every_window_over_there_is_no_limit_data() {
    let bridge = Bridge::new();
    let source = bridge.source();

    // Bridge not set up yet.
    let err = source.fetch_at(t0()).unwrap_err();
    assert_eq!(err.reason, LimitReason::NoSource);
    assert_eq!(err.detail, "status-line bridge not set up");

    // Both windows present, then both over.
    bridge.run(
        &payload(
            Some((50.0, five_reset())),
            Some((10.0, five_reset() + 30 * MIN)),
        ),
        t0(),
    );
    let mut rig = Rig::new(Mode::Max);
    rig.limit(source.fetch_at(t0()));
    assert_eq!(
        rig.snap(t0(), SnapshotCause::Limits).limits.status,
        LimitStatus::Ok
    );

    let after = five_reset() + 30 * MIN;
    let err = source.fetch_at(after).unwrap_err();
    assert_eq!(err.reason, LimitReason::NoSource);
    assert_eq!(err.detail, "no current window");
    rig.limit(Err(err));
    let s = rig.snap(after, SnapshotCause::Limits);
    assert_eq!(s.limits.status, LimitStatus::Unavailable);
    assert_eq!(s.limits.reason, Some(LimitReason::NoSource));
    assert!(s.limits.five_hour.is_none() && s.limits.weekly.is_none());
    assert_eq!(halo_percent(Mode::Max, &s), None);
}

#[test]
fn input_without_a_window_changes_nothing() {
    let bridge = Bridge::new();

    // No rate_limits (no subscription, or before the first response): only
    // the fallback line, no file.
    let none = br#"{"session_id":"fake-session-id","cwd":"/Users/example/projects/synthetic"}"#;
    assert_eq!(bridge.run(none, t0()), FALLBACK_LINE);
    assert!(!bridge.file().exists());
    assert_eq!(
        bridge.source().fetch_at(t0()).unwrap_err().reason,
        LimitReason::NoSource
    );

    // A good run, then garbage and an empty run: the last good reading stays.
    bridge.run(&payload(Some((12.0, five_reset())), None), t0());
    let good = bridge.source().fetch_at(t0()).unwrap();
    assert_eq!(bridge.run(b"not json at all", t0() + MIN), FALLBACK_LINE);
    assert_eq!(bridge.run(b"", t0() + 2 * MIN), FALLBACK_LINE);
    assert_eq!(bridge.run(&none[..], t0() + 3 * MIN), FALLBACK_LINE);
    assert_eq!(bridge.source().fetch_at(t0() + 4 * MIN).unwrap(), good);
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn next_poll(rx: &Receiver<LimitPoll>) -> LimitPoll {
    rx.recv_timeout(Duration::from_secs(5))
        .expect("a poll within 5 seconds")
}

fn poll_until(rx: &Receiver<LimitPoll>, done: impl Fn(&LimitPoll) -> bool) -> LimitPoll {
    for _ in 0..100 {
        let p = next_poll(rx);
        if done(&p) {
            return p;
        }
    }
    panic!("expected poll never arrived");
}

/// The real limits worker thread over a real file: a status line run shows
/// up as a poll, a reset window as another one. Uses the system clock
/// because the source does.
#[test]
fn the_limit_worker_delivers_what_the_bridge_wrote() {
    let bridge = Bridge::new();
    let (tx, rx) = channel();
    let source = Arc::new(bridge.source());
    assert_eq!(source.id(), "statusline");
    let _handle = spawn_with(
        source,
        tx,
        WorkerConfig {
            interval: Duration::from_millis(20),
            min_refresh_gap: Duration::ZERO,
        },
    )
    .unwrap();

    // Nothing set up: the first poll says so.
    let first = next_poll(&rx);
    assert_eq!(first.source, "statusline");
    assert_eq!(first.result.unwrap_err().reason, LimitReason::NoSource);

    // The status line runs.
    let now = now_ms();
    let in_two_hours = now + 120 * MIN;
    bridge.run(&payload(Some((64.5, in_two_hours)), None), now);
    let poll = poll_until(&rx, |p| p.result.is_ok());
    let reading = poll.result.clone().unwrap();
    assert_eq!(reading.five_hour.as_ref().unwrap().utilization, 64.5);
    assert_eq!(reading.fetched_at, now);

    // Through the aggregate, as the ingest worker does.
    let mut rig = Rig::new(Mode::Max);
    rig.limit(poll.result);
    let s = rig.snap(now_ms(), SnapshotCause::Limits);
    assert_eq!(s.limits.status, LimitStatus::Ok);
    assert_eq!(halo_percent(Mode::Max, &s), Some(64.5));

    // Both windows are over (reset times in the past): the worker reports
    // that no window is current.
    bridge.run(&payload(Some((64.5, now - 60 * MIN)), None), now + 1);
    let poll = poll_until(&rx, |p| p.result.is_err());
    assert_eq!(poll.result.unwrap_err().detail, "no current window");
}

/// The aggregate drops a window whose reset time has passed even when no new
/// poll arrived (docs/ARCHITECTURE.md, "Limit status").
#[test]
fn the_aggregate_drops_a_window_that_reset_between_polls() {
    let bridge = Bridge::new();
    bridge.run(
        &payload(Some((50.0, five_reset())), Some((10.0, week_reset()))),
        t0(),
    );
    let mut rig = Rig::new(Mode::Max);
    rig.limit(bridge.source().fetch_at(t0()));

    // The snapshot is built at 12:01 from the last poll, before the next
    // one (a poll every 30 seconds) has dropped the window.
    let s = rig.snap(five_reset() + MIN, SnapshotCause::Tick);
    assert!(
        s.limits.five_hour.is_none(),
        "five-hour window shown after its reset: {:?}",
        s.limits.five_hour
    );
    assert!(s.limits.weekly.is_some());
    assert_eq!(halo_percent(Mode::Max, &s), None);
}
