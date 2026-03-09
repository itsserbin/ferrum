//! Shared layout calculations used by both CPU and GPU renderers.
//!
//! This module contains pure, side-effect-free math functions that compute
//! positions, sizes, and hit-test geometry for the tab bar.  By centralizing
//! these calculations we eliminate duplication between renderer backends.

pub mod banner_layout;
pub mod overlay_layout;
pub mod path_display;
pub mod scrollbar_math;
#[cfg(not(target_os = "macos"))]
pub mod tab_hit_test;
pub mod tab_math;
pub mod ui_layout;

/// Returns `(text_x, text_y)` for horizontally and vertically centering `text`
/// inside a button rectangle described by `(btn_x, btn_y, btn_w, btn_h)`.
///
/// Both the CPU renderer (which draws char-by-char) and the GPU renderer
/// (which calls `push_text`) use the same centering formula, so it lives here.
pub fn centered_button_text_origin(
    btn_x: u32,
    btn_y: u32,
    btn_w: u32,
    btn_h: u32,
    text: &str,
    cell_width: u32,
    cell_height: u32,
) -> (u32, u32) {
    let text_w = text.chars().count() as u32 * cell_width;
    let text_x = btn_x + (btn_w.saturating_sub(text_w)) / 2;
    let text_y = btn_y + (btn_h.saturating_sub(cell_height)) / 2;
    (text_x, text_y)
}
