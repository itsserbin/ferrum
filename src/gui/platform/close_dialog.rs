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
    use objc2_foundation::ns_string;

    let _ = window;

    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("[ferrum] Close confirmation dialog must run on the main thread");
        return false;
    };

    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(NSAlertStyle::Warning);
    alert.setMessageText(ns_string!("Close Ferrum?"));
    alert.setInformativeText(ns_string!(
        "Closing this terminal window will stop all running processes in its tabs."
    ));
    alert.addButtonWithTitle(ns_string!("Close"));
    alert.addButtonWithTitle(ns_string!("Cancel"));
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

    let caption = to_wide("Close Ferrum");
    let text = to_wide(
        "Closing this terminal window will stop all running processes in its tabs.\n\nClose Ferrum?",
    );

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
    use gtk::prelude::*;
    use gtk::{ButtonsType, DialogFlags, MessageDialog, MessageType, ResponseType};

    let _ = window;

    if !gtk::is_initialized_main_thread() && gtk::init().is_err() {
        eprintln!("[ferrum] Failed to initialize GTK for close confirmation dialog");
        return false;
    }

    let dialog = MessageDialog::new(
        None::<&gtk::Window>,
        DialogFlags::MODAL,
        MessageType::Warning,
        ButtonsType::OkCancel,
        "Close Ferrum?",
    );
    dialog.set_title("Close Ferrum");
    dialog.set_secondary_text(Some(
        "Closing this terminal window will stop all running processes in its tabs.",
    ));
    dialog.set_default_response(ResponseType::Cancel);
    let response = dialog.run();
    dialog.close();
    response == ResponseType::Ok
}
