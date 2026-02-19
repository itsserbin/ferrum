pub mod backend;
mod context_menu;
mod cpu;
mod cursor;
#[cfg(feature = "gpu")]
pub mod gpu;
#[cfg(feature = "gpu")]
pub mod metrics;
mod scrollbar;
mod security;
pub mod shared;
mod tab_bar;
mod terminal;
pub mod traits;
pub mod types;

use crate::core::{Color, CursorStyle, Grid, Selection};

pub use backend::RendererBackend;
pub use cpu::CpuRenderer;
pub use traits::Renderer;

pub(super) const FONT_SIZE: f32 = 15.0;
pub(super) const LINE_PADDING: u32 = 2;

/// Scrollbar thumb width in pixels.
pub const SCROLLBAR_WIDTH: u32 = 6;

/// Scrollbar hit zone width from right edge (wider than thumb for easier targeting).
pub const SCROLLBAR_HIT_ZONE: u32 = 14;

/// Margin between the thumb right edge and the window right edge.
pub const SCROLLBAR_MARGIN: u32 = 2;

/// Scrollbar thumb color — Catppuccin Mocha Overlay0 #6C7086.
pub(super) const SCROLLBAR_COLOR: Color = Color {
    r: 108,
    g: 112,
    b: 134,
};

/// Scrollbar thumb color when hovered/dragged — Catppuccin Mocha Overlay1 #7F849C.
pub(super) const SCROLLBAR_HOVER_COLOR: Color = Color {
    r: 127,
    g: 132,
    b: 156,
};

/// Tab bar height in pixels.
#[cfg(not(target_os = "macos"))]
pub const TAB_BAR_HEIGHT: u32 = 36;

/// Outer terminal padding inside the window.
#[cfg(target_os = "windows")]
pub const WINDOW_PADDING: u32 = 12;
/// Outer terminal padding inside the window.
#[cfg(not(target_os = "windows"))]
pub const WINDOW_PADDING: u32 = 8;

/// Active-tab accent (Catppuccin Mocha Lavender #B4BEFE) — used by rename selection.
pub(super) const ACTIVE_ACCENT: Color = Color {
    r: 180,
    g: 190,
    b: 254,
};

/// Security indicator color (Catppuccin Mocha Yellow #F9E2AF).
pub(super) const SECURITY_ACCENT: Color = Color {
    r: 249,
    g: 226,
    b: 175,
};

pub use types::*;
