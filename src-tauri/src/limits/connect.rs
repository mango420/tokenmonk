//! One-click "Connect limits": writes the `statusLine` entry into
//! `settings.json` of the effective Claude folder (TKM-303).
//!
//! * Only after an explicit call to [`connect`]; nothing here runs on its own.
//! * A backup of the original file is copied next to it before the first
//!   change ([`BACKUP_NAME`]).
//! * An existing `statusLine` is wrapped: the new entry runs
//!   `<exe> --statusline --passthrough`, and the old entry is kept in
//!   `<app data>/statusline-previous.json`. The bridge reads the limits and
//!   then runs the old command with its output unchanged ([`run_passthrough`]).
//! * [`disconnect`] puts the old entry back (or removes the key if there was
//!   none) and leaves every other setting as it is now.
//!
//! Only the `statusLine` key is touched. A settings file that is not a JSON
//! object is never modified.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::setup::BRIDGE_FLAG;

/// Flag after [`BRIDGE_FLAG`] that makes the bridge run the previous command.
pub const PASSTHROUGH_FLAG: &str = "--passthrough";
/// Settings file inside the Claude folder.
pub const SETTINGS_NAME: &str = "settings.json";
/// Copy of the original settings, next to the settings file.
pub const BACKUP_NAME: &str = "settings.json.tokenmonk.bak";
/// The replaced entry, inside the app data directory.
pub const PREVIOUS_NAME: &str = "statusline-previous.json";

const KEY: &str = "statusLine";
const STATE_VERSION: u32 = 1;

/// What [`status`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConnectState {
    /// `statusLine` points at this app.
    pub connected: bool,
    /// An earlier entry is kept and run in pass-through.
    pub wrapped: bool,
    /// Connected, but the command differs from the current one (app moved).
    pub stale: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("no Claude folder")]
    NoFolder,
    #[error("settings.json is not a JSON object; left untouched")]
    InvalidSettings,
    #[error("the existing status line is not a command; left untouched")]
    UnsupportedExisting,
    #[error("no command for this app")]
    NoCommand,
    #[error("{0}")]
    Io(#[from] io::Error),
}

/// Stored previous entry.
#[derive(Debug, Serialize, Deserialize)]
struct Previous {
    version: u32,
    /// The replaced `statusLine` value; `None` if there was none.
    entry: Option<Value>,
}

/// The status line command that wraps (or not) the previous entry.
pub fn command_for(base: &str, wrapped: bool) -> String {
    if wrapped {
        format!("{base} {PASSTHROUGH_FLAG}")
    } else {
        base.to_owned()
    }
}

/// Whether `command` starts this app in bridge mode (with or without
/// pass-through).
fn is_ours(command: &str) -> bool {
    let c = command.trim();
    let c = c.strip_suffix(PASSTHROUGH_FLAG).map_or(c, str::trim_end);
    c.ends_with(BRIDGE_FLAG) && c.to_ascii_lowercase().contains("tokenmonk")
}

fn read_settings(path: &Path) -> Result<Option<Map<String, Value>>, ConnectError> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Object(m)) => Ok(Some(m)),
        _ => Err(ConnectError::InvalidSettings),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = dir.join(format!("{name}.{}.tmp", std::process::id()));
    let result = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn write_settings(path: &Path, map: Map<String, Value>) -> Result<(), ConnectError> {
    let mut text = serde_json::to_string_pretty(&Value::Object(map)).map_err(io::Error::other)?;
    text.push('\n');
    write_atomic(path, text.as_bytes())?;
    Ok(())
}

fn command_of(entry: Option<&Value>) -> Option<&str> {
    entry?.get("command")?.as_str()
}

fn load_previous(data_dir: &Path) -> Option<Previous> {
    let bytes = std::fs::read(data_dir.join(PREVIOUS_NAME)).ok()?;
    serde_json::from_slice::<Previous>(&bytes)
        .ok()
        .filter(|p| p.version == STATE_VERSION)
}

fn state_of(settings: Option<&Map<String, Value>>, current: &str) -> ConnectState {
    let Some(cmd) = settings.and_then(|m| command_of(m.get(KEY))) else {
        return ConnectState::default();
    };
    if !is_ours(cmd) {
        return ConnectState::default();
    }
    let wrapped = cmd.trim_end().ends_with(PASSTHROUGH_FLAG);
    ConnectState {
        connected: true,
        wrapped,
        stale: cmd.trim() != command_for(current, wrapped),
    }
}

