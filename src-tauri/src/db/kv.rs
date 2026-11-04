//! Key/value tables (`settings`, `app_state`): JSON text values.

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use super::{Db, DbError, DbResult};
use crate::contracts::settings::{Mode, Settings, SettingsPatch};

const KEY_MODE: &str = "mode";
const KEY_SOUND: &str = "sound";
const KEY_AUTOSTART: &str = "autostart";
const KEY_UPDATE_CHECK: &str = "update_check";

fn get(conn: &Connection, table: &str, key: &str) -> DbResult<Option<Value>> {
    let sql = format!("SELECT value FROM {table} WHERE key = ?1");
    let raw: Option<String> = conn.query_row(&sql, [key], |r| r.get(0)).optional()?;
    // A value that is not valid JSON is treated as absent, not as an error.
    Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
}

fn set(conn: &Connection, table: &str, key: &str, value: &Value) -> DbResult<()> {
    let sql = format!(
        "INSERT INTO {table} (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value"
    );
    conn.execute(&sql, params![key, value.to_string()])?;
    Ok(())
}

impl Db {
    /// Raw JSON value of a setting, if present.
    pub fn setting(&self, key: &str) -> DbResult<Option<Value>> {
        get(self.conn(), "settings", key)
    }

    /// Writes a raw JSON setting value.
    pub fn set_setting(&self, key: &str, value: &Value) -> DbResult<()> {
        set(self.conn(), "settings", key, value)
    }

    /// All settings; missing or invalid values fall back to the defaults.
    pub fn settings(&self) -> DbResult<Settings> {
        let mut s = Settings::default();
        if let Some(mode) = self
            .setting(KEY_MODE)?
            .and_then(|v| serde_json::from_value::<Mode>(v).ok())
        {
            s.mode = mode;
        }
        if let Some(b) = self.setting(KEY_SOUND)?.and_then(|v| v.as_bool()) {
            s.sound = b;
        }
        if let Some(b) = self.setting(KEY_AUTOSTART)?.and_then(|v| v.as_bool()) {
            s.autostart = b;
        }
        if let Some(b) = self.setting(KEY_UPDATE_CHECK)?.and_then(|v| v.as_bool()) {
            s.update_check = b;
        }
        Ok(s)
    }

    /// Applies the present fields of `patch` in one transaction and returns
    /// the resulting settings. Validation (for example rejecting
    /// `mode = noneYet`) is the caller's job.
    pub fn apply_settings_patch(&self, patch: &SettingsPatch) -> DbResult<Settings> {
        let tx = self.conn().unchecked_transaction()?;
        if let Some(mode) = patch.mode {
            let v = serde_json::to_value(mode).map_err(|e| DbError::Invalid(e.to_string()))?;
            set(&tx, "settings", KEY_MODE, &v)?;
        }
        if let Some(b) = patch.sound {
            set(&tx, "settings", KEY_SOUND, &Value::Bool(b))?;
        }
        if let Some(b) = patch.autostart {
            set(&tx, "settings", KEY_AUTOSTART, &Value::Bool(b))?;
        }
        if let Some(b) = patch.update_check {
            set(&tx, "settings", KEY_UPDATE_CHECK, &Value::Bool(b))?;
        }
        tx.commit()?;
        self.settings()
    }

    /// Raw JSON value of an internal flag (`retro_done`, `first_start_at`, ...).
    pub fn app_state(&self, key: &str) -> DbResult<Option<Value>> {
        get(self.conn(), "app_state", key)
    }

    pub fn set_app_state(&self, key: &str, value: &Value) -> DbResult<()> {
        set(self.conn(), "app_state", key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn settings_default_then_patch() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.settings().unwrap(), Settings::default());
        let s = db
            .apply_settings_patch(&SettingsPatch {
                mode: Some(Mode::Diy),
                sound: Some(false),
                autostart: None,
                update_check: None,
                keep_open: None,
            })
            .unwrap();
        assert_eq!(s.mode, Mode::Diy);
        assert!(!s.sound);
        assert!(!s.autostart);
        let s = db
            .apply_settings_patch(&SettingsPatch {
                autostart: Some(true),
                ..SettingsPatch::default()
            })
            .unwrap();
        assert_eq!((s.mode, s.sound, s.autostart), (Mode::Diy, false, true));
    }

    #[test]
    fn update_check_defaults_true_and_round_trips() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.settings().unwrap().update_check);
        let s = db
            .apply_settings_patch(&SettingsPatch {
                update_check: Some(false),
                ..SettingsPatch::default()
            })
            .unwrap();
        assert!(!s.update_check);
        assert!(!db.settings().unwrap().update_check);
        let s = db
            .apply_settings_patch(&SettingsPatch {
                update_check: Some(true),
                ..SettingsPatch::default()
            })
            .unwrap();
        assert!(s.update_check);
    }

    #[test]
    fn invalid_stored_values_fall_back_to_defaults() {
        let db = Db::open_in_memory().unwrap();
        db.set_setting("mode", &json!("bogus")).unwrap();
        db.set_setting("sound", &json!("yes")).unwrap();
        db.conn()
            .execute(
                "INSERT INTO settings (key, value) VALUES ('autostart', '{x')",
                [],
            )
            .unwrap();
        assert_eq!(db.settings().unwrap(), Settings::default());
    }

    #[test]
    fn app_state_roundtrip_and_overwrite() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.app_state("retro_done").unwrap(), None);
        db.set_app_state("retro_done", &json!(true)).unwrap();
        db.set_app_state("first_start_at", &json!(5)).unwrap();
        db.set_app_state("first_start_at", &json!(7)).unwrap();
        assert_eq!(db.app_state("retro_done").unwrap(), Some(json!(true)));
        assert_eq!(db.app_state("first_start_at").unwrap(), Some(json!(7)));
    }
}
