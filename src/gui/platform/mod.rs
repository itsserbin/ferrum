#[cfg(target_os = "macos")]
pub mod macos;

mod close_dialog;

pub use close_dialog::confirm_window_close;
