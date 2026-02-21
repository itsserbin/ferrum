use winit::window::Window;

/// Shows a native confirmation dialog before closing a terminal window.
/// Returns `true` when the user confirms closing.
pub fn confirm_window_close(window: &Window) -> bool {
    #[cfg(target_os = "macos")]
    {
        return confirm_window_close_macos(window);
    }

    #[cfg(target_os = "windows")]
    {
        return confirm_window_close_windows(window);
    }

    #[cfg(target_os = "linux")]
    {
        return confirm_window_close_linux(window);
    }

    #[allow(unreachable_code)]
    {
        let _ = window;
        true
    }
}

#[cfg(target_os = "macos")]
fn confirm_window_close_macos(window: &Window) -> bool {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAlert, NSAlertFirstButtonReturn, NSAlertStyle};
    use objc2_foundation::NSString;

    let _ = window;

    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("[ferrum] Close confirmation dialog must run on the main thread");
        return false;
    };

    let t = crate::i18n::t();
    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(NSAlertStyle::Warning);
    alert.setMessageText(&NSString::from_str(t.close_dialog_title));
    alert.setInformativeText(&NSString::from_str(t.close_dialog_body));
    alert.addButtonWithTitle(&NSString::from_str(t.close_dialog_confirm));
    alert.addButtonWithTitle(&NSString::from_str(t.close_dialog_cancel));
    alert.runModal() == NSAlertFirstButtonReturn
}

#[cfg(target_os = "windows")]
fn confirm_window_close_windows(window: &Window) -> bool {
    use std::ptr;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    type Hwnd = *mut core::ffi::c_void;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(
            hwnd: Hwnd,
            lp_text: *const u16,
            lp_caption: *const u16,
            message_type: u32,
        ) -> i32;
    }

    const MB_OKCANCEL: u32 = 0x0000_0001;
    const MB_ICONWARNING: u32 = 0x0000_0030;
    const MB_DEFBUTTON2: u32 = 0x0000_0100;
    const IDOK: i32 = 1;

    let hwnd = window
        .window_handle()
        .ok()
        .and_then(|handle| match handle.as_raw() {
            RawWindowHandle::Win32(win32) => Some(win32.hwnd.get() as Hwnd),
            _ => None,
        })
        .unwrap_or(ptr::null_mut());

    let t = crate::i18n::t();
    let caption = to_wide(t.close_dialog_title);
    let body = format!("{}\n\n{}", t.close_dialog_body, t.close_dialog_title);
    let text = to_wide(&body);

    // SAFETY: Arguments are valid and null-terminated UTF-16 pointers. Parent handle may
    // be null, which MessageBoxW accepts.
    unsafe {
        MessageBoxW(
            hwnd,
            text.as_ptr(),
            caption.as_ptr(),
            MB_OKCANCEL | MB_ICONWARNING | MB_DEFBUTTON2,
        ) == IDOK
    }
}

#[cfg(target_os = "windows")]
fn to_wide(text: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_os = "linux")]
fn confirm_window_close_linux(window: &Window) -> bool {
    let _ = window;

    let t = crate::i18n::t();
    let title_arg = format!("--title={}", t.close_dialog_title);
    let text_arg = format!("--text={}", t.close_dialog_body);
    let ok_arg = format!("--ok-label={}", t.close_dialog_confirm);
    let cancel_arg = format!("--cancel-label={}", t.close_dialog_cancel);

    // Use zenity for a blocking close-confirmation dialog.
    // GTK4 only provides async dialogs, which cannot block the winit event loop.
    // zenity is available on virtually all Linux desktops.
    match std::process::Command::new("zenity")
        .args([
            "--question",
            &title_arg,
            &text_arg,
            &ok_arg,
            &cancel_arg,
            "--icon-name=dialog-warning",
        ])
        .status()
    {
        Ok(status) => status.success(),
        Err(_) => {
            // zenity unavailable — try kdialog (KDE).
            match std::process::Command::new("kdialog")
                .args([
                    "--warningyesno",
                    t.close_dialog_body,
                    "--title",
                    t.close_dialog_title,
                    "--yes-label",
                    t.close_dialog_confirm,
                    "--no-label",
                    t.close_dialog_cancel,
                ])
                .status()
            {
                Ok(status) => status.success(),
                Err(_) => true, // No dialog tool found — allow close.
            }
        }
    }
}
