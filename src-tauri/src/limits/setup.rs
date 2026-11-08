//! The status-line setup command shown in Settings (`get_limit_setup`).

use std::path::Path;
use std::time::UNIX_EPOCH;

use super::bridge::{self, BridgeFile};
use crate::contracts::{EpochMs, StatuslineFileInfo};

/// Flag that switches the executable into bridge mode.
pub const BRIDGE_FLAG: &str = "--statusline";

/// `<executable> --statusline` in the POSIX shell form (macOS shell, Git
/// Bash on Windows). On Windows the path uses forward slashes; it is wrapped
/// in double quotes only if it contains a space.
pub fn statusline_command(exe: &Path, windows: bool) -> Option<String> {
    let mut path = exe.to_str()?.to_owned();
    if windows {
        path = path.replace('\\', "/");
        // Strip the verbatim prefix some APIs return.
        if let Some(rest) = path.strip_prefix("//?/") {
            path = rest.to_owned();
        }
    }
    if path.is_empty() {
        return None;
    }
    if path.contains(' ') {
        path = format!("\"{path}\"");
    }
    Some(format!("{path} {BRIDGE_FLAG}"))
}

/// The command for the running executable.
pub fn current_statusline_command() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    statusline_command(&exe, cfg!(windows))
}

/// State of the bridge file at the default location. Never fails.
pub fn current_statusline_file() -> StatuslineFileInfo {
    bridge::statusline_path()
        .map(|p| statusline_file_info(&p))
        .unwrap_or_default()
}

/// State of the bridge file at `path`. Any error maps to the missing or
/// partial values; the content is only checked for a valid `receivedAt`.
pub fn statusline_file_info(path: &Path) -> StatuslineFileInfo {
    let Ok(meta) = std::fs::metadata(path) else {
        return StatuslineFileInfo::default();
    };
    if !meta.is_file() {
        return StatuslineFileInfo::default();
    }
    let modified_at = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| EpochMs::try_from(d.as_millis()).ok());
    let received_at = (meta.len() <= bridge::MAX_INPUT_BYTES)
        .then(|| std::fs::read(path).ok())
        .flatten()
        .and_then(|b| serde_json::from_slice::<BridgeFile>(&b).ok())
        .filter(|f| f.version == bridge::FILE_VERSION)
        .map(|f| f.received_at);
    StatuslineFileInfo {
        exists: true,
        received_at,
        modified_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn plain_path_is_unquoted() {
        let p = PathBuf::from("/opt/tokenmonk/tokenmonk");
        assert_eq!(
            statusline_command(&p, false).unwrap(),
            "/opt/tokenmonk/tokenmonk --statusline"
        );
    }

    #[test]
    fn windows_path_uses_forward_slashes() {
        let p = PathBuf::from(r"C:\Apps\Tokenmonk\tokenmonk.exe");
        assert_eq!(
            statusline_command(&p, true).unwrap(),
            "C:/Apps/Tokenmonk/tokenmonk.exe --statusline"
        );
        let verbatim = PathBuf::from(r"\\?\C:\Apps\tokenmonk.exe");
        assert_eq!(
            statusline_command(&verbatim, true).unwrap(),
            "C:/Apps/tokenmonk.exe --statusline"
        );
    }

    #[test]
    fn path_with_a_space_is_quoted() {
        let p = PathBuf::from(r"C:\Example Dir\tokenmonk.exe");
        assert_eq!(
            statusline_command(&p, true).unwrap(),
            "\"C:/Example Dir/tokenmonk.exe\" --statusline"
        );
    }

    #[test]
    fn empty_path_gives_none() {
        assert_eq!(statusline_command(Path::new(""), false), None);
    }

    #[test]
    fn file_info_missing() {
        let dir = tempfile::tempdir().unwrap();
        let info = statusline_file_info(&dir.path().join("statusline.json"));
        assert_eq!(info, StatuslineFileInfo::default());
    }

    #[test]
    fn file_info_valid() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("statusline.json");
        std::fs::write(
            &p,
            r#"{"version":1,"receivedAt":1700000000000,"fiveHour":{"usedPercentage":12.0,"resetsAt":null}}"#,
        )
        .unwrap();
        let info = statusline_file_info(&p);
        assert!(info.exists);
        assert_eq!(info.received_at, Some(1_700_000_000_000));
        assert!(info.modified_at.is_some());
    }

    #[test]
    fn file_info_invalid_content_keeps_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("statusline.json");
        std::fs::write(&p, "not json").unwrap();
        let info = statusline_file_info(&p);
        assert!(info.exists);
        assert_eq!(info.received_at, None);
        assert!(info.modified_at.is_some());
    }

    #[test]
    fn file_info_directory_counts_as_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            statusline_file_info(dir.path()),
            StatuslineFileInfo::default()
        );
    }

    #[test]
    fn file_info_unknown_version_has_no_received_at() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("statusline.json");
        std::fs::write(
            &p,
            r#"{"version":2,"receivedAt":1,"fiveHour":null,"sevenDay":null}"#,
        )
        .unwrap();
        let info = statusline_file_info(&p);
        assert!(info.exists);
        assert_eq!(info.received_at, None);
        assert!(info.modified_at.is_some());
    }
}
