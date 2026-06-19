use crate::paths::{
    find_dir_with_files_under, find_file_under, host_root, runtime_tools_dir, tool_candidate,
};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn locate_python_exe() -> PathBuf {
    [
        tool_candidate(&["python", "python.exe"]),
        tool_candidate(&["python", "python3.exe"]),
        tool_candidate(&["python", "Scripts", "python.exe"]),
        host_root().join("python").join("python.exe"),
    ]
    .into_iter()
    .find(|path| path.exists())
    .unwrap_or_else(|| PathBuf::from("python"))
}

pub(crate) fn locate_make_exe() -> Option<PathBuf> {
    let candidates = [
        tool_candidate(&["make", "bin", "mingw32-make.exe"]),
        tool_candidate(&["make", "bin", "make.exe"]),
        tool_candidate(&["mingw64", "bin", "mingw32-make.exe"]),
        tool_candidate(&["mingw64", "bin", "make.exe"]),
        tool_candidate(&["stm32", "make", "bin", "make.exe"]),
    ];
    candidates
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_file_under(&runtime_tools_dir(), "mingw32-make.exe"))
        .or_else(|| find_file_under(&runtime_tools_dir(), "make.exe"))
}

pub(crate) fn locate_gcc_bin_dir() -> Option<PathBuf> {
    let candidates = [
        tool_candidate(&["gcc-arm-none-eabi", "bin"]),
        tool_candidate(&["arm-none-eabi-gcc", "bin"]),
        tool_candidate(&["gcc", "bin"]),
        tool_candidate(&["stm32", "gcc", "bin"]),
    ];
    candidates
        .into_iter()
        .find(|path| path.join("arm-none-eabi-gcc.exe").exists())
        .or_else(|| {
            find_file_under(&runtime_tools_dir(), "arm-none-eabi-gcc.exe")
                .and_then(|path| path.parent().map(Path::to_path_buf))
        })
}

pub(crate) fn locate_git_bin_dir() -> Option<PathBuf> {
    let candidates = [
        tool_candidate(&["git", "bin"]),
        tool_candidate(&["git", "cmd"]),
    ];
    candidates
        .into_iter()
        .find(|path| path.join("bash.exe").exists() && path.join("sh.exe").exists())
        .or_else(|| find_dir_with_files_under(&runtime_tools_dir(), &["bash.exe", "sh.exe"]))
}

fn uses_flat_msys(git_bin: &Path) -> bool {
    git_bin.join("msys-2.0.dll").exists()
        && git_bin
            .parent()
            .map(|git_root| !git_root.join("usr").join("bin").join("bash.exe").exists())
            .unwrap_or(false)
}

pub(crate) fn to_bash_path_for(path: &Path, git_bin: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if normalized.len() >= 3 && normalized.as_bytes()[1] == b':' && normalized.as_bytes()[2] == b'/' {
        let drive = normalized[..1].to_ascii_lowercase();
        if uses_flat_msys(git_bin) {
            format!("/cygdrive/{drive}{}", &normalized[2..])
        } else {
            format!("/{drive}{}", &normalized[2..])
        }
    } else {
        normalized
    }
}

pub(crate) fn ensure_msys_tmp_dir(git_bin: &Path) -> Result<(), String> {
    if !uses_flat_msys(git_bin) {
        return Ok(());
    }
    let Some(git_root) = git_bin.parent() else {
        return Ok(());
    };
    let Some(tools_root) = git_root.parent() else {
        return Ok(());
    };
    fs::create_dir_all(tools_root.join("tmp"))
        .map_err(|error| format!("failed to create MSYS tmp dir: {error}"))
}

pub(crate) fn build_job_count() -> usize {
    std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4)
        .clamp(1, 24)
}
