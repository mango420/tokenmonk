//! The [`IngestPublisher`] of the app: caches every snapshot, emits events,
//! updates the tray halo and runs the game evaluation.
//!
//! Runs on the ingest worker thread (docs/ARCHITECTURE.md, "Threads"). The
//! outside world (events, tray) sits behind [`Sink`] so the logic is
//! testable with an in-memory database and a recording sink.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Local, TimeZone};

use super::facts::{build_facts, BudgetCache, Scope};
use super::game_store::DbGameStore;
use crate::contracts::EpochMs;
use crate::contracts::{
    AchievementUnlocked, AppInfo, DashboardSnapshot, DataState, GameState, LevelUp, Mode,
};
use crate::db::{Db, DbError};
use crate::game::{run_evaluation, Evaluation, Facts, GameStore};
use crate::logs::UsageRecord;
use crate::watcher::{ClockZone, IngestPublisher, SnapshotCause};

use super::state::AppState;

/// Where the publisher sends its results.
pub trait Sink: Send {
    fn dashboard(&self, snapshot: &DashboardSnapshot);
    fn game(&self, state: &GameState);
    fn achievement(&self, unlocked: &AchievementUnlocked);
    fn level_up(&self, level_up: &LevelUp);
    fn app_info(&self, info: &AppInfo);
    fn halo(&self, percent: Option<f64>, mode: Mode);
}

/// Tray halo value for a snapshot: the 5-hour percent LEFT, in every mode
/// (docs/DESIGN.md, "Tray icon"). The arc drains as the window is used.
/// `None` (no limit data) shows the dotted track only.
pub fn halo_percent(snapshot: &DashboardSnapshot) -> Option<f64> {
    snapshot
        .limits
        .five_hour
        .as_ref()
        .map(|w| w.utilization)
        .filter(|u| u.is_finite())
        .map(|u| (100.0 - u).clamp(0.0, 100.0))
}

pub struct AppPublisher<S: Sink> {
    sink: S,
    state: Arc<AppState>,
    zone: ClockZone,
    /// The startup scan is over; game evaluation starts.
    import_done: bool,
    /// Oldest timestamp of usage seen since the last successful evaluation.
    pending_from: Option<EpochMs>,
    budget_cache: BudgetCache,
}

impl<S: Sink> AppPublisher<S> {
    pub fn new(sink: S, state: Arc<AppState>, zone: ClockZone) -> Self {
        Self {
            sink,
            state,
            zone,
            import_done: false,
            pending_from: None,
            budget_cache: HashMap::new(),
        }
    }

    /// Keeps `AppInfo` current (logs found) and announces changes.
    fn refresh_info(&self, snapshot: &DashboardSnapshot) {
        let mut info = self.state.info();
        let logs_found = snapshot.data.state != DataState::NoLogs;
        if info.logs_found != logs_found {
            info.logs_found = logs_found;
            self.state.set_info(info.clone());
            self.sink.app_info(&info);
        }
    }

    fn evaluate(&mut self, db: &Db, snapshot: &DashboardSnapshot, mode: Mode) {
        match self.zone {
            ClockZone::Local => self.evaluate_in(db, snapshot, mode, &Local),
            ClockZone::Fixed(offset) => self.evaluate_in(db, snapshot, mode, &offset),
        }
    }

    fn evaluate_in<Tz: TimeZone>(
        &mut self,
        db: &Db,
        snapshot: &DashboardSnapshot,
        mode: Mode,
        tz: &Tz,
    ) {
        let state = Arc::clone(&self.state);
        let _guard = state.game_lock.lock().unwrap_or_else(|e| e.into_inner());
        match self.try_evaluate(db, snapshot, mode, tz) {
            Ok((evaluation, retroactive)) => {
                self.pending_from = None;
                self.announce(&evaluation, retroactive);
            }
            Err(e) => log::warn!("game evaluation failed: {e}"),
        }
    }

