#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

mod close_dialog;

pub use close_dialog::confirm_window_close;

/// Expands to the `request_reopen` function for platform settings windows.
///
/// Both Linux and Windows settings windows contain an identical three-line body
/// that operates on their respective module-local statics and `close_settings_window`.
/// This macro deduplicates the source while keeping each platform's items independent.
#[cfg(not(target_os = "macos"))]
macro_rules! impl_settings_request_reopen {
    () => {
        /// Closes the settings window and reopens it at the given tab with fresh translations.
        pub fn request_reopen(
            config: &$crate::config::AppConfig,
            tx: ::std::sync::mpsc::Sender<$crate::config::AppConfig>,
            tab_index: usize,
        ) {
            *REOPEN_DATA.lock().unwrap() = Some((config.clone(), tx));
            REOPEN_WITH_TAB.store(tab_index as isize, ::std::sync::atomic::Ordering::Relaxed);
            close_settings_window();
        }
    };
}
#[cfg(not(target_os = "macos"))]
pub(crate) use impl_settings_request_reopen;

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
