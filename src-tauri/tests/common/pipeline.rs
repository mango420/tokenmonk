//! A small stand-in for the running app: the real ingest function, the real
//! snapshot aggregate, the real `AppPublisher` (game included) and a
//! recording sink instead of Tauri events. Everything runs on the calling
//! thread with explicit clocks, so the tests are deterministic.
//!
//! Also writes synthetic log lines in the shape of docs/LOG-FORMAT.md.
#![allow(dead_code)]

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{FixedOffset, TimeZone};
use tokenmonk_lib::aggregate::{snapshot, LimitState};
use tokenmonk_lib::app::{AppPublisher, AppState, Sink};
use tokenmonk_lib::contracts::limits::{LimitError, LimitReading};
use tokenmonk_lib::contracts::{
    AchievementUnlocked, AppInfo, DashboardSnapshot, DataState, DataStatus, EpochMs, GameState,
    LevelUp, Mode, Platform, Season, Settings,
};
use tokenmonk_lib::db::Db;
use tokenmonk_lib::limits::LimitPoll;
use tokenmonk_lib::watcher::{ingest_file, ClockZone, IngestPublisher, SnapshotCause};

/// What the app would have sent to the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Dashboard,
    Game {
        retro_done: bool,
        level: u32,
    },
    Achievement {
        id: String,
        retroactive: bool,
    },
    LevelUp {
        level: u32,
        previous: u32,
        retroactive: bool,
    },
    AppInfo,
    Halo(Option<f64>),
}

#[derive(Clone, Default)]
pub struct Recorder(Arc<Mutex<Vec<Event>>>);

impl Recorder {
    pub fn events(&self) -> Vec<Event> {
        self.0.lock().unwrap().clone()
    }

    /// Ids of the `achievement-unlocked` events so far, in order.
    pub fn achievements(&self) -> Vec<String> {
        self.events()
            .into_iter()
            .filter_map(|e| match e {
                Event::Achievement { id, .. } => Some(id),
                _ => None,
            })
            .collect()
    }

    pub fn level_ups(&self) -> Vec<Event> {
        self.events()
            .into_iter()
            .filter(|e| matches!(e, Event::LevelUp { .. }))
            .collect()
    }

    pub fn game_updates(&self) -> usize {
        self.events()
            .iter()
            .filter(|e| matches!(e, Event::Game { .. }))
            .count()
    }

    fn push(&self, e: Event) {
        self.0.lock().unwrap().push(e);
    }
}

impl Sink for Recorder {
    fn dashboard(&self, _: &DashboardSnapshot) {
        self.push(Event::Dashboard);
    }
    fn game(&self, g: &GameState) {
        self.push(Event::Game {
            retro_done: g.retro_done,
            level: g.level,
        });
    }
    fn achievement(&self, a: &AchievementUnlocked) {
        self.push(Event::Achievement {
            id: a.id.as_str().to_owned(),
            retroactive: a.retroactive,
        });
    }
    fn level_up(&self, l: &LevelUp) {
        self.push(Event::LevelUp {
            level: l.level,
            previous: l.previous_level,
            retroactive: l.retroactive,
        });
    }
    fn app_info(&self, _: &AppInfo) {
        self.push(Event::AppInfo);
    }
    fn halo(&self, percent: Option<f64>, _: Season) {
        self.push(Event::Halo(percent));
    }
}

pub fn utc() -> FixedOffset {
    FixedOffset::east_opt(0).unwrap()
}

fn app_info() -> AppInfo {
    AppInfo {
        version: "0.0.0-test".into(),
        platform: Platform::Windows,
        season: Season::Regular,
        schema_version: 1,
        logs_found: true,
    }
}

/// The app minus Tauri.
pub struct Rig {
    pub db: Db,
    pub publisher: AppPublisher<Recorder>,
    pub state: Arc<AppState>,
    pub rec: Recorder,
    pub limits: LimitState,
    db_path: PathBuf,
    _dir: tempfile::TempDir,
}