    /// Loads, evaluates and persists. Returns the evaluation and whether it
    /// was the retroactive replay.
    fn try_evaluate<Tz: TimeZone>(
        &mut self,
        db: &Db,
        snapshot: &DashboardSnapshot,
        mode: Mode,
        tz: &Tz,
    ) -> Result<(Evaluation, bool), DbError> {
        let now = snapshot.generated_at;
        let mut store = DbGameStore::new(db);
        let retroactive = mode != Mode::NoneYet && !store.load()?.retro_done;
        let facts = if mode == Mode::NoneYet {
            Facts::default()
        } else {
            let scope = if retroactive {
                Scope::Replay
            } else {
                Scope::Live {
                    touched: self.pending_from,
                }
            };
            let today_budget = snapshot.daily_budget.as_ref().map(|b| b.used_percent);
            build_facts(
                db,
                mode,
                scope,
                now,
                today_budget,
                &mut self.budget_cache,
                tz,
            )?
        };
        let evaluation = run_evaluation(&mut store, &facts, mode, now, tz, retroactive)?;
        Ok((evaluation, retroactive))
    }

    fn announce(&self, evaluation: &Evaluation, retroactive: bool) {
        // The replay stays quiet: one `game-updated` with `retroDone` follows.
        if !retroactive {
            for unlocked in &evaluation.unlocks {
                self.sink.achievement(unlocked);
            }
            if let Some(level_up) = &evaluation.level_up {
                self.sink.level_up(level_up);
            }
        }
        let changed = self.state.game().as_ref() != Some(&evaluation.state);
        self.state.set_game(evaluation.state.clone());
        if changed || retroactive {
            self.sink.game(&evaluation.state);
        }
    }
}

impl<S: Sink> IngestPublisher for AppPublisher<S> {
    fn on_new_usage(&mut self, _db: &mut Db, records: &[UsageRecord], _live: bool) {
        // Backlog read at startup counts as well: days that gained usage
        // while the app was closed must be complete in the next evaluation.
        if let Some(oldest) = records.iter().map(|r| r.ts).min() {
            self.pending_from = Some(self.pending_from.map_or(oldest, |p| p.min(oldest)));
        }
    }

    fn on_snapshot(&mut self, db: &mut Db, snapshot: &DashboardSnapshot, cause: SnapshotCause) {
        self.state.set_dashboard(snapshot.clone());
        self.sink.dashboard(snapshot);
        self.refresh_info(snapshot);

        let mode = self.state.settings().mode;
        self.sink.halo(halo_percent(snapshot), mode);

        match cause {
            SnapshotCause::Import => {}
            SnapshotCause::ImportDone => {
                self.import_done = true;
                self.evaluate(db, snapshot, mode);
            }
            _ if self.import_done => self.evaluate(db, snapshot, mode),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }
    use crate::aggregate::{snapshot, LimitState};
    use crate::contracts::{
        AchievementId, DataStatus, LimitReading, LimitWindow, Platform, Settings,
    };
    use crate::limits::LimitPoll;
    use chrono::FixedOffset;
    use std::sync::Mutex;

    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<String>>>);

    impl Recorder {
        fn log(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }
        fn push(&self, s: String) {
            self.0.lock().unwrap().push(s);
        }
        fn count(&self, prefix: &str) -> usize {
            self.log().iter().filter(|e| e.starts_with(prefix)).count()
        }
    }

    impl Sink for Recorder {
        fn dashboard(&self, _: &DashboardSnapshot) {
            self.push("dashboard".into());
        }
        fn game(&self, g: &GameState) {
            self.push(format!("game retro={} level={}", g.retro_done, g.level));
        }
        fn achievement(&self, a: &AchievementUnlocked) {
            self.push(format!("achievement {}", a.id.as_str()));
        }
        fn level_up(&self, l: &LevelUp) {
            self.push(format!("levelUp {}", l.level));
        }
        fn app_info(&self, i: &AppInfo) {
            self.push(format!("info logs={}", i.logs_found));
        }
        fn halo(&self, p: Option<f64>, m: Mode) {
            self.push(format!("halo {p:?} {m:?}"));
        }
    }

