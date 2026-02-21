#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

mod close_dialog;

pub use close_dialog::confirm_window_close;

/// Returns whether the native settings window is currently open.
/// Delegates to the platform-specific implementation.
#[cfg(not(target_os = "macos"))]
pub(crate) fn is_settings_window_open() -> bool {
    #[cfg(target_os = "windows")]
    {
        return windows::settings_window::is_settings_window_open();
    }
    #[cfg(target_os = "linux")]
    {
        return linux::settings_window::is_settings_window_open();
    }
    #[allow(unreachable_code)]
    false
}