impl Rig {
    pub fn new(mode: Mode) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("tokenmonk-e2e.db");
        let db = Db::open(&db_path).unwrap();
        let mut rig = Self {
            db,
            publisher: placeholder(),
            state: placeholder_state(),
            rec: Recorder::default(),
            limits: LimitState::default(),
            db_path,
            _dir: dir,
        };
        rig.boot(mode);
        rig
    }

    /// (Re)creates everything that lives in memory: state, publisher, sink,
    /// limit knowledge restored from the stored readings. The database file
    /// stays, as it does across an app restart.
    fn boot(&mut self, mode: Mode) {
        let state = Arc::new(AppState::new(
            Db::open(&self.db_path).unwrap(),
            Settings {
                mode,
                ..Settings::default()
            },
            app_info(),
        ));
        self.rec = Recorder::default();
        self.publisher = AppPublisher::new(
            self.rec.clone(),
            Arc::clone(&state),
            ClockZone::Fixed(utc()),
        )
        .with_season(|| Season::Regular);
        self.state = state;
        self.limits = LimitState::from_stored(self.db.latest_limit_snapshot().unwrap());
    }

    /// Simulates quitting and starting the app again on the same database.
    pub fn restart(&mut self) {
        let mode = self.state.settings().mode;
        self.db = Db::open(&self.db_path).unwrap();
        self.boot(mode);
    }

    /// The user picks a mode (onboarding or settings).
    pub fn set_mode(&self, mode: Mode) {
        self.state.set_settings(Settings {
            mode,
            ..self.state.settings()
        });
    }

    /// Ingests `files` through the real incremental reader and tells the
    /// publisher about the records, as the ingest worker does. `live` is
    /// `false` for the startup scan.
    pub fn scan(&mut self, files: &[PathBuf], now: EpochMs, live: bool) {
        let Rig { db, publisher, .. } = self;
        for path in files {
            ingest_file(db, path, now, &mut |db, records| {
                publisher.on_new_usage(db, records, live);
            });
        }
    }

    /// Builds a snapshot at `now` from the database and the limit knowledge,
    /// hands it to the publisher and returns it.
    pub fn snap(&mut self, now: EpochMs, cause: SnapshotCause) -> DashboardSnapshot {
        let data = DataStatus {
            state: DataState::Ready,
            files_total: 4,
            files_done: 4,
            last_message_at: None,
            detail: None,
        };
        let mode = self.state.settings().mode;
        let s = snapshot(&self.db, &self.limits, mode, data, now, &utc()).unwrap();
        self.publisher.on_snapshot(&mut self.db, &s, cause);
        s
    }

    /// One limit poll arrives: folded into the limit knowledge and, if good,
    /// stored (as the ingest worker does).
    pub fn limit(&mut self, result: Result<LimitReading, LimitError>) {
        let poll = LimitPoll {
            source: "statusline",
            result,
        };
        if let Some(reading) = self.limits.apply(&poll) {
            self.db
                .insert_limit_snapshot("statusline", reading)
                .unwrap();
        }
    }

    pub fn xp_total(&self) -> i64 {
        self.db.xp_total().unwrap()
    }

    /// `(kind, ref, amount)` of every ledger row, sorted.
    pub fn ledger(&self) -> Vec<(String, String, i64)> {
        let mut rows: Vec<_> = self
            .db
            .xp_events()
            .unwrap()
            .into_iter()
            .map(|e| (e.kind, e.reference, e.amount))
            .collect();
        rows.sort();
        rows
    }
}

fn placeholder_state() -> Arc<AppState> {
    Arc::new(AppState::new(
        Db::open_in_memory().unwrap(),
        Settings::default(),
        app_info(),
    ))
}

fn placeholder() -> AppPublisher<Recorder> {
    AppPublisher::new(Recorder::default(), placeholder_state(), ClockZone::Local)
}

// --- synthetic log files ---------------------------------------------------

pub const SESSION_HEAVY: &str = "55555555-5555-4555-8555-555555555555";

/// One assistant line (docs/LOG-FORMAT.md) with fake ids and content. `n`
/// numbers the message and request ids.
pub fn assistant_line(
    n: u32,
    session: &str,
    ts: EpochMs,
    input: u64,
    output: u64,
    cache_creation: u64,
    cache_read: u64,
) -> String {
    let stamp = chrono::Utc
        .timestamp_millis_opt(ts)
        .single()
        .unwrap()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ");
    format!(
        r#"{{"type":"assistant","isSidechain":false,"timestamp":"{stamp}","sessionId":"{session}","cwd":"/Users/example/projects/synthetic","version":"2.1.0","requestId":"req_{n:024}","message":{{"id":"msg_{n:024}","type":"message","role":"assistant","model":"claude-sonnet-5","content":[{{"type":"text","text":"lorem ipsum"}}],"stop_reason":"end_turn","usage":{{"input_tokens":{input},"cache_creation_input_tokens":{cache_creation},"cache_read_input_tokens":{cache_read},"output_tokens":{output}}}}}}}"#
    )
}

/// A message with `headline` tokens (input + output + cache creation) and a
/// huge cache read that must never count. `headline` must be at least 1000.
pub fn heavy_line(n: u32, ts: EpochMs, headline: u64) -> String {
    assert!(headline >= 1_000);
    assistant_line(n, SESSION_HEAVY, ts, 500, 500, headline - 1_000, 5_000_000)
}

/// `<root>/projects/-Users-example-projects-synthetic/<session>.jsonl`.
pub fn session_file(root: &Path, session: &str) -> PathBuf {
    root.join("projects")
        .join("-Users-example-projects-synthetic")
        .join(format!("{session}.jsonl"))
}

/// Appends complete lines to `path`, creating the file and folders.
pub fn append_lines(path: &Path, lines: &[String]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    for l in lines {
        writeln!(f, "{l}").unwrap();
    }
}
