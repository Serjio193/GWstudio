use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::backups::device_backups_dir;

static CURRENT_DEVICE_UID: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn current_device_uid_cell() -> &'static Mutex<Option<String>> {
    CURRENT_DEVICE_UID.get_or_init(|| Mutex::new(None))
}

pub(crate) fn set_current_device_uid(uid: Option<String>) {
    if let Ok(mut slot) = current_device_uid_cell().lock() {
        *slot = uid;
    }
}

fn current_device_uid() -> Option<String> {
    current_device_uid_cell().lock().ok().and_then(|slot| slot.clone())
}

pub(crate) fn required_active_backups_dir() -> Result<PathBuf, String> {
    current_device_uid()
        .filter(|uid| !uid.trim().is_empty() && uid != "UNKNOWN")
        .map(|uid| device_backups_dir(&uid))
        .ok_or_else(|| "MCU UID is unknown; run Read Device Info before reading backups".to_string())
}
