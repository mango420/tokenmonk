//! [`LimitSource`] backed by the file the status-line bridge writes
//! (docs/ARCHITECTURE.md, "File source").

use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::bridge::{self, BridgeFile, BridgeWindow, FILE_VERSION};
use crate::contracts::limits::{LimitError, LimitReading, LimitReason, LimitSource, LimitWindow};
use crate::contracts::EpochMs;

/// Id stored with every reading of this source.
pub const SOURCE_ID: &str = "statusline";

/// Reads `statusline.json`. Local file only: no network, no credentials.
#[derive(Debug, Clone)]
pub struct StatuslineSource {
    path: Option<PathBuf>,
}

impl StatuslineSource {
    /// The bridge file in the app data directory.
    pub fn new() -> Self {
        Self {
            path: bridge::statusline_path(),
        }
    }

    /// A specific file (tests).
    pub fn at(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    /// [`LimitSource::fetch`] at an explicit time.
    pub fn fetch_at(&self, now: EpochMs) -> Result<LimitReading, LimitError> {
        let err = |reason, detail: &str| LimitError {
            reason,
            detail: detail.to_owned(),
        };
        let Some(path) = &self.path else {
            return Err(err(LimitReason::NoSource, "no app data directory"));
        };
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return Err(err(LimitReason::NoSource, "status-line bridge not set up"));
            }
            Err(e) => return Err(err(LimitReason::Other, &format!("{:?}", e.kind()))),
        };
        let file: BridgeFile = serde_json::from_slice(&bytes)
            .map_err(|_| err(LimitReason::InvalidResponse, "bridge file is not valid"))?;
        reading_from_file(&file, now)
    }
}

impl Default for StatuslineSource {
    fn default() -> Self {
        Self::new()
    }
}

fn window(w: Option<BridgeWindow>, now: EpochMs) -> Option<LimitWindow> {
    let w = w?;
    if !w.used_percentage.is_finite() || w.used_percentage < 0.0 {
        return None;
    }
    // A window that has reset is dropped, as Claude Code does.
    if w.resets_at.is_some_and(|r| r <= now) {
        return None;
    }
    Some(LimitWindow {
        utilization: w.used_percentage,
        resets_at: w.resets_at,
    })
}

fn reading_from_file(file: &BridgeFile, now: EpochMs) -> Result<LimitReading, LimitError> {
    if file.version != FILE_VERSION {
        return Err(LimitError {
            reason: LimitReason::InvalidResponse,
            detail: format!("unknown bridge file version {}", file.version),
        });
    }
    let five_hour = window(file.five_hour, now);
    let weekly = window(file.seven_day, now);
    if five_hour.is_none() && weekly.is_none() {
        return Err(LimitError {
            reason: LimitReason::NoSource,
            detail: "no current window".to_owned(),
        });
    }
    Ok(LimitReading {
        five_hour,
        weekly,
        fetched_at: file.received_at,
    })
}

fn now_ms() -> EpochMs {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

impl LimitSource for StatuslineSource {
    fn id(&self) -> &'static str {
        SOURCE_ID
    }

    fn fetch(&self) -> Result<LimitReading, LimitError> {
        self.fetch_at(now_ms())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: EpochMs = 1_767_225_600_000;

    fn source_with(json: &str) -> (tempfile::TempDir, StatuslineSource) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(bridge::FILE_NAME);
        std::fs::write(&path, json).unwrap();
        (dir, StatuslineSource::at(path))
    }

    #[test]
    fn missing_file_is_no_source() {
        let dir = tempfile::tempdir().unwrap();
        let s = StatuslineSource::at(dir.path().join("nope.json"));
        let e = s.fetch_at(NOW).unwrap_err();
        assert_eq!(e.reason, LimitReason::NoSource);
        assert_eq!(s.id(), "statusline");
    }

    #[test]
    fn valid_file_becomes_a_reading() {
        let (_d, s) = source_with(
            r#"{"version":1,"receivedAt":1767225000000,
                "fiveHour":{"usedPercentage":23.5,"resetsAt":1767240000000},
                "sevenDay":{"usedPercentage":41.2,"resetsAt":null}}"#,
        );
        let r = s.fetch_at(NOW).unwrap();
        assert_eq!(r.fetched_at, 1_767_225_000_000);
        let five = r.five_hour.unwrap();
        assert_eq!(five.utilization, 23.5);
        assert_eq!(five.resets_at, Some(1_767_240_000_000));
        assert_eq!(r.weekly.unwrap().resets_at, None);
    }

    #[test]
    fn expired_windows_are_dropped_and_none_left_is_no_source() {
        let (_d, s) = source_with(
            r#"{"version":1,"receivedAt":1767000000000,
                "fiveHour":{"usedPercentage":90,"resetsAt":1767225600000},
                "sevenDay":{"usedPercentage":41,"resetsAt":1767600000000}}"#,
        );
        let r = s.fetch_at(NOW).unwrap();
        assert!(r.five_hour.is_none(), "resetsAt == now counts as expired");
        assert!(r.weekly.is_some());
        let e = s.fetch_at(1_767_700_000_000).unwrap_err();
        assert_eq!(e.reason, LimitReason::NoSource);
        assert_eq!(e.detail, "no current window");
    }

    #[test]
    fn broken_files_are_invalid_responses() {
        for json in [
            "not json",
            r#"{"version":2,"receivedAt":1,"fiveHour":null,"sevenDay":null}"#,
            r#"{"version":1,"receivedAt":"x","fiveHour":null,"sevenDay":null}"#,
            r#"{"version":1}"#,
        ] {
            let (_d, s) = source_with(json);
            let e = s.fetch_at(NOW).unwrap_err();
            assert_eq!(e.reason, LimitReason::InvalidResponse, "{json}");
        }
    }

    #[test]
    fn negative_percentages_in_the_file_are_ignored() {
        let (_d, s) = source_with(
            r#"{"version":1,"receivedAt":1,
                "fiveHour":{"usedPercentage":-3,"resetsAt":null},
                "sevenDay":{"usedPercentage":5,"resetsAt":null}}"#,
        );
        let r = s.fetch_at(NOW).unwrap();
        assert!(r.five_hour.is_none() && r.weekly.is_some());
    }
}