    fn tz() -> FixedOffset {
        FixedOffset::east_opt(0).unwrap()
    }

    fn ms(d: u32, h: u32) -> EpochMs {
        date(2026, 10, d)
            .and_hms_opt(h, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis()
    }

    fn setup(mode: Mode) -> (AppPublisher<Recorder>, Recorder, Arc<AppState>, Db) {
        let info = AppInfo {
            version: "0".into(),
            platform: Platform::Windows,
            schema_version: 1,
            logs_found: true,
        };
        let state = Arc::new(AppState::new(
            Db::open_in_memory().unwrap(),
            Settings {
                mode,
                ..Settings::default()
            },
            info,
        ));
        let rec = Recorder::default();
        let publisher = AppPublisher::new(rec.clone(), Arc::clone(&state), ClockZone::Fixed(tz()));
        (publisher, rec, state, Db::open_in_memory().unwrap())
    }

    fn snap(db: &Db, mode: Mode, now: EpochMs, limits: &LimitState) -> DashboardSnapshot {
        let data = DataStatus {
            state: DataState::Ready,
            files_total: 1,
            files_done: 1,
            last_message_at: None,
            detail: None,
        };
        snapshot(db, limits, mode, data, now, &tz()).unwrap()
    }

    fn msg(id: &str, ts: EpochMs, out: u64) -> UsageRecord {
        UsageRecord {
            message_id: id.into(),
            request_id: None,
            session_id: "s".into(),
            agent_id: None,
            is_sidechain: false,
            ts,
            model: "m".into(),
            project_path: "/p".into(),
            input_tokens: 10,
            output_tokens: out,
            cache_creation_tokens: 5,
            cache_read_tokens: 0,
        }
    }

    #[test]
    fn halo_is_five_hour_percent_left() {
        let db = Db::open_in_memory().unwrap();
        let now = ms(28, 12);
        let mut limits = LimitState::default();
        limits.apply(&LimitPoll {
            source: "statusline",
            result: Ok(LimitReading {
                five_hour: Some(LimitWindow {
                    utilization: 64.5,
                    resets_at: Some(now + 3_600_000),
                }),
                weekly: None,
                fetched_at: now,
            }),
        });
        let mut s = snap(&db, Mode::Max, now, &limits);
        assert_eq!(halo_percent(&s), Some(35.5), "percent left, not used");
        s.limits.five_hour = Some(LimitWindow {
            utilization: 130.0,
            resets_at: None,
        });
        assert_eq!(halo_percent(&s), Some(0.0), "overage is clamped");
        s.limits.five_hour = None;
        assert_eq!(halo_percent(&s), None, "no limit data: dotted track only");
        s.data.state = DataState::Importing;
        s.data.files_total = 4;
        s.data.files_done = 1;
        assert_eq!(halo_percent(&s), None, "import progress is not a value");
    }

    #[test]
    fn snapshot_is_cached_emitted_and_drives_the_halo() {
        let (mut p, rec, state, mut db) = setup(Mode::Max);
        let now = ms(28, 12);
        let s = snap(&db, Mode::Max, now, &LimitState::default());
        p.on_snapshot(&mut db, &s, SnapshotCause::Import);
        assert_eq!(state.dashboard().unwrap(), s);
        assert_eq!(rec.log(), ["dashboard", "halo None Max"]);
        assert!(state.game().is_none(), "no evaluation while importing");
    }

    #[test]
    fn without_a_mode_the_game_waits_but_reports_a_state() {
        let (mut p, rec, state, mut db) = setup(Mode::NoneYet);
        db.upsert_messages(&[msg("a", ms(28, 10), 100)]).unwrap();
        let s = snap(&db, Mode::NoneYet, ms(28, 12), &LimitState::default());
        p.on_snapshot(&mut db, &s, SnapshotCause::ImportDone);
        assert_eq!(state.game().unwrap().xp_total, 0);
        assert!(!state.game().unwrap().retro_done);
        assert_eq!(rec.count("achievement"), 0);
        assert_eq!(rec.count("game"), 1);
        // Same state again: no repeated event.
        p.on_snapshot(&mut db, &s, SnapshotCause::Tick);
        assert_eq!(rec.count("game"), 1);
    }

    #[test]
    fn retroactive_replay_runs_once_and_stays_quiet() {
        let (mut p, rec, state, mut db) = setup(Mode::Max);
        db.upsert_messages(&[msg("a", ms(26, 10), 300_000), msg("b", ms(27, 3), 100)])
            .unwrap();
        let s = snap(&db, Mode::Max, ms(28, 12), &LimitState::default());
        p.on_snapshot(&mut db, &s, SnapshotCause::ImportDone);

        let game = state.game().unwrap();
        assert!(game.retro_done && game.xp_total > 0);
        assert_eq!(rec.count("achievement"), 0);
        assert_eq!(rec.count("levelUp"), 0);
        assert_eq!(rec.count("game retro=true"), 1);
        let unlocked: Vec<_> = game
            .achievements
            .iter()
            .filter(|a| a.unlocked_at.is_some())
            .map(|a| a.id)
            .collect();
        assert!(unlocked.contains(&AchievementId::FirstBlood));
        assert!(unlocked.contains(&AchievementId::NightOwl));
        let xp = game.xp_total;

        // Later snapshots are live evaluations and award nothing new.
        p.on_snapshot(&mut db, &s, SnapshotCause::Tick);
        assert_eq!(state.game().unwrap().xp_total, xp);
        assert_eq!(rec.count("game"), 1);
    }

    #[test]
    fn live_usage_unlocks_and_announces() {
        let (mut p, rec, state, mut db) = setup(Mode::Max);
        let first = snap(&db, Mode::Max, ms(28, 9), &LimitState::default());
        p.on_snapshot(&mut db, &first, SnapshotCause::ImportDone);
        assert!(
            state.game().unwrap().retro_done,
            "empty replay marks it done"
        );
        assert_eq!(rec.count("achievement"), 0);

        let records = [msg("a", ms(28, 10), 250_000)];
        db.upsert_messages(&records).unwrap();
        p.on_new_usage(&mut db, &records, true);
        let s = snap(&db, Mode::Max, ms(28, 11), &LimitState::default());
        p.on_snapshot(&mut db, &s, SnapshotCause::Files);

        let log = rec.log();
        assert!(
            log.contains(&"achievement firstBlood".to_string()),
            "{log:?}"
        );
        assert!(state.game().unwrap().xp_total >= 20, "two token blocks");
        // The unlock event comes before the state it is part of.
        let unlock_at = log
            .iter()
            .position(|e| e.starts_with("achievement"))
            .unwrap();
        let game_at = log.iter().rposition(|e| e.starts_with("game")).unwrap();
        assert!(unlock_at < game_at);
        assert_eq!(p.pending_from, None);

        // Nothing new, nothing announced.
        let before = rec.log().len();
        p.on_snapshot(&mut db, &s, SnapshotCause::Tick);
        let after = rec.log();
        assert!(after[before..]
            .iter()
            .all(|e| !e.starts_with("achievement")));
        assert_eq!(rec.count("achievement"), 1);
    }

    #[test]
    fn missing_logs_update_app_info() {
        let (p, rec, state, db) = setup(Mode::Max);
        let mut p = p;
        let mut db2 = db;
        let mut s = snap(&db2, Mode::Max, ms(28, 12), &LimitState::default());
        s.data.state = DataState::NoLogs;
        p.on_snapshot(&mut db2, &s, SnapshotCause::Import);
        assert!(!state.info().logs_found);
        assert_eq!(rec.count("info logs=false"), 1);
        // No change, no event.
        p.on_snapshot(&mut db2, &s, SnapshotCause::Import);
        assert_eq!(rec.count("info"), 1);
    }
}
