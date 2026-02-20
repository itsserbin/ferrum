use std::collections::HashMap;
use std::time::Instant;

use muda::MenuId;

use crate::gui::*;

/// PTY event tagged with the source tab id and pane id.
pub(super) enum PtyEvent {
    Data {
        tab_id: u64,
        pane_id: u64,
        bytes: Vec<u8>,
    },
    Exited {
        tab_id: u64,
        pane_id: u64,
    },
}

/// Metadata for recently closed tabs (Ctrl+Shift+T restore).
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) struct ClosedTabInfo {
    pub(super) title: String,
}

/// Per-tab scrollbar visual state.
pub(super) struct ScrollbarState {
    pub(super) last_activity: Instant,
    pub(super) hover: bool,
    pub(super) dragging: bool,
    pub(super) drag_start_y: f64,
    pub(super) drag_start_offset: usize,
}

impl ScrollbarState {
    pub(super) fn new() -> Self {
        // Start far in the past so the scrollbar is invisible on launch.
        Self {
            last_activity: Instant::now() - std::time::Duration::from_secs(10),
            hover: false,
            dragging: false,
            drag_start_y: 0.0,
            drag_start_offset: 0,
        }
    }
}

/// Runtime state for a single terminal tab.
///
/// Each tab holds a pane tree (binary tree of terminal panes).
/// For single-pane tabs, the tree is a single `PaneNode::Leaf`.
pub(super) struct TabState {
    pub(super) id: u64,
    pub(super) title: String,
    pub(super) pane_tree: crate::gui::pane::PaneNode,
    pub(super) focused_pane: crate::gui::pane::PaneId,
    pub(super) next_pane_id: crate::gui::pane::PaneId,
}

impl TabState {
    /// Returns the focused pane leaf (immutable).
    pub(super) fn focused_leaf(&self) -> Option<&crate::gui::pane::PaneLeaf> {
        self.pane_tree.find_leaf(self.focused_pane)
    }

    /// Returns the focused pane leaf (mutable).
    pub(super) fn focused_leaf_mut(&mut self) -> Option<&mut crate::gui::pane::PaneLeaf> {
        self.pane_tree.find_leaf_mut(self.focused_pane)
    }

    /// Returns `true` if this tab contains more than one pane.
    pub(super) fn has_multiple_panes(&self) -> bool {
        !self.pane_tree.is_leaf()
    }

    /// Picks the pane that should receive focus after `closing_id` is removed.
    ///
    /// Preference order:
    /// 1) Most recently created pane with id `< closing_id` (reverse create order)
    /// 2) Otherwise, the most recently created remaining pane.
    pub(super) fn focus_after_closing_pane(
        &self,
        closing_id: crate::gui::pane::PaneId,
    ) -> Option<crate::gui::pane::PaneId> {
        let mut previous_created: Option<crate::gui::pane::PaneId> = None;
        let mut newest_remaining: Option<crate::gui::pane::PaneId> = None;

        for pane_id in self.pane_tree.leaf_ids() {
            newest_remaining = Some(newest_remaining.map_or(pane_id, |v| v.max(pane_id)));
            if pane_id < closing_id {
                previous_created = Some(previous_created.map_or(pane_id, |v| v.max(pane_id)));
            }
        }

        previous_created.or(newest_remaining)
    }
}

/// Drag state for divider resize between panes.
pub(super) struct DividerDragState {
    /// Last pointer position used to identify the dragged divider.
    pub(super) initial_mouse_pos: (u32, u32),
    /// Direction of the divider being dragged.
    pub(super) direction: crate::gui::pane::SplitDirection,
}

/// Drag-and-drop state for tab reordering.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) struct DragState {
    pub(super) source_index: usize, // Which tab is being dragged.
    pub(super) start_x: f64,        // Mouse x at drag start.
    pub(super) start_y: f64,        // Mouse y at drag start.
    pub(super) current_x: f64,      // Current mouse x.
    pub(super) current_y: f64,      // Current mouse y.
    pub(super) is_active: bool,     // True once threshold exceeded.
    pub(super) indicator_x: f32,    // Smoothly interpolated insertion indicator x.
}

/// Post-reorder slide animation for tabs.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) struct TabReorderAnimation {
    pub(super) started: Instant,
    pub(super) duration_ms: u32,
    /// Per-tab pixel offsets at animation start (shrink toward 0 over duration).
    pub(super) offsets: Vec<f32>,
}

/// Temporary inline rename state for the tab bar.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) struct RenameState {
    pub(super) tab_index: usize,
    pub(super) text: String,
    pub(super) original_title: String, // Title before rename started, for Escape revert.
    pub(super) cursor: usize,          // Byte index at a valid UTF-8 char boundary.
    pub(super) selection_anchor: Option<usize>, // Byte index for selection anchor.
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SelectionDragMode {
    Character,
    Word,
    Line,
}

