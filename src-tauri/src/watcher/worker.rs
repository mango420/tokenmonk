//! The ingest worker thread (docs/ARCHITECTURE.md, "Threads" and "Ingest").
//!
//! One thread owns the only write connection. It reacts to file-system
//! events (debounced), a slow periodic rescan, limit polls from the limits
//! worker and a minute tick. After each change it recomputes the dashboard
//! snapshot and hands it to the [`IngestPublisher`].

use std::collections::HashSet;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{FixedOffset, Local};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use super::discover::{discover, is_jsonl, projects_dir, DiscoverError};
use super::ingest::{ingest_file, needs_work};
use super::publisher::{IngestPublisher, SnapshotCause};
use crate::aggregate::{snapshot, LimitState};
use crate::contracts::dashboard::{DataState, DataStatus};
use crate::contracts::settings::Mode;
use crate::contracts::EpochMs;
use crate::db::Db;
use crate::limits::LimitPoll;

/// Safety-net scan for missed file events and for the log folder appearing.
pub const RESCAN_INTERVAL: Duration = Duration::from_secs(60);
/// Day rollover, burn rate decay, session idle cut-off, season check.
pub const TICK_INTERVAL: Duration = Duration::from_secs(60);
/// File events are collected this long before the files are read.
pub const DEBOUNCE: Duration = Duration::from_millis(300);
/// Minimum gap between progress snapshots during the initial import.
pub const PROGRESS_INTERVAL: Duration = Duration::from_millis(400);
/// Limit snapshots older than this are deleted at start.
const LIMIT_SNAPSHOT_RETENTION_MS: i64 = 35 * 24 * 60 * 60_000;

/// Time zone for calendar boundaries.
#[derive(Debug, Clone, Copy)]
pub enum ClockZone {
    /// The system's local time zone (the app default).
    Local,
    /// A fixed UTC offset (tests).
    Fixed(FixedOffset),
}

/// Settings of the ingest worker.
#[derive(Debug, Clone)]
pub struct IngestConfig {
    /// Directory searched recursively for `*.jsonl` (Claude Code's projects folder).
    pub logs_root: PathBuf,
    pub rescan_interval: Duration,
    pub tick_interval: Duration,
    pub debounce: Duration,
    pub zone: ClockZone,
}

impl IngestConfig {
    /// Default timing for the given log root.
    pub fn new(logs_root: PathBuf) -> Self {
        Self {
            logs_root,
            rescan_interval: RESCAN_INTERVAL,
            tick_interval: TICK_INTERVAL,
            debounce: DEBOUNCE,
            zone: ClockZone::Local,
        }
    }

    /// Config for `<home>/.claude/projects`; `None` if the home directory is unknown.
    pub fn standard() -> Option<Self> {
        projects_dir().map(Self::new)
    }
}

enum Msg {
    Fs(notify::Result<notify::Event>),
    Limit(LimitPoll),
    Rescan,
    Recompute,
    Stop,
}

/// Handle to the running worker. Keep it in Tauri managed state.
pub struct IngestHandle {
    tx: Sender<Msg>,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl IngestHandle {
    /// Scans the log folder now instead of waiting for the next interval.
    pub fn rescan(&self) {
        let _ = self.tx.send(Msg::Rescan);
    }

    /// Recomputes and publishes the snapshot (cause `Requested`), for
    /// example after a settings change. The mode is read from the database.
    pub fn recompute(&self) {
        let _ = self.tx.send(Msg::Recompute);
    }

    /// Asks the worker to stop; it also aborts a running import between files.
    /// Never blocks.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = self.tx.send(Msg::Stop);
    }

