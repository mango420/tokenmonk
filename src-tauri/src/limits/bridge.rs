//! Status-line bridge (docs/ARCHITECTURE.md, "Limits", "Bridge mode").
//!
//! Claude Code pipes a JSON object to its status-line command. When the
//! executable starts as `tokenmonk --statusline`, [`run`] keeps only the
//! `rate_limits` part, writes it atomically to `<app data dir>/statusline.json`
//! and prints one short line. Nothing else of the input is kept, stored or
//! logged. The bridge never fails: bad input, I/O errors and missing
//! directories end in the fallback line, and the exit code stays 0.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::contracts::EpochMs;

/// Bundle identifier from `tauri.conf.json`; Tauri's `app_data_dir()` is
/// `dirs::data_dir()` joined with it (the app sets no
/// `appDirectoriesOverride`).
pub const APP_IDENTIFIER: &str = "app.tokenmonk.desktop";
/// Name of the bridge file inside the app data directory.
pub const FILE_NAME: &str = "statusline.json";
/// Format version of the bridge file.
pub const FILE_VERSION: u32 = 1;
/// More input than this counts as bad input.
pub const MAX_INPUT_BYTES: u64 = 1024 * 1024;
/// Printed when no window could be read.
pub const FALLBACK_LINE: &str = "Tokenmonk: no limit data yet";

/// The app data directory, resolved without a running Tauri app. Bridge,
/// file source and database location all use this one helper.
pub fn app_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join(APP_IDENTIFIER))
}

/// Full path of the bridge file.
pub fn statusline_path() -> Option<PathBuf> {
    app_data_dir().map(|d| d.join(FILE_NAME))
}

/// One rate-limit window as stored in the bridge file.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeWindow {
    /// Used share, 0 to 100 (may exceed 100).
    pub used_percentage: f64,
    /// Window reset, epoch milliseconds.
    pub resets_at: Option<EpochMs>,
}

/// Content of `statusline.json`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeFile {
    pub version: u32,
    /// When the bridge ran, epoch milliseconds.
    pub received_at: EpochMs,
    pub five_hour: Option<BridgeWindow>,
    pub seven_day: Option<BridgeWindow>,
}

impl BridgeFile {
    fn has_window(&self) -> bool {
        self.five_hour.is_some() || self.seven_day.is_some()
    }
}

/// Reads one window from the status-line input. Valid if `used_percentage`
/// is a finite number >= 0; `resets_at` (seconds) counts only if it is a
/// finite number > 0.
fn parse_window(value: Option<&Value>) -> Option<BridgeWindow> {
    let window = value?;
    let used = window.get("used_percentage")?.as_f64()?;
    if !used.is_finite() || used < 0.0 {
        return None;
    }
    let resets_at = window
        .get("resets_at")
        .and_then(Value::as_f64)
        .filter(|s| s.is_finite() && *s > 0.0 && *s < 9.0e12)
        .map(|s| (s * 1000.0).round() as EpochMs);
    Some(BridgeWindow {
        used_percentage: used,
        resets_at,
    })
}

/// Extracts the bridge file content from the raw status-line input. `None`
/// if the input is not JSON or has no valid window.
pub fn parse_input(input: &[u8], now: EpochMs) -> Option<BridgeFile> {
    let value: Value = serde_json::from_slice(input).ok()?;
    let limits = value.get("rate_limits")?;
    let file = BridgeFile {
        version: FILE_VERSION,
        received_at: now,
        five_hour: parse_window(limits.get("five_hour")),
        seven_day: parse_window(limits.get("seven_day")),
    };
    file.has_window().then_some(file)
}

/// The short status line, for example `5h 24% | 7d 41%`.
pub fn output_line(file: Option<&BridgeFile>) -> String {
    let Some(file) = file else {
        return FALLBACK_LINE.to_owned();
    };
    let mut parts = Vec::new();
    if let Some(w) = &file.five_hour {
        parts.push(format!("5h {}%", w.used_percentage.floor() as i64));
    }
    if let Some(w) = &file.seven_day {
        parts.push(format!("7d {}%", w.used_percentage.floor() as i64));
    }
    if parts.is_empty() {
        FALLBACK_LINE.to_owned()
    } else {
        parts.join(" | ")
    }
}

/// Writes `file` to `<dir>/statusline.json` via a temp file in the same
/// directory and a rename, creating the directory if needed.
pub fn write_atomic(dir: &Path, file: &BridgeFile) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let target = dir.join(FILE_NAME);
    let tmp = dir.join(format!("{FILE_NAME}.{}.tmp", std::process::id()));
    let json = serde_json::to_vec(file).map_err(io::Error::other)?;
    let result = std::fs::write(&tmp, json).and_then(|()| std::fs::rename(&tmp, &target));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Core of the bridge: input bytes in, line out; writes the file into `dir`
/// when the input has a valid window. Every failure ends in the fallback line.
pub fn process(input: &[u8], now: EpochMs, dir: Option<&Path>) -> String {
    let parsed = parse_input(input, now);
    if let (Some(file), Some(dir)) = (&parsed, dir) {
        let _ = write_atomic(dir, file);
    }
    output_line(parsed.as_ref())
}

fn now_ms() -> EpochMs {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// Reads at most [`MAX_INPUT_BYTES`]; `None` if there is more or the read fails.
fn read_bounded(mut input: impl Read) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    input
        .by_ref()
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut buf)
        .ok()?;
    (buf.len() as u64 <= MAX_INPUT_BYTES).then_some(buf)
}

