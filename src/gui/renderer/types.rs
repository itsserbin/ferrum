/// Tab context menu actions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextAction {
    Close,
    Rename,
    Duplicate,
}

/// Context menu state for one tab.
pub struct ContextMenu {
    pub x: u32,
    pub y: u32,
    pub tab_index: usize,
    pub items: Vec<(ContextAction, &'static str)>,
    pub hover_index: Option<usize>,
}

/// Render-time tab metadata.
pub struct TabInfo<'a> {
    pub title: &'a str,
    pub is_active: bool,
    pub security_count: usize,
    pub is_renaming: bool,
    pub rename_text: Option<&'a str>,
    pub rename_cursor: usize,
    pub rename_selection: Option<(usize, usize)>, // Byte range within rename_text.
}

pub struct SecurityPopup {
    pub tab_index: usize,
    pub x: u32,
    pub y: u32,
    pub title: &'static str,
    pub lines: Vec<String>,
}

/// Result of tab-bar hit testing.
#[derive(Debug)]
pub enum TabBarHit {
    /// Clicked on a tab by index.
    Tab(usize),
    /// Clicked on a tab close button by index.
    CloseTab(usize),
    /// Clicked on the new-tab button.
    NewTab,
    /// Clicked on a window control button (non-macOS).
    #[cfg(not(target_os = "macos"))]
    WindowButton(WindowButton),
    /// Clicked empty bar area (window drag).
    Empty,
}

/// Window control button type (non-macOS).
#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowButton {
    Minimize,
    Maximize,
    Close,
}

pub(super) struct GlyphBitmap {
    pub(super) data: Vec<u8>,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) left: i32,
    pub(super) top: i32,
}
