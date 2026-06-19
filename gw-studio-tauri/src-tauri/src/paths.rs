use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static PORTABLE_RUNTIME_ROOT: OnceLock<PathBuf> = OnceLock::new();

pub(crate) fn host_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub(crate) fn workspace_root() -> PathBuf {
    host_root().join("GameWatchBuilderData")
}

pub(crate) fn portable_runtime_root() -> PathBuf {
    PORTABLE_RUNTIME_ROOT
        .get()
        .cloned()
        .unwrap_or_else(workspace_root)
}

pub(crate) fn current_portable_runtime_root() -> Option<PathBuf> {
    PORTABLE_RUNTIME_ROOT.get().cloned()
}

pub(crate) fn set_portable_runtime_root(path: PathBuf) {
    let _ = PORTABLE_RUNTIME_ROOT.set(path);
}

pub(crate) fn runtime_tools_dir() -> PathBuf {
    portable_runtime_root().join("tools")
}

pub(crate) fn portable_source_dir() -> PathBuf {
    portable_runtime_root().join("sources")
}

pub(crate) fn tool_candidate(parts: &[&str]) -> PathBuf {
    parts
        .iter()
        .fold(runtime_tools_dir(), |path, part| path.join(part))
}

pub(crate) fn find_file_under(root: &Path, file_name: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case(file_name))
                .unwrap_or(false)
            {
                return Some(path);
            }
        }
    }
    None
}

pub(crate) fn find_dir_with_files_under(root: &Path, required_files: &[&str]) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if required_files.iter().all(|file| dir.join(file).is_file()) {
            return Some(dir);
        }
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            }
        }
    }
    None
}

fn contains_cyrillic(value: &str) -> bool {
    value.chars().any(|ch| {
        matches!(
            ch,
            '\u{0400}'..='\u{04ff}'
                | '\u{0500}'..='\u{052f}'
                | '\u{2de0}'..='\u{2dff}'
                | '\u{a640}'..='\u{a69f}'
        )
    })
}

pub(crate) fn validate_exe_path_for_portable_runtime() -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|error| format!("failed to resolve executable path: {error}"))?;
    let exe_path_text = exe_path.to_string_lossy();
    if contains_cyrillic(&exe_path_text) {
        return Err(format!(
            "GW Studio cannot run from a path containing Cyrillic characters.\n\nCurrent path:\n{}\n\nMove the program to a folder with Latin-only path, for example C:\\GWStudio\\, and start it again.\n\nGW Studio не может работать из папки с кириллицей. Переместите программу в папку без кириллицы, например C:\\GWStudio\\.",
            exe_path.display()
        ));
    }
    Ok(())
}
