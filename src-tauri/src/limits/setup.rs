//! The status-line setup command shown in Settings (`get_limit_setup`).

use std::path::Path;

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
}
