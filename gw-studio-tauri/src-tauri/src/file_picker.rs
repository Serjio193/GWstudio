use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize)]
pub(crate) struct BinaryFilePathRequest {
    path: String,
}

#[derive(Deserialize)]
pub(crate) struct RevealPathRequest {
    path: String,
}

#[derive(Deserialize)]
pub(crate) struct BinFilePickerRequest {
    title: String,
    default_path: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct BinFilePickerResult {
    name: String,
    path: String,
}

#[tauri::command]
pub(crate) fn read_binary_file(request: BinaryFilePathRequest) -> Result<Vec<u8>, String> {
    let path = PathBuf::from(&request.path);
    let metadata = fs::metadata(&path).map_err(|error| format!("failed to stat file: {error}"))?;
    if !metadata.is_file() {
        return Err("path is not a file".to_string());
    }
    fs::read(&path).map_err(|error| format!("failed to read file: {error}"))
}

#[tauri::command]
pub(crate) fn reveal_path_in_explorer(request: RevealPathRequest) -> Result<(), String> {
    let path = PathBuf::from(&request.path);
    let target = if path.exists() {
        path
    } else {
        return Err("path does not exist".to_string());
    };

    let mut command = Command::new("explorer.exe");
    if target.is_file() {
        command.arg("/select,").arg(&target);
    } else {
        command.arg(&target);
    }

    command
        .spawn()
        .map_err(|error| format!("failed to open explorer: {error}"))?;
    Ok(())
}

#[tauri::command]
pub(crate) fn select_bin_file(request: BinFilePickerRequest) -> Result<Option<BinFilePickerResult>, String> {
    select_bin_file_native(&request.title, request.default_path.as_deref())
}

#[cfg(target_os = "windows")]
fn select_bin_file_native(title: &str, default_path: Option<&str>) -> Result<Option<BinFilePickerResult>, String> {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, IShellItem, SHCreateItemFromParsingName, SIGDN_FILESYSPATH,
    };

    struct ComApartment {
        initialized: bool,
    }

    impl Drop for ComApartment {
        fn drop(&mut self) {
            if self.initialized {
                unsafe {
                    CoUninitialize();
                }
            }
        }
    }

    unsafe {
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let apartment = if init.is_ok() {
            ComApartment { initialized: true }
        } else if init == RPC_E_CHANGED_MODE {
            ComApartment { initialized: false }
        } else {
            return Err(format!("failed to initialize COM: {init:?}"));
        };

        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("failed to create native file picker: {error}"))?;
        let title = HSTRING::from(title);
        dialog
            .SetTitle(PCWSTR(title.as_ptr()))
            .map_err(|error| format!("failed to set file picker title: {error}"))?;

        let filters = [
            COMDLG_FILTERSPEC {
                pszName: w!("Binary files (*.bin)"),
                pszSpec: w!("*.bin"),
            },
            COMDLG_FILTERSPEC {
                pszName: w!("All files (*.*)"),
                pszSpec: w!("*.*"),
            },
        ];
        dialog
            .SetFileTypes(&filters)
            .map_err(|error| format!("failed to set file picker filters: {error}"))?;
        dialog
            .SetFileTypeIndex(1)
            .map_err(|error| format!("failed to set file picker filter index: {error}"))?;

        if let Some(default_path) = default_path.and_then(resolve_file_picker_folder) {
            let folder = HSTRING::from(default_path.to_string_lossy().as_ref());
            if let Ok(shell_item) = SHCreateItemFromParsingName::<_, _, IShellItem>(
                PCWSTR(folder.as_ptr()),
                None,
            ) {
                let _ = dialog.SetFolder(&shell_item);
                let _ = dialog.SetDefaultFolder(&shell_item);
            }
        }

        if let Err(error) = dialog.Show(None) {
            let _ = apartment;
            let code = error.code().0 as u32;
            if code == 0x800704C7 {
                return Ok(None);
            }
            return Err(format!("native file picker failed: {error}"));
        }

        let item = dialog
            .GetResult()
            .map_err(|error| format!("failed to read selected file: {error}"))?;
        let path_ptr = item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .map_err(|error| format!("failed to read selected file path: {error}"))?;
        let path_text = path_ptr
            .to_string()
            .map_err(|error| format!("failed to decode selected file path: {error}"))?;
        CoTaskMemFree(Some(path_ptr.as_ptr().cast()));

        if path_text.trim().is_empty() {
            return Ok(None);
        }
        let path = PathBuf::from(&path_text);
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&path_text)
            .to_string();
        Ok(Some(BinFilePickerResult {
            name,
            path: path_text,
        }))
    }
}

#[cfg(not(target_os = "windows"))]
fn select_bin_file_native(_title: &str, _default_path: Option<&str>) -> Result<Option<BinFilePickerResult>, String> {
    Err("native file picker is only implemented on Windows".to_string())
}

fn resolve_file_picker_folder(default_path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(default_path.trim());
    if path.is_dir() {
        return Some(path);
    }
    if path.is_file() {
        return path.parent().map(Path::to_path_buf);
    }
    path.parent()
        .filter(|parent| parent.is_dir())
        .map(Path::to_path_buf)
        .or_else(|| {
            if path.exists() {
                None
            } else if default_path.trim().ends_with('\\') || default_path.trim().ends_with('/') {
                None
            } else {
                path.parent().map(Path::to_path_buf)
            }
        })
}
