//! TKM-303: "Connect limits" writes the status-line entry into the effective
//! Claude folder's `settings.json` (with a backup), wraps an existing
//! status line with pass-through, and "Disconnect" restores the old entry.
//!
//! Everything runs against temp folders: a fake Claude folder with a fake
//! `settings.json` and a fake app data folder.

mod common;

use std::fs;
use std::path::Path;

use serde_json::{json, Value};
use tokenmonk_lib::limits::bridge::FILE_NAME;
use tokenmonk_lib::limits::connect;

/// Thin wrappers over `limits::connect` (TKM-303). The claude folder and the
/// app data folder are both temp dirs, so nothing outside them is touched.
mod api {
    use std::path::Path;

    use tokenmonk_lib::limits::bridge;
    use tokenmonk_lib::limits::connect;

    pub fn connect(claude_dir: &Path, data_dir: &Path, command: &str) -> Result<(), String> {
        connect::connect(Some(claude_dir), data_dir, command)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn disconnect(claude_dir: &Path, data_dir: &Path) -> Result<(), String> {
        connect::disconnect(Some(claude_dir), data_dir)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// What the connected status line does for one payload (`--statusline
    /// --passthrough`): the bridge records the limits into `data_dir`, then
    /// the old command runs; returns its stdout and exit code.
    pub fn run_connected(data_dir: &Path, input: &[u8], now: i64) -> Option<(Vec<u8>, i32)> {
        bridge::process(input, now, Some(data_dir));
        connect::run_passthrough(data_dir, input)
    }
}

const OUR_COMMAND: &str = "/opt/tokenmonk/tokenmonk --statusline";
const OLD_COMMAND: &str = "echo old-status-line-synthetic";

fn settings_path(dir: &Path) -> std::path::PathBuf {
    dir.join("settings.json")
}

fn read_settings(dir: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(settings_path(dir)).unwrap()).unwrap()
}

fn write_settings(dir: &Path, v: &Value) -> String {
    let text = serde_json::to_string_pretty(v).unwrap();
    fs::write(settings_path(dir), &text).unwrap();
    text
}

/// The backup written before the first change holds the original text.
fn assert_backup_of(dir: &Path, original: &str) {
    assert_eq!(
        fs::read_to_string(dir.join(connect::BACKUP_NAME)).expect("backup file"),
        original
    );
}

/// The entry stored for pass-through, if any.
fn previous_entry(data: &Path) -> Option<Value> {
    fs::read_to_string(data.join(connect::PREVIOUS_NAME))
        .ok()
        .map(|t| serde_json::from_str(&t).unwrap())
}

fn keys(v: &Value) -> Vec<String> {
    v.as_object().unwrap().keys().cloned().collect()
}

/// Equal values and the same key order at the top level and in `statusLine`.
fn assert_same_with_order(got: &Value, want: &Value) {
    assert_eq!(got, want);
    assert_eq!(keys(got), keys(want), "top-level key order");
    if want.get("statusLine").is_some() {
        assert_eq!(
            keys(&got["statusLine"]),
            keys(&want["statusLine"]),
            "statusLine key order"
        );
    }
}

fn payload() -> Vec<u8> {
    br#"{"session_id":"fake-session-id","cwd":"/Users/example/projects/synthetic","rate_limits":{"five_hour":{"used_percentage":42.0,"resets_at":1793188800},"seven_day":{"used_percentage":10.0,"resets_at":1793383200}}}"#.to_vec()
}

fn base_settings() -> Value {
    json!({
        "theme": "dark",
        "permissions": { "allow": ["Bash(ls:*)"] },
        "env": { "FAKE_FLAG": "1" }
    })
}

#[test]
fn connect_without_existing_status_line_adds_ours_and_keeps_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let original = write_settings(dir.path(), &base_settings());

    api::connect(dir.path(), data.path(), OUR_COMMAND).unwrap();

    let s = read_settings(dir.path());
    assert_eq!(s["statusLine"]["type"], "command");
    assert_eq!(s["statusLine"]["command"].as_str().unwrap(), OUR_COMMAND);
    for key in ["theme", "permissions", "env"] {
        assert_eq!(s[key], base_settings()[key], "{key} must survive");
    }
    assert_backup_of(dir.path(), &original);
    assert!(
        previous_entry(data.path()).is_none(),
        "nothing to pass through"
    );
}

#[test]
fn disconnect_without_prior_status_line_removes_ours() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    write_settings(dir.path(), &base_settings());

    api::connect(dir.path(), data.path(), OUR_COMMAND).unwrap();
    api::disconnect(dir.path(), data.path()).unwrap();

    assert_same_with_order(&read_settings(dir.path()), &base_settings());
    assert!(previous_entry(data.path()).is_none());
}

