use serde::Serialize;
use std::io::{self, BufRead};
use tauri::Emitter;

#[derive(Serialize, Clone)]
struct BuildProgressEvent {
    progress: f64,
    message: String,
}

pub(crate) fn emit_build_progress(app: &tauri::AppHandle, progress: f64, message: impl Into<String>) {
    let _ = app.emit(
        "build-progress",
        BuildProgressEvent {
            progress,
            message: message.into(),
        },
    );
}

pub(crate) fn read_lossy_output_line<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut bytes = Vec::new();
    let read = reader.read_until(b'\n', &mut bytes)?;
    if read == 0 {
        return Ok(None);
    }
    if bytes.ends_with(b"\n") {
        bytes.pop();
        if bytes.ends_with(b"\r") {
            bytes.pop();
        }
    }
    Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
}
