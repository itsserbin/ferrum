#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::CpuRenderer;
#[cfg(not(target_os = "macos"))]
use super::super::WindowButton;

impl CpuRenderer {
    /// Returns true when pointer is over the custom window minimize button.
    #[allow(dead_code)]
    pub fn is_window_minimize_button(&self, x: f64, y: f64, buf_width: usize) -> bool {
        #[cfg(not(target_os = "macos"))]
        {
            self.window_button_at_position(x, y, buf_width as u32) == Some(WindowButton::Minimize)
        }
        #[cfg(target_os = "macos")]
        {
            let _ = (x, y, buf_width);
            false
        }
    }

    /// Returns true when pointer is over the custom window maximize button.
    #[allow(dead_code)]
    pub fn is_window_maximize_button(&self, x: f64, y: f64, buf_width: usize) -> bool {
        #[cfg(not(target_os = "macos"))]
        {
            self.window_button_at_position(x, y, buf_width as u32) == Some(WindowButton::Maximize)
        }
        #[cfg(target_os = "macos")]
        {
            let _ = (x, y, buf_width);
            false
        }
    }

    /// Returns true when pointer is over the custom window close button.
    #[allow(dead_code)]
    pub fn is_window_close_button(&self, x: f64, y: f64, buf_width: usize) -> bool {
        #[cfg(not(target_os = "macos"))]
        {
            self.window_button_at_position(x, y, buf_width as u32) == Some(WindowButton::Close)
        }
        #[cfg(target_os = "macos")]
        {
            let _ = (x, y, buf_width);
            false
        }
    }
}