#[test]
fn connect_creates_settings_json_when_missing_and_disconnect_leaves_valid_json() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();

    api::connect(dir.path(), data.path(), OUR_COMMAND).unwrap();
    assert_eq!(
        read_settings(dir.path())["statusLine"]["command"],
        OUR_COMMAND
    );

    api::disconnect(dir.path(), data.path()).unwrap();
    // Either the file is gone or it is valid JSON without a status line.
    if settings_path(dir.path()).exists() {
        assert!(read_settings(dir.path()).get("statusLine").is_none());
    }
}

#[test]
fn existing_status_line_is_wrapped_backed_up_and_restored_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut with_old = base_settings();
    with_old["statusLine"] = json!({ "type": "command", "command": OLD_COMMAND, "padding": 1 });
    let original = write_settings(dir.path(), &with_old);

    api::connect(dir.path(), data.path(), OUR_COMMAND).unwrap();

    let s = read_settings(dir.path());
    assert_eq!(s["statusLine"]["type"], "command");
    assert_eq!(
        s["statusLine"]["padding"], 1,
        "unknown keys of the entry stay"
    );
    assert!(s["statusLine"]["command"]
        .as_str()
        .unwrap()
        .contains("--statusline"));
    assert_ne!(s["statusLine"]["command"], OLD_COMMAND, "now ours");
    // Other top-level keys keep their order, statusLine stays last.
    assert_eq!(keys(&s), keys(&with_old), "top-level key order");
    assert_backup_of(dir.path(), &original);
    assert_eq!(
        previous_entry(data.path()).expect("old entry stored"),
        with_old["statusLine"],
        "old entry kept unchanged for pass-through"
    );

    api::disconnect(dir.path(), data.path()).unwrap();
    assert_same_with_order(&read_settings(dir.path()), &with_old);
    assert!(
        previous_entry(data.path()).is_none(),
        "record gone after disconnect"
    );
}

#[test]
fn wrapped_status_line_passes_the_old_output_through_unchanged() {
    let claude = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut with_old = base_settings();
    with_old["statusLine"] = json!({ "type": "command", "command": OLD_COMMAND });
    write_settings(claude.path(), &with_old);
    api::connect(claude.path(), data.path(), OUR_COMMAND).unwrap();

    let (out, code) =
        api::run_connected(data.path(), &payload(), 1_793_185_200_000).expect("old command ran");

    // Same bytes the old command prints on its own.
    assert_eq!(
        out,
        b"old-status-line-synthetic
",
        "got {out:?}"
    );
    assert_eq!(code, 0);
    // And the limits were still recorded.
    assert!(
        data.path().join(FILE_NAME).is_file(),
        "bridge file written next to the pass-through"
    );
}

#[test]
fn without_an_old_status_line_there_is_nothing_to_pass_through_but_limits_are_recorded() {
    let claude = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    write_settings(claude.path(), &base_settings());
    api::connect(claude.path(), data.path(), OUR_COMMAND).unwrap();

    assert!(api::run_connected(data.path(), &payload(), 1_793_185_200_000).is_none());
    assert!(data.path().join(FILE_NAME).is_file());
}

#[test]
fn connecting_twice_does_not_wrap_our_own_entry_and_disconnect_still_restores() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut with_old = base_settings();
    with_old["statusLine"] = json!({ "type": "command", "command": OLD_COMMAND });
    write_settings(dir.path(), &with_old);

    api::connect(dir.path(), data.path(), OUR_COMMAND).unwrap();
    api::connect(dir.path(), data.path(), OUR_COMMAND).unwrap();
    api::disconnect(dir.path(), data.path()).unwrap();

    assert_same_with_order(&read_settings(dir.path()), &with_old);
}

#[test]
fn unreadable_settings_json_is_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let broken = "{ not json, \"statusLine\": ";
    fs::write(settings_path(dir.path()), broken).unwrap();

    assert!(api::connect(dir.path(), data.path(), OUR_COMMAND).is_err());
    assert_eq!(
        fs::read_to_string(settings_path(dir.path())).unwrap(),
        broken
    );
}

#[test]
fn disconnect_when_never_connected_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut with_old = base_settings();
    with_old["statusLine"] = json!({ "type": "command", "command": OLD_COMMAND });
    let text = write_settings(dir.path(), &with_old);

    let _ = api::disconnect(dir.path(), data.path());

    assert_eq!(fs::read_to_string(settings_path(dir.path())).unwrap(), text);
}