/// Bridge mode of the executable: stdin to stdout, then return (exit code 0).
/// Never panics, never writes to stderr.
pub fn run() {
    let input = read_bounded(io::stdin().lock()).unwrap_or_default();
    let line = process(&input, now_ms(), app_data_dir().as_deref());
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: EpochMs = 1_767_225_600_000;

    fn input(five: &str, seven: &str) -> Vec<u8> {
        format!(
            r#"{{"session_id":"s","cwd":"/x","rate_limits":{{"five_hour":{five},"seven_day":{seven}}}}}"#
        )
        .into_bytes()
    }

    #[test]
    fn parses_both_windows_and_converts_seconds() {
        let raw = input(
            r#"{"used_percentage":23.5,"resets_at":1767240000}"#,
            r#"{"used_percentage":41,"resets_at":1767600000}"#,
        );
        let f = parse_input(&raw, NOW).unwrap();
        assert_eq!(f.version, 1);
        assert_eq!(f.received_at, NOW);
        assert_eq!(
            f.five_hour,
            Some(BridgeWindow {
                used_percentage: 23.5,
                resets_at: Some(1_767_240_000_000)
            })
        );
        assert_eq!(f.seven_day.unwrap().used_percentage, 41.0);
    }

    #[test]
    fn one_missing_or_invalid_window_is_left_out() {
        let raw = input(r#"{"used_percentage":-1}"#, r#"{"used_percentage":5}"#);
        let f = parse_input(&raw, NOW).unwrap();
        assert_eq!(f.five_hour, None);
        assert_eq!(f.seven_day.unwrap().resets_at, None);
        let raw = br#"{"rate_limits":{"five_hour":{"used_percentage":"high"}}}"#;
        assert_eq!(parse_input(raw, NOW), None);
    }

    #[test]
    fn resets_at_must_be_positive() {
        let raw = input(
            r#"{"used_percentage":1,"resets_at":0}"#,
            r#"{"used_percentage":2,"resets_at":-5}"#,
        );
        let f = parse_input(&raw, NOW).unwrap();
        assert_eq!(f.five_hour.unwrap().resets_at, None);
        assert_eq!(f.seven_day.unwrap().resets_at, None);
    }

    #[test]
    fn bad_input_yields_nothing() {
        assert_eq!(parse_input(b"", NOW), None);
        assert_eq!(parse_input(b"not json", NOW), None);
        assert_eq!(parse_input(br#"{"model":"x"}"#, NOW), None);
        assert_eq!(parse_input(br#"{"rate_limits":null}"#, NOW), None);
        assert_eq!(parse_input(br#"{"rate_limits":{}}"#, NOW), None);
    }

    #[test]
    fn output_line_rounds_down_and_skips_missing_windows() {
        let raw = input(r#"{"used_percentage":24.9}"#, r#"{"used_percentage":41.2}"#);
        let f = parse_input(&raw, NOW);
        assert_eq!(output_line(f.as_ref()), "5h 24% | 7d 41%");
        let raw = input("null", r#"{"used_percentage":7}"#);
        let f = parse_input(&raw, NOW);
        assert_eq!(output_line(f.as_ref()), "7d 7%");
        assert_eq!(output_line(None), FALLBACK_LINE);
    }

    #[test]
    fn process_writes_the_file_and_keeps_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("nested");
        let raw = input(r#"{"used_percentage":10,"resets_at":1767240000}"#, "null");
        let line = process(&raw, NOW, Some(&sub));
        assert_eq!(line, "5h 10%");
        let text = std::fs::read_to_string(sub.join(FILE_NAME)).unwrap();
        assert!(!text.contains("session_id") && !text.contains("/x"));
        let back: BridgeFile = serde_json::from_str(&text).unwrap();
        assert_eq!(back.received_at, NOW);
        assert_eq!(back.seven_day, None);
        // No stray temp files.
        assert_eq!(std::fs::read_dir(&sub).unwrap().count(), 1);
    }

    #[test]
    fn bad_input_leaves_an_existing_file_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let good = input(r#"{"used_percentage":10}"#, "null");
        process(&good, NOW, Some(dir.path()));
        let before = std::fs::read(dir.path().join(FILE_NAME)).unwrap();
        let line = process(b"garbage", NOW + 1, Some(dir.path()));
        assert_eq!(line, FALLBACK_LINE);
        assert_eq!(std::fs::read(dir.path().join(FILE_NAME)).unwrap(), before);
    }

    #[test]
    fn unwritable_directory_still_prints_the_line() {
        let dir = tempfile::tempdir().unwrap();
        // A file where the directory should be makes create_dir_all fail.
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, b"x").unwrap();
        let raw = input(r#"{"used_percentage":10}"#, "null");
        assert_eq!(process(&raw, NOW, Some(&blocker.join("sub"))), "5h 10%");
        assert_eq!(process(&raw, NOW, None), "5h 10%");
    }

    #[test]
    fn oversized_input_is_rejected() {
        let big = vec![b' '; MAX_INPUT_BYTES as usize + 1];
        assert_eq!(read_bounded(&big[..]), None);
        let ok = vec![b' '; MAX_INPUT_BYTES as usize];
        assert_eq!(read_bounded(&ok[..]).map(|v| v.len()), Some(ok.len()));
    }

    #[test]
    fn data_dir_ends_in_the_bundle_identifier() {
        if let Some(p) = statusline_path() {
            assert!(p.ends_with(std::path::Path::new(APP_IDENTIFIER).join(FILE_NAME)));
        }
    }
}
