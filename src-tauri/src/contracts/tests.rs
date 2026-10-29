//! Wire-format tests: they pin the JSON shapes that `src/lib/contracts.ts`
//! mirrors.

use serde_json::{Value, json, to_value};

use super::*;

fn sample_limits() -> LimitsView {
    LimitsView {
        status: LimitStatus::Ok,
        reason: None,
        detail: None,
        source: Some("none".into()),
        five_hour: Some(LimitWindow {
            utilization: 72.0,
            resets_at: Some(1_700_000_000_000),
        }),
        weekly: None,
        fetched_at: Some(1_700_000_000_000),
    }
}

fn sample_unlock() -> AchievementUnlocked {
    AchievementUnlocked {
        id: AchievementId::NightOwl,
        name: "Night Owl".into(),
        detail: Some("Tokens burned at 02:47".into()),
        xp: 50,
        unlocked_at: 1_700_000_000_000,
        retroactive: false,
    }
}

#[test]
fn settings_use_camel_case_strings() {
    assert_eq!(
        to_value(Settings::default()).unwrap(),
        json!({ "mode": "noneYet", "sound": true, "autostart": false })
    );
    assert_eq!(to_value(Mode::Diy).unwrap(), json!("diy"));
}

#[test]
fn settings_patch_omits_absent_fields() {
    let patch = SettingsPatch {
        sound: Some(false),
        ..Default::default()
    };
    assert_eq!(to_value(&patch).unwrap(), json!({ "sound": false }));
    let parsed: SettingsPatch = serde_json::from_value(json!({ "mode": "max" })).unwrap();
    assert_eq!(parsed.mode, Some(Mode::Max));
    assert_eq!(parsed.sound, None);
    let args: UpdateSettingsArgs =
        serde_json::from_value(json!({ "patch": { "autostart": true } })).unwrap();
    assert_eq!(args.patch.autostart, Some(true));
}

#[test]
fn dashboard_snapshot_shape() {
    let snapshot = DashboardSnapshot {
        generated_at: 1,
        today: TokenBreakdown {
            input: 1,
            output: 2,
            cache_creation: 3,
            cache_read: 4,
            total: 6,
        },
        session: None,
        burn_rate: BurnRate {
            tokens_per_minute: 1.5,
            window_minutes: 15,
            limit_eta_minutes: None,
        },
        limits: sample_limits(),
        daily_budget: Some(DailyBudget {
            basis: BudgetBasis::WeeklyLimit,
            allowance: 10.0,
            used_today: 2.3,
            used_percent: 23.0,
            days_left: 6,
            resets_at: None,
        }),
        data: DataStatus {
            state: DataState::NoLogs,
            files_total: 0,
            files_done: 0,
            last_message_at: None,
            detail: None,
        },
    };
    let v = to_value(&snapshot).unwrap();
    assert_eq!(v["today"]["cacheCreation"], json!(3));
    assert_eq!(v["burnRate"]["limitEtaMinutes"], Value::Null);
    assert_eq!(
        v["limits"]["fiveHour"]["resetsAt"],
        json!(1_700_000_000_000_i64)
    );
    assert_eq!(v["limits"]["weekly"], Value::Null);
    assert_eq!(v["dailyBudget"]["basis"], json!("weeklyLimit"));
    assert_eq!(v["data"]["state"], json!("noLogs"));
    assert_eq!(v["session"], Value::Null);
    let back: DashboardSnapshot = serde_json::from_value(v).unwrap();
    assert_eq!(back, snapshot);
}

#[test]
fn game_types_shape() {
    assert_eq!(
        to_value(AchievementId::RateLimitRambo).unwrap(),
        json!("rateLimitRambo")
    );
    for id in AchievementId::ALL {
        assert_eq!(to_value(id).unwrap(), json!(id.as_str()));
    }
    let v = to_value(EasterEggResult {
        newly_unlocked: true,
        achievement: sample_unlock(),
    })
    .unwrap();
    assert_eq!(v["newlyUnlocked"], json!(true));
    assert_eq!(v["achievement"]["unlockedAt"], json!(1_700_000_000_000_i64));
    let level = to_value(LevelUp {
        level: 8,
        previous_level: 7,
        title: "Token Squire".into(),
        title_changed: false,
        retroactive: false,
    })
    .unwrap();
    assert_eq!(level["previousLevel"], json!(7));
}

#[test]
fn app_info_and_errors_shape() {
    let info = AppInfo {
        version: "0.1.0".into(),
        platform: Platform::Macos,
        season: Season::Halloween,
        schema_version: schema::SCHEMA_VERSION,
        logs_found: true,
    };
    assert_eq!(
        to_value(info).unwrap(),
        json!({
            "version": "0.1.0",
            "platform": "macos",
            "season": "halloween",
            "schemaVersion": 1,
            "logsFound": true
        })
    );
    let err = CommandError {
        code: CommandErrorCode::InvalidArgument,
        message: "mode must be max or diy".into(),
    };
    assert_eq!(to_value(err).unwrap()["code"], json!("invalidArgument"));
    assert_eq!(
        to_value(NavigateTo {
            view: View::Settings
        })
        .unwrap(),
        json!({ "view": "settings" })
    );
}

#[test]
fn names_are_unique_and_well_formed() {
    let mut seen = std::collections::HashSet::new();
    for name in commands::ALL {
        assert!(name.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
        assert!(seen.insert(name), "duplicate command {name}");
    }
    for name in events::ALL {
        assert!(name.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
        assert!(seen.insert(name), "duplicate event {name}");
    }
}

#[test]
fn schema_v1_defines_all_tables() {
    for table in [
        "source_files",
        "messages",
        "limit_snapshots",
        "settings",
        "xp_events",
        "achievements",
        "app_state",
    ] {
        assert!(
            schema::SCHEMA_V1_SQL.contains(&format!("CREATE TABLE {table} (")),
            "missing table {table}"
        );
    }
}