    /// Stops the worker and waits for the thread to end.
    pub fn stop_and_wait(mut self) {
        self.stop();
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for IngestHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Starts the ingest worker.
///
/// * `db`: the write connection; the worker owns it from now on. Other
///   threads open their own read connections with `Db::open`.
/// * `limits_rx`: the receiving end of the channel given to `limits::spawn`.
/// * `publisher`: receives snapshots and new usage, see [`IngestPublisher`].
///
/// The first thing the worker does is the initial scan; its result arrives
/// as a snapshot with cause `ImportDone`.
pub fn spawn_ingest(
    config: IngestConfig,
    db: Db,
    limits_rx: Receiver<LimitPoll>,
    publisher: Box<dyn IngestPublisher + Send>,
) -> io::Result<IngestHandle> {
    let (tx, rx) = channel();
    let stop = Arc::new(AtomicBool::new(false));

    let forward = tx.clone();
    thread::Builder::new()
        .name("ingest-limits-fwd".into())
        .spawn(move || {
            for poll in limits_rx {
                if forward.send(Msg::Limit(poll)).is_err() {
                    break;
                }
            }
        })?;

    let worker = Worker::new(config, db, publisher, tx.clone(), Arc::clone(&stop));
    let join = thread::Builder::new()
        .name("ingest-worker".into())
        .spawn(move || worker.run(rx))?;
    Ok(IngestHandle {
        tx,
        stop,
        join: Some(join),
    })
}

fn now_ms() -> EpochMs {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

struct Status {
    state: DataState,
    files_total: u32,
    files_done: u32,
    detail: Option<String>,
}

struct Worker {
    cfg: IngestConfig,
    db: Db,
    publisher: Box<dyn IngestPublisher + Send>,
    limits: LimitState,
    status: Status,
    tx: Sender<Msg>,
    stop: Arc<AtomicBool>,
    watcher: Option<RecommendedWatcher>,
    /// Canonical form of `cfg.logs_root` while it exists; every path the
    /// worker stores or reads is derived from it, so `source_files` keys
    /// never split between spellings of the same file.
    root: Option<PathBuf>,
    dirty: HashSet<PathBuf>,
    dirty_at: Option<Instant>,
    next_rescan: Instant,
    next_tick: Instant,
    last_progress: Instant,
}

impl Worker {
    fn new(
        cfg: IngestConfig,
        db: Db,
        publisher: Box<dyn IngestPublisher + Send>,
        tx: Sender<Msg>,
        stop: Arc<AtomicBool>,
    ) -> Self {
        let now = Instant::now();
        Self {
            next_rescan: now + cfg.rescan_interval,
            next_tick: now + cfg.tick_interval,
            cfg,
            db,
            publisher,
            limits: LimitState::default(),
            status: Status {
                state: DataState::Importing,
                files_total: 0,
                files_done: 0,
                detail: None,
            },
            tx,
            stop,
            watcher: None,
            root: None,
            dirty: HashSet::new(),
            dirty_at: None,
            last_progress: now,
        }
    }

    fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    fn run(mut self, rx: Receiver<Msg>) {
        self.start();
        while !self.stopped() {
            let wake = [Some(self.next_rescan), Some(self.next_tick), self.dirty_at]
                .into_iter()
                .flatten()
                .min()
                .unwrap_or(self.next_tick);
            match rx.recv_timeout(wake.saturating_duration_since(Instant::now())) {
                Ok(Msg::Stop) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(msg) => self.handle(msg),
                Err(RecvTimeoutError::Timeout) => {}
            }
            let now = Instant::now();
            if self.dirty_at.is_some_and(|t| t <= now) {
                self.process_dirty();
            }
            if now >= self.next_rescan {
                self.next_rescan = now + self.cfg.rescan_interval;
                if self.scan(false) {
                    self.publish(SnapshotCause::Files);
                }
            }
            if now >= self.next_tick {
                self.next_tick = now + self.cfg.tick_interval;
                self.publish(SnapshotCause::Tick);
            }
        }
    }

    fn start(&mut self) {
        match self.db.latest_limit_snapshot() {
            Ok(stored) => self.limits = LimitState::from_stored(stored),
            Err(e) => log::warn!("ingest: cannot load the last limit reading: {e}"),
        }
        if let Err(e) = self
            .db
            .prune_limit_snapshots(now_ms() - LIMIT_SNAPSHOT_RETENTION_MS)
        {
            log::warn!("ingest: cannot prune limit snapshots: {e}");
        }
        self.scan(true);
        if !self.stopped() {
            self.publish(SnapshotCause::ImportDone);
        }
    }

    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Fs(Ok(event)) => self.on_fs_event(event),
            Msg::Fs(Err(e)) => {
                log::warn!("ingest: file watcher error: {e}");
                self.rescan_soon();
            }
            Msg::Limit(poll) => self.on_limit_poll(&poll),
            Msg::Rescan => {
                self.next_rescan = Instant::now() + self.cfg.rescan_interval;
                if self.scan(false) {
                    self.publish(SnapshotCause::Files);
                }
            }
            Msg::Recompute => self.publish(SnapshotCause::Requested),
            Msg::Stop => {}
        }
    }

    fn on_fs_event(&mut self, event: notify::Event) {
        if event.need_rescan() {
            self.rescan_soon();
        }
        let mut any = false;
        for path in event.paths {
            let path = self.normalize(&path);
            if is_jsonl(&path) {
                self.dirty.insert(path);
                any = true;
            } else if matches!(event.kind, notify::EventKind::Create(_)) && path.is_dir() {
                // A new project folder: its files may predate the watch.
                self.rescan_soon();
            }
        }
        if any && self.dirty_at.is_none() {
            self.dirty_at = Some(Instant::now() + self.cfg.debounce);
        }
    }

    fn rescan_soon(&mut self) {
        let soon = Instant::now() + self.cfg.debounce;
        self.next_rescan = self.next_rescan.min(soon);
    }

    fn on_limit_poll(&mut self, poll: &LimitPoll) {
        if let Some(reading) = self.limits.apply(poll) {
            if let Err(e) = self.db.insert_limit_snapshot(poll.source, reading) {
                log::warn!("ingest: cannot store the limit reading: {e}");
            }
        }
        self.publish(SnapshotCause::Limits);
    }

    fn process_dirty(&mut self) {
        self.dirty_at = None;
        let paths: Vec<PathBuf> = self.dirty.drain().collect();
        let mut changed = false;
        for path in paths {
            if self.stopped() {
                return;
            }
            changed |= self.ingest(&path, true);
        }
        if changed {
            if self.status.state == DataState::NoLogs {
                self.status.state = DataState::Ready;
            }
            self.publish(SnapshotCause::Files);
        }
    }

    /// Ingests one file; returns whether anything changed.
    fn ingest(&mut self, path: &std::path::Path, live: bool) -> bool {
        let publisher = &mut self.publisher;
        let outcome = ingest_file(&mut self.db, path, now_ms(), &mut |db, records| {
            publisher.on_new_usage(db, records, live);
        });
        if let super::ingest::FileOutcome::Failed(e) = &outcome {
            log::warn!("ingest: {}: {e}", path.display());
        }
        outcome.changed()
    }

    /// The folder to scan and watch: canonical when it exists.
    fn root(&self) -> &std::path::Path {
        self.root.as_deref().unwrap_or(&self.cfg.logs_root)
    }

    /// Re-resolves the canonical root; a changed root drops the watcher.
    fn refresh_root(&mut self) {
        let canonical = std::fs::canonicalize(&self.cfg.logs_root).ok();
        if canonical != self.root {
            self.root = canonical;
            self.watcher = None;
        }
    }

    /// Maps an event path to the canonical spelling used for storage. The
    /// file may already be gone, so the parent is resolved as a fallback.
    fn normalize(&self, path: &std::path::Path) -> PathBuf {
        if self.root.as_deref().is_some_and(|r| path.starts_with(r)) {
            return path.to_path_buf();
        }
        if let Ok(p) = std::fs::canonicalize(path) {
            return p;
        }
        match (path.parent().map(std::fs::canonicalize), path.file_name()) {
            (Some(Ok(parent)), Some(name)) => parent.join(name),
            _ => path.to_path_buf(),
        }
    }

    fn ensure_watch(&mut self) {
        if self.watcher.is_some() || self.root.is_none() {
            return;
        }
        let tx = self.tx.clone();
        let created = notify::recommended_watcher(move |res| {
            let _ = tx.send(Msg::Fs(res));
        });
        match created {
            Ok(mut w) => match w.watch(self.root(), RecursiveMode::Recursive) {
                Ok(()) => self.watcher = Some(w),
                Err(e) => log::warn!("ingest: cannot watch the log folder: {e}"),
            },
            Err(e) => log::warn!("ingest: cannot create a file watcher: {e}"),
        }
    }

    /// Full scan: lists the folder, reads whatever is new, forgets deleted
    /// files. `startup` marks the first scan, which reports `importing`.
    /// Returns whether the data or the status changed.
    fn scan(&mut self, startup: bool) -> bool {
        // Watch first so nothing written during the scan is missed.
        self.refresh_root();
        self.ensure_watch();
        let before = self.status.state;
        let files = match discover(self.root()) {
            Ok(f) => f,
            Err(e) => {
                self.watcher = None;
                self.status = match e {
                    DiscoverError::Missing => Status {
                        state: DataState::NoLogs,
                        files_total: 0,
                        files_done: 0,
                        detail: None,
                    },
                    DiscoverError::Unreadable(d) => Status {
                        state: DataState::Error,
                        files_total: 0,
                        files_done: 0,
                        detail: Some(d),
                    },
                };
                return self.status.state != before;
            }
        };

        let work: Vec<&PathBuf> = files.iter().filter(|p| needs_work(&self.db, p)).collect();
        // The first scan, or logs showing up after being absent, shows import
        // progress; only the startup scan reports its usage as not live.
        let importing =
            !work.is_empty() && (startup || matches!(before, DataState::NoLogs | DataState::Error));
        if importing {
            self.status = Status {
                state: DataState::Importing,
                files_total: u32::try_from(work.len()).unwrap_or(u32::MAX),
                files_done: 0,
                detail: None,
            };
            self.publish(SnapshotCause::Import);
        }

        let mut changed = false;
        for path in &work {
            if self.stopped() {
                return false;
            }
            changed |= self.ingest(path, !startup);
            if importing {
                self.status.files_done += 1;
                if self.last_progress.elapsed() >= PROGRESS_INTERVAL {
                    self.publish(SnapshotCause::Import);
                }
            }
        }

        match self.db.list_source_files() {
            Ok(stored) => {
                let present: HashSet<String> = files
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                for gone in stored.iter().filter(|s| !present.contains(&s.path)) {
                    if let Err(e) = self.db.remove_source_file(&gone.path) {
                        log::warn!("ingest: cannot forget {}: {e}", gone.path);
                    }
                }
            }
            Err(e) => log::warn!("ingest: cannot list source files: {e}"),
        }

        let total = u32::try_from(files.len()).unwrap_or(u32::MAX);
        self.status = Status {
            state: if files.is_empty() {
                DataState::NoLogs
            } else {
                DataState::Ready
            },
            files_total: total,
            files_done: total,
            detail: None,
        };
        changed || self.status.state != before
    }

    fn data_status(&self) -> DataStatus {
        DataStatus {
            state: self.status.state,
            files_total: self.status.files_total,
            files_done: self.status.files_done,
            last_message_at: None,
            detail: self.status.detail.clone(),
        }
    }

    fn publish(&mut self, cause: SnapshotCause) {
        self.last_progress = Instant::now();
        let now = now_ms();
        let mode = match self.db.settings() {
            Ok(s) => s.mode,
            Err(e) => {
                log::warn!("ingest: cannot read settings: {e}");
                Mode::NoneYet
            }
        };
        let data = self.data_status();
        let result = match self.cfg.zone {
            ClockZone::Local => snapshot(&self.db, &self.limits, mode, data, now, &Local),
            ClockZone::Fixed(offset) => snapshot(&self.db, &self.limits, mode, data, now, &offset),
        };
        match result {
            Ok(snap) => self.publisher.on_snapshot(&mut self.db, &snap, cause),
            Err(e) => log::error!("ingest: cannot build the snapshot: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ingest::tests::line;
    use super::*;
    use crate::contracts::dashboard::DashboardSnapshot;
    use crate::contracts::limits::{
        LimitError, LimitReading, LimitReason, LimitStatus, LimitWindow,
    };
    use crate::logs::UsageRecord;
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    const WAIT: Duration = Duration::from_secs(15);

    enum Event {
        Snapshot(Box<DashboardSnapshot>, SnapshotCause),
        Usage { records: usize, live: bool },
    }

    struct TestPublisher(Sender<Event>);

    impl IngestPublisher for TestPublisher {
        fn on_new_usage(&mut self, _db: &mut Db, records: &[UsageRecord], live: bool) {
            let _ = self.0.send(Event::Usage {
                records: records.len(),
                live,
            });
        }

        fn on_snapshot(&mut self, _db: &mut Db, s: &DashboardSnapshot, cause: SnapshotCause) {
            let _ = self.0.send(Event::Snapshot(Box::new(s.clone()), cause));
        }
    }

    struct Rig {
        handle: IngestHandle,
        events: Receiver<Event>,
        limits: Sender<LimitPoll>,
        /// `(records, live)` of every usage event seen while waiting.
        usage: std::cell::RefCell<Vec<(usize, bool)>>,
    }

    fn config(root: &Path) -> IngestConfig {
        IngestConfig {
            logs_root: root.to_path_buf(),
            rescan_interval: Duration::from_millis(300),
            tick_interval: Duration::from_millis(250),
            debounce: Duration::from_millis(40),
            zone: ClockZone::Fixed(FixedOffset::east_opt(0).unwrap()),
        }
    }

    fn start(root: &Path, db: Db) -> Rig {
        let (ptx, events) = channel();
        let (limits, lrx) = channel();
        let handle = spawn_ingest(config(root), db, lrx, Box::new(TestPublisher(ptx))).unwrap();
        Rig {
            handle,
            events,
            limits,
            usage: Default::default(),
        }
    }

    impl Rig {
        /// Waits for the first snapshot matching `pred`.
        fn wait_snapshot(
            &self,
            what: &str,
            mut pred: impl FnMut(&DashboardSnapshot, SnapshotCause) -> bool,
        ) -> DashboardSnapshot {
            let deadline = Instant::now() + WAIT;
            loop {
                let left = deadline.saturating_duration_since(Instant::now());
                match self.events.recv_timeout(left) {
                    Ok(Event::Snapshot(s, cause)) if pred(&s, cause) => return *s,
                    Ok(Event::Usage { records, live }) => {
                        self.usage.borrow_mut().push((records, live));
                    }
                    Ok(Event::Snapshot(..)) => {}
                    Err(_) => panic!("timed out waiting for: {what}"),
                }
            }
        }
    }

    fn session_total(s: &DashboardSnapshot) -> u64 {
        s.session.as_ref().map_or(0, |x| x.tokens.total)
    }

    fn write_lines(path: &Path, lines: &[String]) {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).unwrap();
        }
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
    }

    fn recent(offset_secs: i64) -> i64 {
        now_ms() - offset_secs * 1000
    }

    #[test]
    fn initial_import_reports_progress_then_ready() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("projects");
        write_lines(
            &root.join("proj-a/s1.jsonl"),
            &[line("m1", "s1", recent(120), 10)],
        );
        write_lines(
            &root.join("proj-b/s2.jsonl"),
            &[line("m2", "s2", recent(60), 20)],
        );
        let rig = start(&root, Db::open_in_memory().unwrap());
        let ready = rig.wait_snapshot("import done", |_, c| c == SnapshotCause::ImportDone);
        assert_eq!(ready.data.state, DataState::Ready);
        assert_eq!((ready.data.files_total, ready.data.files_done), (2, 2));
        // The newest session wins: s2 with output 20 + input 1.
        assert_eq!(ready.session.as_ref().unwrap().session_id, "s2");
        assert_eq!(session_total(&ready), 21);
        assert!(ready.data.last_message_at.is_some());
        rig.handle.stop_and_wait();
    }

    #[test]
    fn import_progress_is_visible_and_not_live() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("projects");
        write_lines(&root.join("p/a.jsonl"), &[line("m1", "s", recent(10), 5)]);
        let rig = start(&root, Db::open_in_memory().unwrap());
        let started = rig.wait_snapshot("importing", |s, c| {
            c == SnapshotCause::Import && s.data.state == DataState::Importing
        });
        assert_eq!(started.data.files_total, 1);
        rig.wait_snapshot("done", |_, c| c == SnapshotCause::ImportDone);
        // The import's usage is reported as not live.
        assert_eq!(*rig.usage.borrow(), [(1, false)]);
        rig.handle.stop_and_wait();
    }

