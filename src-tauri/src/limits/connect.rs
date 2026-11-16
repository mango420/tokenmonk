//! One-click "Connect limits": writes the `statusLine` entry into
//! `settings.json` of the effective Claude folder (TKM-303, contract TKM-300,
//! docs/ARCHITECTURE.md "Connect limits").
//!
//! * Only after an explicit call to [`connect`] or [`disconnect`].
//! * Every write first copies the current file to [`BACKUP_NAME`].
//! * An existing `statusLine` is stored verbatim in
//!   `<app data>/statusline-previous.json`; the new entry runs
//!   `<exe> --statusline --passthrough`, and the bridge runs the old command
//!   after writing the limits ([`run_passthrough`]).
//! * [`disconnect`] puts the old entry back (or removes the key).
//!
//! Only the `statusLine` key is touched; other keys keep their order. A file
//! that cannot be parsed is never rewritten.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use super::setup::BRIDGE_FLAG;
use crate::contracts::{LimitsConnection, LimitsConnectionProblem, LimitsConnectionState};

/// Flag after [`BRIDGE_FLAG`] that makes the bridge run the previous command.
pub const PASSTHROUGH_FLAG: &str = "--passthrough";
/// Settings file inside the Claude folder.
pub const SETTINGS_NAME: &str = "settings.json";
/// Copy of the file before each write, next to the settings file.
pub const BACKUP_NAME: &str = "settings.json.tokenmonk.bak";
/// The replaced entry, inside the app data directory.
pub const PREVIOUS_NAME: &str = "statusline-previous.json";

const KEY: &str = "statusLine";

/// Connect or disconnect refused or failed; the file is unchanged.
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("blocked: {0:?}")]
    Blocked(LimitsConnectionProblem),
    #[error("no command for this app")]
    NoCommand,
    #[error("{0}")]
    Io(#[from] io::Error),
}

/// The status line command that wraps (or not) the previous entry.
pub fn command_for(base: &str, wrapped: bool) -> String {
    if wrapped {
        format!("{base} {PASSTHROUGH_FLAG}")
    } else {
        base.to_owned()
    }
}

/// First token of a shell command line (double quotes allowed) and the rest.
fn split_first(command: &str) -> (&str, &str) {
    let c = command.trim_start();
    if let Some(rest) = c.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return (&rest[..end], &rest[end + 1..]);
        }
        return (rest, "");
    }
    match c.find(char::is_whitespace) {
        Some(i) => (&c[..i], &c[i..]),
        None => (c, ""),
    }
}

/// A `command` that runs a Tokenmonk executable with `--statusline` as its
/// first argument.
fn is_ours(command: &str) -> bool {
    // PowerShell form: `& "C:/.../tokenmonk.exe" --statusline`.
    let command = command.trim_start();
    let command = command.strip_prefix('&').unwrap_or(command);
    let (exe, rest) = split_first(command);
    let name = exe
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    name.starts_with("tokenmonk") && rest.split_whitespace().next() == Some(BRIDGE_FLAG)
}

fn command_of(entry: Option<&Value>) -> Option<&str> {
    entry?.get("command")?.as_str()
}

fn entry_is_ours(entry: &Value) -> bool {
    entry.get("type").and_then(Value::as_str) == Some("command")
        && command_of(Some(entry)).is_some_and(is_ours)
}

/// Parsed `settings.json`, or why it cannot be used.
enum Loaded {
    Ok(Map<String, Value>),
    Missing,
    Blocked(LimitsConnectionProblem, String),
}