/// Current state. Never fails: unreadable or odd files count as not connected.
pub fn status(claude_dir: &Path, _data_dir: &Path, command: &str) -> ConnectState {
    let settings = read_settings(&claude_dir.join(SETTINGS_NAME))
        .ok()
        .flatten();
    state_of(settings.as_ref(), command)
}

/// Connects limits. `command` is `<exe> --statusline`
/// ([`super::setup::current_statusline_command`]). Idempotent; an entry that
/// already points at this app is only refreshed if the path moved.
pub fn connect(
    claude_dir: &Path,
    data_dir: &Path,
    command: &str,
) -> Result<ConnectState, ConnectError> {
    if command.trim().is_empty() {
        return Err(ConnectError::NoCommand);
    }
    let path = claude_dir.join(SETTINGS_NAME);
    let existing = read_settings(&path)?;
    let mut map = existing.clone().unwrap_or_default();
    let old = map.get(KEY).cloned();

    let ours = old
        .as_ref()
        .and_then(|e| command_of(Some(e)))
        .is_some_and(is_ours);

    let (wrapped, new_entry) = if ours {
        // Keep whatever was stored; only the command may change.
        let wrapped = load_previous(data_dir).is_some_and(|p| p.entry.is_some());
        let mut e = old.clone().unwrap_or_else(|| json!({}));
        e["command"] = Value::String(command_for(command, wrapped));
        (wrapped, e)
    } else {
        let wrapped = old.is_some();
        if let Some(e) = &old {
            if command_of(Some(e)).is_none() {
                return Err(ConnectError::UnsupportedExisting);
            }
        }
        // Keep unknown keys of the old entry (padding, refreshInterval, ...).
        let mut e = old.clone().unwrap_or_else(|| json!({}));
        e["type"] = json!("command");
        e["command"] = Value::String(command_for(command, wrapped));
        // Backup first, then the previous entry, then the settings.
        if existing.is_some() {
            std::fs::copy(&path, claude_dir.join(BACKUP_NAME))?;
        }
        let prev = Previous {
            version: STATE_VERSION,
            entry: old.clone(),
        };
        write_atomic(
            &data_dir.join(PREVIOUS_NAME),
            &serde_json::to_vec(&prev).map_err(io::Error::other)?,
        )?;
        (wrapped, e)
    };

    map.insert(KEY.to_owned(), new_entry);
    if let Err(e) = write_settings(&path, map) {
        if !ours {
            let _ = std::fs::remove_file(data_dir.join(PREVIOUS_NAME));
        }
        return Err(e);
    }
    Ok(ConnectState {
        connected: true,
        wrapped,
        stale: false,
    })
}

/// Disconnects limits: restores the previous entry, or removes the key if
/// there was none. If `statusLine` no longer points at this app (the user
/// changed it), settings stay as they are and only the stored entry is dropped.
pub fn disconnect(claude_dir: &Path, data_dir: &Path) -> Result<ConnectState, ConnectError> {
    let path = claude_dir.join(SETTINGS_NAME);
    if let Some(mut map) = read_settings(&path)? {
        let ours = command_of(map.get(KEY)).is_some_and(is_ours);
        if ours {
            match load_previous(data_dir).and_then(|p| p.entry) {
                Some(entry) => {
                    map.insert(KEY.to_owned(), entry);
                }
                None => {
                    map.remove(KEY);
                }
            }
            write_settings(&path, map)?;
        }
    }
    match std::fs::remove_file(data_dir.join(PREVIOUS_NAME)) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(ConnectState::default())
}

/// Path of the backup for `claude_dir`, for display.
pub fn backup_path(claude_dir: &Path) -> PathBuf {
    claude_dir.join(BACKUP_NAME)
}

/// The previous command stored for pass-through, if any.
pub fn previous_command(data_dir: &Path) -> Option<String> {
    let p = load_previous(data_dir)?;
    command_of(p.entry.as_ref()).map(str::to_owned)
}

