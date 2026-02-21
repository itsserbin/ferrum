#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(not(target_os = "macos"))]
pub(crate) mod settings_window;

mod close_dialog;

pub use close_dialog::confirm_window_close;

/// Returns whether the native settings window is currently open.
/// Delegates to the platform-specific implementation.
#[cfg(not(target_os = "macos"))]
pub(crate) fn is_settings_window_open() -> bool {
    settings_window::is_settings_window_open()
}