fn load(claude_dir: Option<&Path>) -> (Option<PathBuf>, Loaded) {
    let Some(dir) = claude_dir else {
        return (
            None,
            Loaded::Blocked(LimitsConnectionProblem::NoClaudeFolder, "unknown".into()),
        );
    };
    let path = dir.join(SETTINGS_NAME);
    if !dir.is_dir() {
        return (
            Some(path),
            Loaded::Blocked(LimitsConnectionProblem::NoClaudeFolder, "no folder".into()),
        );
    }
    let loaded = match std::fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice::<Value>(&bytes) {
            Ok(Value::Object(m)) => match m.get(KEY) {
                Some(v) if !v.is_object() => Loaded::Blocked(
                    LimitsConnectionProblem::SettingsInvalid,
                    "statusLine is not an object".into(),
                ),
                _ => Loaded::Ok(m),
            },
            _ => Loaded::Blocked(
                LimitsConnectionProblem::SettingsInvalid,
                "not a JSON object".into(),
            ),
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Loaded::Missing,
        Err(e) => Loaded::Blocked(LimitsConnectionProblem::SettingsUnreadable, e.to_string()),
    };
    (Some(path), loaded)
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

/// Backup of the current file (if any), then the atomic write.
fn write_settings(path: &Path, map: Map<String, Value>) -> io::Result<()> {
    if path.is_file() {
        if let Some(dir) = path.parent() {
            std::fs::copy(path, dir.join(BACKUP_NAME))?;
        }
    }
    let mut text = serde_json::to_string_pretty(&Value::Object(map)).map_err(io::Error::other)?;
    text.push('\n');
    write_atomic(path, text.as_bytes())
}

/// The stored previous `statusLine` value, verbatim.
fn load_previous(data_dir: &Path) -> Option<Value> {
    let bytes = std::fs::read(data_dir.join(PREVIOUS_NAME)).ok()?;
    serde_json::from_slice::<Value>(&bytes)
        .ok()
        .filter(Value::is_object)
}

fn remove_previous(data_dir: &Path) -> io::Result<()> {
    match std::fs::remove_file(data_dir.join(PREVIOUS_NAME)) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

fn describe(path: Option<PathBuf>, loaded: &Loaded, data_dir: &Path) -> LimitsConnection {
    let settings_path = path.map(|p| p.to_string_lossy().into_owned());
    let (state, previous, problem, detail) = match loaded {
        Loaded::Blocked(p, d) => (
            LimitsConnectionState::Blocked,
            false,
            Some(*p),
            Some(d.clone()),
        ),
        Loaded::Missing => (LimitsConnectionState::NotConnected, false, None, None),
        Loaded::Ok(m) => match m.get(KEY) {
            Some(e) if entry_is_ours(e) => (
                LimitsConnectionState::Connected,
                load_previous(data_dir).is_some(),
                None,
                None,
            ),
            other => (
                LimitsConnectionState::NotConnected,
                other.is_some(),
                None,
                None,
            ),
        },
    };
    LimitsConnection {
        state,
        settings_path,
        previous_status_line: previous,
        problem,
        detail,
    }
}

/// Current state, read fresh. Never fails.
pub fn inspect(claude_dir: Option<&Path>, data_dir: &Path) -> LimitsConnection {
    let (path, loaded) = load(claude_dir);
    describe(path, &loaded, data_dir)
}

/// Connects limits. `command` is `<exe> --statusline`
/// ([`super::setup::current_statusline_command`]). Idempotent.
pub fn connect(
    claude_dir: Option<&Path>,
    data_dir: &Path,
    command: &str,
) -> Result<LimitsConnection, ConnectError> {
    if command.trim().is_empty() {
        return Err(ConnectError::NoCommand);
    }
    let (path, loaded) = load(claude_dir);
    let mut map = match loaded {
        Loaded::Blocked(p, _) => return Err(ConnectError::Blocked(p)),
        Loaded::Missing => Map::new(),
        Loaded::Ok(m) => m,
    };
    let path = path.ok_or(ConnectError::Blocked(
        LimitsConnectionProblem::NoClaudeFolder,
    ))?;
    let old = map.get(KEY).cloned();
    let ours = old.as_ref().is_some_and(entry_is_ours);
    let mut stored_now = false;
    if !ours {
        if old.is_none() {
            // A record from an earlier connect must not come back later.
            remove_previous(data_dir)?;
        }
        if let Some(prev) = &old {
            write_atomic(
                &data_dir.join(PREVIOUS_NAME),
                &serde_json::to_vec(prev).map_err(io::Error::other)?,
            )?;
            stored_now = true;
        }
    }
    let wrapped = if ours {
        load_previous(data_dir).is_some()
    } else {
        old.is_some()
    };
    // Unknown keys of the old entry stay as they are.
    let mut entry = old.unwrap_or_else(|| json!({}));
    entry["type"] = json!("command");
    entry["command"] = Value::String(command_for(command, wrapped));
    map.insert(KEY.to_owned(), entry);
    if let Err(e) = write_settings(&path, map) {
        if stored_now {
            let _ = remove_previous(data_dir);
        }
        return Err(e.into());
    }
    Ok(inspect(claude_dir, data_dir))
}

/// Disconnects limits: restores the previous entry or removes the key. When
/// not connected the file is not touched; a leftover record is deleted.
pub fn disconnect(
    claude_dir: Option<&Path>,
    data_dir: &Path,
) -> Result<LimitsConnection, ConnectError> {
    let (path, loaded) = load(claude_dir);
    match loaded {
        Loaded::Blocked(p, _) => return Err(ConnectError::Blocked(p)),
        Loaded::Ok(mut map) if map.get(KEY).is_some_and(entry_is_ours) => {
            let path = path.ok_or(ConnectError::Blocked(
                LimitsConnectionProblem::NoClaudeFolder,
            ))?;
            match load_previous(data_dir) {
                Some(prev) => {
                    map.insert(KEY.to_owned(), prev);
                }
                None => {
                    map.shift_remove(KEY);
                }
            }
            write_settings(&path, map)?;
        }
        _ => {}
    }
    remove_previous(data_dir)?;
    Ok(inspect(claude_dir, data_dir))
}

/// The previous command stored for pass-through, if any.
pub fn previous_command(data_dir: &Path) -> Option<String> {
    command_of(load_previous(data_dir).as_ref()).map(str::to_owned)
}

/// Longest the old command may run; Claude Code cancels slow scripts anyway,
/// and this keeps an orphan from living on after that.
const PASSTHROUGH_TIMEOUT: Duration = Duration::from_secs(10);
/// How long to wait for the last output after the child ended.
const OUTPUT_GRACE: Duration = Duration::from_millis(300);

/// Git Bash on Windows, as Claude Code finds it: `CLAUDE_CODE_GIT_BASH_PATH`
/// (docs: env-vars), then `git.exe` on PATH (`<git>/../../bin/bash.exe`), then
/// the usual install folders.
#[cfg(windows)]
fn git_bash() -> Option<PathBuf> {
    let is_bash = |p: &PathBuf| {
        p.is_file()
            && matches!(
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(str::to_ascii_lowercase)
                    .as_deref(),
                Some("bash.exe" | "sh.exe" | "bash" | "sh")
            )
    };
    if let Some(p) = std::env::var_os("CLAUDE_CODE_GIT_BASH_PATH").map(PathBuf::from) {
        if is_bash(&p) {
            return Some(p);
        }
    }
    let on_path = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join("git.exe"))
            .find(|g| g.is_file())
            .and_then(|g| {
                let root = g.parent()?.parent()?;
                Some(root.join("bin").join("bash.exe"))
            })
            .filter(|p| p.is_file())
    });
    if on_path.is_some() {
        return on_path;
    }
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
/// Windows; `sh -c` elsewhere. No console window flashes on Windows.
fn shell_command(line: &str) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut c = if let Some(bash) = git_bash() {
            let mut c = Command::new(bash);
            c.arg("-c").arg(line);
            c
        } else {
            let mut c = Command::new("powershell");
            c.args(["-NoProfile", "-NonInteractive", "-Command"])
                .arg(line);
            c
        };
        c.creation_flags(CREATE_NO_WINDOW);
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
/// exit code. The child is killed after `timeout`. Threads are never joined:
/// a grandchild may keep the pipes open, so output is collected from a channel
/// for a short grace period after the child ends.
fn run_shell(line: &str, input: &[u8], timeout: Duration) -> Option<(Vec<u8>, i32)> {
    let mut child = shell_command(line)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    let mut stdout = child.stdout.take()?;
    let data = input.to_vec();
    std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
    });
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        while let Ok(n) = stdout.read(&mut chunk) {
            if n == 0 || tx.send(chunk[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    let start = Instant::now();
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(0),
            Ok(None) if start.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break 0;
            }
        }
    };
    let mut out = Vec::new();
    let deadline = Instant::now() + OUTPUT_GRACE;
    while let Ok(chunk) = rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        out.extend_from_slice(&chunk);
    }
    Some((out, code))
}

