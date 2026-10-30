//! Finding the Claude Code log files (read only).

use std::fs;
use std::path::{Path, PathBuf};

/// Deepest directory level searched below the root. Real trees are
/// `<project>/<session>.jsonl` and `<project>/<session>/subagents/*.jsonl`.
const MAX_DEPTH: usize = 8;

/// Why discovery produced no list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoverError {
    /// The root does not exist (yet) or is not a directory.
    Missing,
    /// The root exists but cannot be read.
    Unreadable(String),
}

/// `<home>/.claude/projects`, if the home directory is known.
pub fn projects_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("projects"))
}

/// All `*.jsonl` files below `root`, sorted. Unreadable subdirectories are
/// skipped; symbolic links to directories are not followed.
pub fn discover(root: &Path) -> Result<Vec<PathBuf>, DiscoverError> {
    match fs::metadata(root) {
        Ok(m) if m.is_dir() => {}
        Ok(_) => return Err(DiscoverError::Missing),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(DiscoverError::Missing),
        Err(e) => return Err(DiscoverError::Unreadable(e.to_string())),
    }
    let mut out = Vec::new();
    // The root itself must be listable; deeper failures are tolerated.
    let top = fs::read_dir(root).map_err(|e| DiscoverError::Unreadable(e.to_string()))?;
    visit(top, 0, &mut out);
    out.sort();
    Ok(out)
}

fn visit(entries: fs::ReadDir, depth: usize, out: &mut Vec<PathBuf>) {
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if depth < MAX_DEPTH {
                if let Ok(sub) = fs::read_dir(&path) {
                    visit(sub, depth + 1, out);
                }
            }
        } else if is_jsonl(&path) && (kind.is_file() || is_file_link(&path)) {
            out.push(path);
        }
    }
}

fn is_file_link(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|m| m.is_file())
}

/// Whether the path names a log file.
pub fn is_jsonl(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "jsonl")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_nested_jsonl_only_and_sorts() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("proj-a/sess-1/subagents")).unwrap();
        fs::create_dir_all(root.join("proj-b")).unwrap();
        for f in [
            "proj-b/s2.jsonl",
            "proj-a/s1.jsonl",
            "proj-a/sess-1/subagents/agent-x.jsonl",
            "proj-a/notes.txt",
            "proj-a/s1.jsonl.bak",
            "top.jsonl",
        ] {
            fs::write(root.join(f), "{}\n").unwrap();
        }
        let found = discover(root).unwrap();
        let rel: Vec<String> = found
            .iter()
            .map(|p| {
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert_eq!(
            rel,
            [
                "proj-a/s1.jsonl",
                "proj-a/sess-1/subagents/agent-x.jsonl",
                "proj-b/s2.jsonl",
                "top.jsonl"
            ]
        );
    }

    #[test]
    fn missing_root_and_file_root_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            discover(&dir.path().join("nope")),
            Err(DiscoverError::Missing)
        );
        let file = dir.path().join("f.jsonl");
        fs::write(&file, "x").unwrap();
        assert_eq!(discover(&file), Err(DiscoverError::Missing));
    }

    #[test]
    fn empty_root_is_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(discover(dir.path()), Ok(vec![]));
    }

    #[test]
    fn depth_is_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let mut deep = dir.path().to_path_buf();
        for i in 0..(MAX_DEPTH + 3) {
            deep.push(format!("d{i}"));
        }
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("deep.jsonl"), "{}\n").unwrap();
        assert_eq!(discover(dir.path()), Ok(vec![]));
    }
}
