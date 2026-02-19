/// Tab context menu actions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextAction {
    CloseTab,
    RenameTab,
    DuplicateTab,
    CopySelection,
    Paste,
    ClearSelection,
}

/// Context menu origin/target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextMenuTarget {
    Tab { tab_index: usize },
    TerminalSelection,
}

/// Context menu state.
pub struct ContextMenu {
    pub x: u32,
    pub y: u32,
    pub target: ContextMenuTarget,
    pub items: Vec<(ContextAction, &'static str)>,
    pub hover_index: Option<usize>,
    pub hover_progress: Vec<f32>,
    pub opened_at: std::time::Instant,
}

impl ContextMenu {
    /// Pure hit-test: returns the hovered item index given pointer coordinates
    /// and the cell dimensions used to compute menu geometry.
    pub fn hit_test(&self, x: f64, y: f64, cell_width: u32, cell_height: u32) -> Option<usize> {
        let mw = self.width(cell_width);
        let ih = self.item_height(cell_height);
        let mh = self.height(cell_height);

        if x < self.x as f64
            || x >= (self.x + mw) as f64
            || y < self.y as f64
            || y >= (self.y + mh) as f64
        {
            return None;
        }

        let rel_y = (y - self.y as f64 - 2.0) as u32;
        let idx = rel_y / ih;
        if (idx as usize) < self.items.len() {
            Some(idx as usize)
        } else {
            None
        }
    }
}

/// Render-time tab metadata.
pub struct TabInfo<'a> {
    pub title: &'a str,
    pub is_active: bool,
    pub security_count: usize,
    pub hover_progress: f32,
    pub close_hover_progress: f32,
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
    /// Clicked on the pin button (non-macOS).
    #[cfg(not(target_os = "macos"))]
    PinButton,
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