/// Pass-through half of the bridge: the previous command's stdout bytes and
/// exit code. `None` if there is no stored command or it could not start.
pub fn run_passthrough(data_dir: &Path, input: &[u8]) -> Option<(Vec<u8>, i32)> {
    let line = previous_command(data_dir)?;
    run_shell(&line, input, PASSTHROUGH_TIMEOUT)
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

    impl Env {
        fn dir(&self) -> Option<&Path> {
            Some(self.claude.path())
        }
        fn connect(&self) -> Result<LimitsConnection, ConnectError> {
            connect(self.dir(), self.data.path(), CMD)
        }
        fn disconnect(&self) -> Result<LimitsConnection, ConnectError> {
            disconnect(self.dir(), self.data.path())
        }
        fn inspect(&self) -> LimitsConnection {
            inspect(self.dir(), self.data.path())
        }
        fn text(&self) -> String {
            std::fs::read_to_string(self.claude.path().join(SETTINGS_NAME)).unwrap()
        }
        fn settings(&self) -> Value {
            serde_json::from_str(&self.text()).unwrap()
        }
    }

    fn keys(v: &Value) -> Vec<String> {
        v.as_object().unwrap().keys().cloned().collect()
    }

    #[test]
    fn ownership_checks_the_first_argument() {
        assert!(is_ours("/a/tokenmonk --statusline"));
        assert!(is_ours(
            "\"C:/Program Files/Tokenmonk/tokenmonk.exe\" --statusline --passthrough"
        ));
        assert!(is_ours(
            "& \"C:/Program Files/Tokenmonk/tokenmonk.exe\" --statusline"
        ));
        assert!(!is_ours("echo tokenmonk --statusline"));
        assert!(!is_ours("/a/tokenmonk --other --statusline"));
        assert!(!is_ours("/a/other --statusline"));
    }

    #[test]
    fn connects_without_settings_file() {
        let e = env(None);
        assert_eq!(e.inspect().state, LimitsConnectionState::NotConnected);
        let c = e.connect().unwrap();
        assert_eq!(c.state, LimitsConnectionState::Connected);
        assert!(!c.previous_status_line);
        assert_eq!(e.settings()["statusLine"]["command"], CMD);
        assert!(!e.claude.path().join(BACKUP_NAME).exists());
    }

    #[test]
    fn disconnect_keeps_later_edits_and_backs_up() {
        let e = env(Some(r#"{"theme":"dark"}"#));
        e.connect().unwrap();
        assert_eq!(
            std::fs::read_to_string(e.claude.path().join(BACKUP_NAME)).unwrap(),
            r#"{"theme":"dark"}"#
        );
        let mut s = e.settings();
        s["model"] = json!("x");
        std::fs::write(e.claude.path().join(SETTINGS_NAME), s.to_string()).unwrap();
        let c = e.disconnect().unwrap();
        assert_eq!(c.state, LimitsConnectionState::NotConnected);
        assert_eq!(e.settings(), json!({"theme":"dark","model":"x"}));
        assert!(!e.data.path().join(PREVIOUS_NAME).exists());
    }

    #[test]
    fn wraps_an_existing_entry_and_restores_it() {
        let orig = json!({"type":"command","command":"echo hi","padding":2,"refreshInterval":5});
        let e = env(Some(&json!({"statusLine": orig, "a": 1}).to_string()));
        assert!(e.inspect().previous_status_line);
        let c = e.connect().unwrap();
        assert!(c.previous_status_line);
        let s = e.settings();
        assert_eq!(
            s["statusLine"]["command"],
            format!("{CMD} {PASSTHROUGH_FLAG}")
        );
        assert_eq!(s["statusLine"]["padding"], 2);
        assert_eq!(s["statusLine"]["refreshInterval"], 5);
        assert_eq!(previous_command(e.data.path()).unwrap(), "echo hi");
        e.disconnect().unwrap();
        assert_eq!(e.settings(), json!({"statusLine": orig, "a": 1}));
    }

    #[test]
    fn connect_twice_does_not_double_wrap() {
        let e = env(Some(
            r#"{"statusLine":{"type":"command","command":"echo hi"}}"#,
        ));
        e.connect().unwrap();
        let first = e.text();
        e.connect().unwrap();
        assert_eq!(e.text(), first);
        assert_eq!(previous_command(e.data.path()).unwrap(), "echo hi");
    }

    #[test]
    fn round_trip_keeps_key_order() {
        let raw = r#"{"zeta":1,"alpha":{"b":1,"a":2},"statusLine":{"padding":1,"type":"command","command":"echo hi"},"mid":[3,1,2]}"#;
        let e = env(Some(raw));
        e.connect().unwrap();
        assert_eq!(keys(&e.settings()), ["zeta", "alpha", "statusLine", "mid"]);
        assert_eq!(
            keys(&e.settings()["statusLine"]),
            ["padding", "type", "command"]
        );
        e.disconnect().unwrap();
        assert_eq!(keys(&e.settings()), ["zeta", "alpha", "statusLine", "mid"]);
        assert_eq!(keys(&e.settings()["alpha"]), ["b", "a"]);
        assert_eq!(
            keys(&e.settings()["statusLine"]),
            ["padding", "type", "command"]
        );
        // Without a previous entry the key goes away; the rest keeps its order.
        let e = env(Some(r#"{"zeta":1,"alpha":2,"mid":3}"#));
        e.connect().unwrap();
        e.disconnect().unwrap();
        assert_eq!(keys(&e.settings()), ["zeta", "alpha", "mid"]);
    }

    #[test]
    fn stale_previous_record_is_dropped_when_the_entry_is_gone() {
        let e = env(Some(
            r#"{"statusLine":{"type":"command","command":"echo X"}}"#,
        ));
        e.connect().unwrap();
        let mut s = e.settings();
        s.as_object_mut().unwrap().remove("statusLine");
        std::fs::write(e.claude.path().join(SETTINGS_NAME), s.to_string()).unwrap();
        let c = e.connect().unwrap();
        assert!(!c.previous_status_line);
        e.disconnect().unwrap();
        assert!(e.settings().get("statusLine").is_none());
    }

    #[test]
    fn foreign_entry_survives_disconnect_untouched() {
        let raw = r#"{"statusLine":{"type":"command","command":"echo hi"}}"#;
        let e = env(Some(raw));
        assert_eq!(e.inspect().state, LimitsConnectionState::NotConnected);
        e.disconnect().unwrap();
        assert_eq!(e.text(), raw);
        assert!(!e.claude.path().join(BACKUP_NAME).exists());
    }

    #[test]
    fn blocked_cases_never_modify_anything() {
        for raw in ["not json", "[1,2]", r#"{"statusLine":"echo"}"#] {
            let e = env(Some(raw));
            let c = e.inspect();
            assert_eq!(c.state, LimitsConnectionState::Blocked);
            assert_eq!(c.problem, Some(LimitsConnectionProblem::SettingsInvalid));
            assert!(matches!(e.connect(), Err(ConnectError::Blocked(_))));
            assert!(matches!(e.disconnect(), Err(ConnectError::Blocked(_))));
            assert_eq!(e.text(), raw);
            assert!(!e.claude.path().join(BACKUP_NAME).exists());
        }
        let data = tempfile::tempdir().unwrap();
        let c = inspect(None, data.path());
        assert_eq!(c.problem, Some(LimitsConnectionProblem::NoClaudeFolder));
        let missing = data.path().join("nope");
        let c = inspect(Some(&missing), data.path());
        assert_eq!(c.problem, Some(LimitsConnectionProblem::NoClaudeFolder));
        assert!(!missing.exists());
    }

    #[test]
    fn non_command_entry_is_stored_and_restored() {
        let raw = r#"{"statusLine":{"type":"other","x":1}}"#;
        let e = env(Some(raw));
        e.connect().unwrap();
        assert_eq!(e.settings()["statusLine"]["type"], "command");
        assert_eq!(e.settings()["statusLine"]["x"], 1);
        assert!(run_passthrough(e.data.path(), b"{}").is_none());
        e.disconnect().unwrap();
        assert_eq!(e.settings(), serde_json::from_str::<Value>(raw).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn passthrough_returns_the_old_output_unchanged() {
        let e = env(Some(
            r#"{"statusLine":{"type":"command","command":"cat; printf ' done'; exit 3"}}"#,
        ));
        e.connect().unwrap();
        let (out, code) = run_passthrough(e.data.path(), b"payload").unwrap();
        assert_eq!(out, b"payload done");
        assert_eq!(code, 3);
    }

    #[cfg(unix)]
    #[test]
    fn a_slow_command_is_cut_off_even_with_a_grandchild() {
        let start = Instant::now();
        let (out, _) = run_shell("sleep 30; echo x", b"", Duration::from_secs(1)).unwrap();
        assert!(start.elapsed() < Duration::from_secs(5));
        assert!(out.is_empty());
    }

    #[test]
    fn no_passthrough_without_a_previous_entry() {
        let e = env(None);
        e.connect().unwrap();
        assert!(run_passthrough(e.data.path(), b"{}").is_none());
    }
}
