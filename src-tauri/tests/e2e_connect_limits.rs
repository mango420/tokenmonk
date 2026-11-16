//! TKM-303: "Connect limits" writes the status-line entry into the effective
//! Claude folder's `settings.json` (with a backup), wraps an existing
//! status line with pass-through, and "Disconnect" restores the old entry.
//!
//! Everything runs against a temp folder with a fake `settings.json`.
//! The assumed library API is isolated in `api` below so it can be adapted
//! in one place once TKM-303 has landed.

mod common;

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

/// Assumed API of TKM-303 (names to verify after the coder PR is merged).
mod api {
    use std::path::Path;

    use tokenmonk_lib::limits::connect;

    /// Connect: write our status-line entry into `<claude_dir>/settings.json`.
    pub fn connect(claude_dir: &Path, command: &str) -> Result<(), String> {
        connect::connect_limits(claude_dir, command).map_err(|e| e.to_string())
    }

    /// Disconnect: restore the entry that was there before.
    pub fn disconnect(claude_dir: &Path) -> Result<(), String> {
        connect::disconnect_limits(claude_dir).map_err(|e| e.to_string())
    }

    /// What the connected status line prints for one payload: the bridge
    /// records the limits into `data_dir`, then the old command (if any)
    /// runs and its output is returned unchanged.
    pub fn run_connected(claude_dir: &Path, data_dir: &Path, input: &[u8], now: i64) -> String {
        connect::run_wrapped(claude_dir, data_dir, input, now)
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

/// Any file next to `settings.json` that holds exactly `original`.
fn has_backup_of(dir: &Path, original: &str) -> bool {
    fs::read_dir(dir).unwrap().flatten().any(|e| {
        e.file_name() != "settings.json"
            && fs::read_to_string(e.path()).is_ok_and(|s| s == original)
    })
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
    let original = write_settings(dir.path(), &base_settings());

    api::connect(dir.path(), OUR_COMMAND).unwrap();

    let s = read_settings(dir.path());
    assert_eq!(s["statusLine"]["type"], "command");
    assert_eq!(s["statusLine"]["command"].as_str().unwrap(), OUR_COMMAND);
    for key in ["theme", "permissions", "env"] {
        assert_eq!(s[key], base_settings()[key], "{key} must survive");
    }
    assert!(
        has_backup_of(dir.path(), &original),
        "backup of the original"
    );
}

#[test]
fn disconnect_without_prior_status_line_removes_ours() {
    let dir = tempfile::tempdir().unwrap();
    write_settings(dir.path(), &base_settings());

    api::connect(dir.path(), OUR_COMMAND).unwrap();
    api::disconnect(dir.path()).unwrap();

    assert_eq!(read_settings(dir.path()), base_settings());
}

#[test]
fn connect_creates_settings_json_when_missing_and_disconnect_leaves_valid_json() {
    let dir = tempfile::tempdir().unwrap();

    api::connect(dir.path(), OUR_COMMAND).unwrap();
    assert_eq!(
        read_settings(dir.path())["statusLine"]["command"],
        OUR_COMMAND
    );

    api::disconnect(dir.path()).unwrap();
    // Either the file is gone or it is valid JSON without a status line.
    if settings_path(dir.path()).exists() {
        assert!(read_settings(dir.path()).get("statusLine").is_none());
    }
}

#[test]
fn existing_status_line_is_wrapped_backed_up_and_restored_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let mut with_old = base_settings();
    with_old["statusLine"] = json!({ "type": "command", "command": OLD_COMMAND, "padding": 1 });
    let original = write_settings(dir.path(), &with_old);

    api::connect(dir.path(), OUR_COMMAND).unwrap();

    let s = read_settings(dir.path());
    assert_ne!(s["statusLine"], with_old["statusLine"], "now ours");
    assert!(s["statusLine"]["command"]
        .as_str()
        .unwrap()
        .contains("--statusline"));
    assert!(
        has_backup_of(dir.path(), &original),
        "backup holds the old entry"
    );

    api::disconnect(dir.path()).unwrap();
    assert_eq!(read_settings(dir.path()), with_old, "entry restored 1:1");
}

#[test]
fn wrapped_status_line_passes_the_old_output_through_unchanged() {
    let claude = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut with_old = base_settings();
    with_old["statusLine"] = json!({ "type": "command", "command": OLD_COMMAND });
    write_settings(claude.path(), &with_old);
    api::connect(claude.path(), OUR_COMMAND).unwrap();

    let out = api::run_connected(claude.path(), data.path(), &payload(), 1_793_185_200_000);

    // Same bytes the old command prints on its own (echo adds the newline).
    assert!(
        out == "old-status-line-synthetic\n" || out == "old-status-line-synthetic",
        "got {out:?}"
    );
    // And the limits were still recorded.
    assert!(
        fs::read_dir(data.path()).unwrap().next().is_some(),
        "bridge file written next to the pass-through"
    );
}

#[test]
fn without_an_old_status_line_nothing_else_is_printed_but_limits_are_recorded() {
    let claude = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    write_settings(claude.path(), &base_settings());
    api::connect(claude.path(), OUR_COMMAND).unwrap();

    let out = api::run_connected(claude.path(), data.path(), &payload(), 1_793_185_200_000);

    assert!(!out.contains("old-status-line"));
    assert!(fs::read_dir(data.path()).unwrap().next().is_some());
}

#[test]
fn connecting_twice_does_not_wrap_our_own_entry_and_disconnect_still_restores() {
    let dir = tempfile::tempdir().unwrap();
    let mut with_old = base_settings();
    with_old["statusLine"] = json!({ "type": "command", "command": OLD_COMMAND });
    write_settings(dir.path(), &with_old);

    api::connect(dir.path(), OUR_COMMAND).unwrap();
    api::connect(dir.path(), OUR_COMMAND).unwrap();
    api::disconnect(dir.path()).unwrap();

    assert_eq!(read_settings(dir.path()), with_old);
}

#[test]
fn unreadable_settings_json_is_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let broken = "{ not json, \"statusLine\": ";
    fs::write(settings_path(dir.path()), broken).unwrap();

    assert!(api::connect(dir.path(), OUR_COMMAND).is_err());
    assert_eq!(
        fs::read_to_string(settings_path(dir.path())).unwrap(),
        broken
    );
}

#[test]
fn disconnect_when_never_connected_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut with_old = base_settings();
    with_old["statusLine"] = json!({ "type": "command", "command": OLD_COMMAND });
    let text = write_settings(dir.path(), &with_old);

    let _ = api::disconnect(dir.path());

    assert_eq!(fs::read_to_string(settings_path(dir.path())).unwrap(), text);
}
