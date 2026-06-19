use serde::Serialize;

#[derive(Serialize, Clone)]
pub(crate) struct PortableRuntimeReadyEvent {
    pub(crate) ok: bool,
    pub(crate) runtime_dir: String,
    pub(crate) message: String,
}

pub(crate) fn show_startup_error(message: &str) {
    #[cfg(target_os = "windows")]
    {
        use windows::core::PCWSTR;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

        let title = "GW Studio startup error"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let text = message
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        unsafe {
            let _ = MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                PCWSTR(title.as_ptr()),
                MB_OK | MB_ICONERROR,
            );
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("{message}");
    }
}