/// Longest the old command may run; Claude Code cancels slow scripts anyway,
/// and this keeps an orphan from living on after that.
const PASSTHROUGH_TIMEOUT: Duration = Duration::from_secs(10);

/// Git Bash on Windows, if installed.
#[cfg(windows)]
fn git_bash() -> Option<PathBuf> {
    ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
        .iter()
        .filter_map(std::env::var_os)
        .flat_map(|base| {
            let base = PathBuf::from(base);
            [
                base.join("Git").join("bin").join("bash.exe"),
                base.join("Programs")
                    .join("Git")
                    .join("bin")
                    .join("bash.exe"),
            ]
        })
        .find(|p| p.is_file())
}

/// The shell Claude Code uses: Git Bash if installed, else PowerShell on
/// Windows; `sh -c` elsewhere.
fn shell_command(line: &str) -> Command {
    #[cfg(windows)]
    {
        if let Some(bash) = git_bash() {
            let mut c = Command::new(bash);
            c.arg("-c").arg(line);
            return c;
        }
        let mut c = Command::new("powershell");
        c.args(["-NoProfile", "-NonInteractive", "-Command"])
            .arg(line);
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = Command::new("sh");
        c.arg("-c").arg(line);
        c
    }
}

/// Runs `line` in the shell with `input` on stdin and returns its stdout and
/// exit code. The child is killed after [`PASSTHROUGH_TIMEOUT`].
fn run_shell(line: &str, input: &[u8]) -> Option<(Vec<u8>, i32)> {
    let mut child = shell_command(line)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    let mut stdout = child.stdout.take()?;
    let data = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
    });
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let start = Instant::now();
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(0),
            Ok(None) if start.elapsed() < PASSTHROUGH_TIMEOUT => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break 0;
            }
        }
    };
    let _ = writer.join();
    Some((reader.join().unwrap_or_default(), code))
}