    #[test]
    fn appended_lines_arrive_via_events_or_rescan() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("projects");
        let file = root.join("p/a.jsonl");
        write_lines(&file, &[line("m1", "s", recent(30), 10)]);
        let rig = start(&root, Db::open_in_memory().unwrap());
        let first = rig.wait_snapshot("import done", |_, c| c == SnapshotCause::ImportDone);
        assert_eq!(session_total(&first), 11);

        write_lines(&file, &[line("m2", "s", recent(5), 40)]);
        let s = rig.wait_snapshot("live update", |s, c| {
            c == SnapshotCause::Files && session_total(s) == 52
        });
        assert_eq!(s.session.as_ref().unwrap().tokens.cache_read, 14);
        // The new read reported live usage.
        assert_eq!(rig.usage.borrow().last(), Some(&(1, true)));
        rig.handle.stop_and_wait();
    }

    #[test]
    fn restart_resumes_from_stored_offsets() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("projects");
        let file = root.join("p/a.jsonl");
        let db_path = dir.path().join("t.db");
        write_lines(&file, &[line("m1", "s", recent(30), 10)]);

        let rig = start(&root, Db::open(&db_path).unwrap());
        rig.wait_snapshot("first import", |_, c| c == SnapshotCause::ImportDone);
        rig.handle.stop_and_wait();

        write_lines(&file, &[line("m2", "s", recent(5), 40)]);
        let rig = start(&root, Db::open(&db_path).unwrap());
        let s = rig.wait_snapshot("second import", |_, c| c == SnapshotCause::ImportDone);
        assert_eq!(session_total(&s), 52);
        // Only the appended line was parsed again.
        let parsed: usize = rig.usage.borrow().iter().map(|(n, _)| n).sum();
        assert_eq!(parsed, 1);
        rig.handle.stop_and_wait();
    }

    #[test]
    fn missing_log_folder_is_no_logs_until_it_appears() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("projects");
        let rig = start(&root, Db::open_in_memory().unwrap());
        let s = rig.wait_snapshot("no logs", |_, c| c == SnapshotCause::ImportDone);
        assert_eq!(s.data.state, DataState::NoLogs);
        assert_eq!(s.today.total, 0);

        write_lines(&root.join("p/a.jsonl"), &[line("m1", "s", recent(5), 9)]);
        let s = rig.wait_snapshot("logs appear", |s, _| {
            s.data.state == DataState::Ready && session_total(s) == 10
        });
        assert_eq!(s.data.files_total, 1);
        // Logs that show up after the startup scan are live usage.
        assert_eq!(rig.usage.borrow().last(), Some(&(1, true)));
        assert!(rig.usage.borrow().iter().all(|&(_, live)| live));
        rig.handle.stop_and_wait();
    }

    #[test]
    fn root_spelling_does_not_split_source_file_keys() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("projects");
        let file = real.join("p/a.jsonl");
        write_lines(&file, &[line("m1", "s", recent(30), 10)]);
        let db_path = dir.path().join("t.db");
        // Same folder, spelled with a `..` detour.
        let odd = dir.path().join("elsewhere/../projects");
        fs::create_dir_all(dir.path().join("elsewhere")).unwrap();
        let rig = start(&odd, Db::open(&db_path).unwrap());
        rig.wait_snapshot("import", |_, c| c == SnapshotCause::ImportDone);
        write_lines(&file, &[line("m2", "s", recent(5), 40)]);
        rig.wait_snapshot("live update", |s, c| {
            c == SnapshotCause::Files && session_total(s) == 52
        });
        rig.handle.stop_and_wait();

        let keys = Db::open(&db_path).unwrap().list_source_files().unwrap();
        assert_eq!(keys.len(), 1, "{keys:?}");
        assert!(!keys[0].path.contains(".."), "{}", keys[0].path);
    }

    #[test]
    fn limit_polls_update_the_view() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("projects");
        let rig = start(&root, Db::open_in_memory().unwrap());
        rig.wait_snapshot("start", |_, c| c == SnapshotCause::ImportDone);

        rig.limits
            .send(LimitPoll {
                source: "test",
                result: Ok(LimitReading {
                    five_hour: Some(LimitWindow {
                        utilization: 33.0,
                        resets_at: Some(now_ms() + 3_600_000),
                    }),
                    weekly: None,
                    fetched_at: now_ms(),
                }),
            })
            .unwrap();
        let s = rig.wait_snapshot("limit ok", |_, c| c == SnapshotCause::Limits);
        assert_eq!(s.limits.status, LimitStatus::Ok);
        assert_eq!(s.limits.five_hour.unwrap().utilization, 33.0);

        rig.limits
            .send(LimitPoll {
                source: "test",
                result: Err(LimitError {
                    reason: LimitReason::Network,
                    detail: "down".into(),
                }),
            })
            .unwrap();
        let s = rig.wait_snapshot("error keeps last reading", |_, c| {
            c == SnapshotCause::Limits
        });
        // A fresh reading stays ok despite the failed poll.
        assert_eq!(s.limits.status, LimitStatus::Ok);
        rig.handle.stop_and_wait();
    }

    #[test]
    fn last_limit_reading_survives_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("projects");
        let db_path = dir.path().join("t.db");
        let rig = start(&root, Db::open(&db_path).unwrap());
        rig.wait_snapshot("start", |_, c| c == SnapshotCause::ImportDone);
        rig.limits
            .send(LimitPoll {
                source: "test",
                result: Ok(LimitReading {
                    five_hour: Some(LimitWindow {
                        utilization: 12.0,
                        resets_at: None,
                    }),
                    weekly: None,
                    fetched_at: now_ms(),
                }),
            })
            .unwrap();
        rig.wait_snapshot("limit", |_, c| c == SnapshotCause::Limits);
        rig.handle.stop_and_wait();

        let rig = start(&root, Db::open(&db_path).unwrap());
        let s = rig.wait_snapshot("restart", |_, c| c == SnapshotCause::ImportDone);
        assert_eq!(s.limits.status, LimitStatus::Ok);
        assert_eq!(s.limits.five_hour.unwrap().utilization, 12.0);
        rig.handle.stop_and_wait();
    }

    #[test]
    fn tick_and_requested_snapshots_are_published() {
        let dir = tempfile::tempdir().unwrap();
        let rig = start(&dir.path().join("projects"), Db::open_in_memory().unwrap());
        rig.wait_snapshot("tick", |_, c| c == SnapshotCause::Tick);
        rig.handle.recompute();
        rig.wait_snapshot("requested", |_, c| c == SnapshotCause::Requested);
        rig.handle.stop_and_wait();
    }
}