/// Request from a FerrumWindow to the App (window manager).
pub(super) enum WindowRequest {
    /// Detach a tab into a new window at the given screen position.
    #[cfg(not(target_os = "macos"))]
    DetachTab {
        tab: Box<TabState>,
        cursor_pos: Option<winit::dpi::PhysicalPosition<i32>>,
    },
    /// Close this window (all tabs gone or user closed).
    CloseWindow,
    /// Create a new standalone window (Ctrl/Cmd+N).
    NewWindow { cwd: Option<String> },
    /// Create a new native macOS tab (new window in tab group).
    #[cfg(target_os = "macos")]
    NewTab { cwd: Option<String> },
    /// Reopen a recently closed tab as a native macOS tab.
    #[cfg(target_os = "macos")]
    ReopenTab { title: String },
}

/// Tracks which context menu is currently open and what actions it maps to.
pub(super) enum MenuContext {
    Tab {
        tab_index: usize,
        action_map: Vec<(MenuId, crate::gui::menus::MenuAction)>,
    },
    Terminal {
        pane_id: Option<crate::gui::pane::PaneId>,
        action_map: Vec<(MenuId, crate::gui::menus::MenuAction)>,
    },
}

/// Per-window state. Each window is self-contained with its own tabs, renderer, surface.
pub(super) struct FerrumWindow {
    pub(super) window: Arc<Window>,
    pub(super) window_title: String,
    pub(super) pending_grid_resize: Option<(usize, usize)>,
    pub(super) backend: renderer::RendererBackend,
    pub(super) tabs: Vec<TabState>,
    pub(super) active_tab: usize,
    pub(super) modifiers: ModifiersState,
    pub(super) is_selecting: bool,
    pub(super) mouse_pos: (f64, f64),
    pub(super) clipboard: Option<arboard::Clipboard>,
    pub(super) last_click_time: std::time::Instant,
    pub(super) last_click_pos: Position,
    pub(super) click_streak: u8,
    pub(super) selection_anchor: Option<crate::core::SelectionPoint>,
    pub(super) keyboard_selection_anchor: Option<crate::core::SelectionPoint>,
    pub(super) selection_drag_mode: SelectionDragMode,
    pub(super) hovered_tab: Option<usize>,
    pub(super) pending_menu_context: Option<MenuContext>,
    pub(super) security_popup: Option<SecurityPopup>,
    #[cfg(not(target_os = "macos"))]
    pub(super) tab_hover_progress: Vec<f32>,
    #[cfg(not(target_os = "macos"))]
    pub(super) close_hover_progress: Vec<f32>,
    #[cfg(not(target_os = "macos"))]
    pub(super) ui_animation_last_tick: std::time::Instant,
    pub(super) closed_tabs: Vec<ClosedTabInfo>,
    pub(super) renaming_tab: Option<RenameState>,
    pub(super) dragging_tab: Option<DragState>,
    pub(super) tab_reorder_animation: Option<TabReorderAnimation>,
    pub(super) last_tab_click: Option<(usize, std::time::Instant)>,
    pub(super) last_topbar_empty_click: Option<std::time::Instant>,
    pub(super) resize_direction: Option<ResizeDirection>,
    pub(super) cursor_blink_start: std::time::Instant,
    pub(super) suppress_click_to_cursor_once: bool,
    #[cfg(target_os = "macos")]
    pub(super) pending_native_tab_syncs: u8,
    #[cfg(target_os = "macos")]
    pub(super) next_native_tab_sync_at: Option<std::time::Instant>,
    /// Accumulates fractional pixel scroll for trackpad (PixelDelta).
    pub(super) scroll_accumulator: f64,
    /// Pending requests from this window to the App (detach, close, etc.).
    pub(super) pending_requests: Vec<WindowRequest>,
    /// Whether this window is pinned (always-on-top).
    pub(super) pinned: bool,
    /// Active divider drag state (pane resize).
    pub(super) divider_drag: Option<DividerDragState>,
}

/// App is now a window manager holding multiple FerrumWindows.
pub(super) struct App {
    pub(super) windows: HashMap<WindowId, FerrumWindow>,
    pub(super) context: Option<Context<winit::event_loop::OwnedDisplayHandle>>,
    pub(super) next_tab_id: u64,
    pub(super) tx: mpsc::Sender<PtyEvent>,
    pub(super) rx: mpsc::Receiver<PtyEvent>,
    pub(super) update_rx: mpsc::Receiver<crate::update::AvailableRelease>,
    pub(super) available_release: Option<crate::update::AvailableRelease>,
}