/// Pass-through half of the bridge: the previous command's stdout bytes and
/// exit code. `None` if there is no stored command or it could not start.
pub fn run_passthrough(data_dir: &Path, input: &[u8]) -> Option<(Vec<u8>, i32)> {
    let line = previous_command(data_dir)?;
    run_shell(&line, input)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CMD: &str = "/opt/tokenmonk/tokenmonk --statusline";

    struct Env {
        claude: tempfile::TempDir,
        data: tempfile::TempDir,
    }

    fn env(settings: Option<&str>) -> Env {
        let e = Env {
            claude: tempfile::tempdir().unwrap(),
            data: tempfile::tempdir().unwrap(),
        };
        if let Some(s) = settings {
            std::fs::write(e.claude.path().join(SETTINGS_NAME), s).unwrap();
        }
        e
    }

    fn settings(e: &Env) -> Value {
        let b = std::fs::read(e.claude.path().join(SETTINGS_NAME)).unwrap();
        serde_json::from_slice(&b).unwrap()
    }

    #[test]
    fn connects_without_settings_file() {
        let e = env(None);
        let st = connect(e.claude.path(), e.data.path(), CMD).unwrap();
        assert_eq!(
            st,
            ConnectState {
                connected: true,
                wrapped: false,
                stale: false
            }
        );
        assert_eq!(settings(&e)["statusLine"]["command"], CMD);
        assert!(!e.claude.path().join(BACKUP_NAME).exists());
        assert_eq!(status(e.claude.path(), e.data.path(), CMD), st);
    }

    #[test]
    fn disconnect_removes_the_key_but_keeps_other_settings() {
        let e = env(Some(r#"{"theme":"dark"}"#));
        connect(e.claude.path(), e.data.path(), CMD).unwrap();
        assert_eq!(
            std::fs::read_to_string(e.claude.path().join(BACKUP_NAME)).unwrap(),
            r#"{"theme":"dark"}"#
        );
        // A later edit by the user survives.
        let mut s = settings(&e);
        s["model"] = json!("x");
        std::fs::write(e.claude.path().join(SETTINGS_NAME), s.to_string()).unwrap();
        let st = disconnect(e.claude.path(), e.data.path()).unwrap();
        assert!(!st.connected);
        assert_eq!(settings(&e), json!({"theme":"dark","model":"x"}));
        assert!(!e.data.path().join(PREVIOUS_NAME).exists());
    }

    #[test]
    fn wraps_an_existing_entry_and_restores_it() {
        let orig = json!({"type":"command","command":"echo hi","padding":2,"refreshInterval":5});
        let e = env(Some(&json!({"statusLine": orig, "a": 1}).to_string()));
        let st = connect(e.claude.path(), e.data.path(), CMD).unwrap();
        assert!(st.connected && st.wrapped);
        let s = settings(&e);
        assert_eq!(
            s["statusLine"]["command"],
            format!("{CMD} {PASSTHROUGH_FLAG}")
        );
        assert_eq!(s["statusLine"]["padding"], 2);
        assert_eq!(s["statusLine"]["refreshInterval"], 5);
        assert_eq!(previous_command(e.data.path()).unwrap(), "echo hi");
        assert!(e.claude.path().join(BACKUP_NAME).exists());
        disconnect(e.claude.path(), e.data.path()).unwrap();
        assert_eq!(settings(&e), json!({"statusLine": orig, "a": 1}));
    }

    #[test]
    fn connect_twice_keeps_the_first_previous_entry() {
        let e = env(Some(
            r#"{"statusLine":{"type":"command","command":"echo hi"}}"#,
        ));
        connect(e.claude.path(), e.data.path(), CMD).unwrap();
        let st = connect(e.claude.path(), e.data.path(), CMD).unwrap();
        assert!(st.wrapped);
        assert_eq!(previous_command(e.data.path()).unwrap(), "echo hi");
    }

    #[test]
    fn moved_app_is_stale_and_refreshed_on_connect() {
        let e = env(None);
        connect(e.claude.path(), e.data.path(), CMD).unwrap();
        let moved = "/new/tokenmonk --statusline";
        let st = status(e.claude.path(), e.data.path(), moved);
        assert!(st.connected && st.stale);
        let st = connect(e.claude.path(), e.data.path(), moved).unwrap();
        assert!(!st.stale);
        assert_eq!(settings(&e)["statusLine"]["command"], moved);
    }

    #[test]
    fn foreign_entry_is_not_connected_and_survives_disconnect() {
        let e = env(Some(
            r#"{"statusLine":{"type":"command","command":"echo hi"}}"#,
        ));
        assert_eq!(
            status(e.claude.path(), e.data.path(), CMD),
            ConnectState::default()
        );
        disconnect(e.claude.path(), e.data.path()).unwrap();
        assert_eq!(settings(&e)["statusLine"]["command"], "echo hi");
    }

    #[test]
    fn invalid_settings_are_never_modified() {
        for raw in ["not json", "[1,2]", "null"] {
            let e = env(Some(raw));
            assert!(matches!(
                connect(e.claude.path(), e.data.path(), CMD),
                Err(ConnectError::InvalidSettings)
            ));
            assert_eq!(
                std::fs::read_to_string(e.claude.path().join(SETTINGS_NAME)).unwrap(),
                raw
            );
            assert!(!e.claude.path().join(BACKUP_NAME).exists());
        }
    }

    #[test]
    fn non_command_entry_is_left_untouched() {
        let raw = r#"{"statusLine":{"type":"other"}}"#;
        let e = env(Some(raw));
        assert!(matches!(
            connect(e.claude.path(), e.data.path(), CMD),
            Err(ConnectError::UnsupportedExisting)
        ));
        assert_eq!(
            std::fs::read_to_string(e.claude.path().join(SETTINGS_NAME)).unwrap(),
            raw
        );
    }

    #[cfg(unix)]
    #[test]
    fn passthrough_returns_the_old_output_unchanged() {
        let e = env(Some(
            r#"{"statusLine":{"type":"command","command":"cat; printf ' done'; exit 3"}}"#,
        ));
        connect(e.claude.path(), e.data.path(), CMD).unwrap();
        let (out, code) = run_passthrough(e.data.path(), b"payload").unwrap();
        assert_eq!(out, b"payload done");
        assert_eq!(code, 3);
    }

    #[test]
    fn no_passthrough_without_a_previous_entry() {
        let e = env(None);
        connect(e.claude.path(), e.data.path(), CMD).unwrap();
        assert!(run_passthrough(e.data.path(), b"{}").is_none());
    }
}
