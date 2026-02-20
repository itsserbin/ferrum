mod events;
mod input;
mod interaction;
mod lifecycle;
mod pane;
mod platform;
mod renderer;
mod state;
mod tabs;

use std::io::Write;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::mpsc;

use softbuffer::Context;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};
#[cfg(target_os = "windows")]
use winit::platform::windows::{CornerPreference, WindowAttributesExtWindows};
#[cfg(not(target_os = "macos"))]
use winit::window::WindowLevel;
use winit::window::{CursorIcon, ResizeDirection, Window, WindowId};

use crate::core::terminal::Terminal;
use crate::core::{MouseMode, Position, SecurityGuard, Selection};
#[cfg(not(target_os = "macos"))]
use crate::gui::renderer::TAB_BAR_HEIGHT;
use crate::gui::renderer::{
    ContextMenu, CpuRenderer, Renderer as _, RendererBackend, SecurityPopup, WINDOW_PADDING,
};
use crate::pty;
use crate::update;

/// Minimum number of columns for the terminal window.
const MIN_WINDOW_COLS: u32 = 40;
/// Minimum number of rows for the terminal window.
const MIN_WINDOW_ROWS: u32 = 10;

use self::state::{
    App, ClosedTabInfo, DragState, FerrumWindow, PtyEvent, RenameState, ScrollbarState,
    SelectionDragMode, TabReorderAnimation, TabState, WindowRequest,
};

impl FerrumWindow {
    /// Creates a new FerrumWindow wrapping an already-created winit window and renderer backend.
    fn new(window: Arc<Window>, context: &Context<winit::event_loop::OwnedDisplayHandle>) -> Self {
        let mut backend = RendererBackend::new(window.clone(), context);
        backend.set_scale(window.scale_factor());

        FerrumWindow {
            window,
            window_title: "Ferrum".to_string(),
            pending_grid_resize: None,
            backend,
            tabs: Vec::new(),
            active_tab: 0,
            modifiers: ModifiersState::empty(),
            is_selecting: false,
            mouse_pos: (0.0, 0.0),
            clipboard: arboard::Clipboard::new().ok(),
            last_click_time: std::time::Instant::now(),
            last_click_pos: Position { row: 0, col: 0 },
            click_streak: 0,
            selection_anchor: None,
            keyboard_selection_anchor: None,
            selection_drag_mode: SelectionDragMode::Character,
            hovered_tab: None,
            context_menu: None,
            security_popup: None,
            #[cfg(not(target_os = "macos"))]
            tab_hover_progress: Vec::new(),
            #[cfg(not(target_os = "macos"))]
            close_hover_progress: Vec::new(),
            #[cfg(not(target_os = "macos"))]
            ui_animation_last_tick: std::time::Instant::now(),
            closed_tabs: Vec::new(),
            renaming_tab: None,
            dragging_tab: None,
            tab_reorder_animation: None,
            last_tab_click: None,
            last_topbar_empty_click: None,
            resize_direction: None,
            cursor_blink_start: std::time::Instant::now(),
            suppress_click_to_cursor_once: false,
            #[cfg(target_os = "macos")]
            pending_native_tab_syncs: 0,
            #[cfg(target_os = "macos")]
            next_native_tab_sync_at: None,
            scroll_accumulator: 0.0,
            pending_requests: Vec::new(),
            pinned: false,
        }
    }

    /// Calculates terminal rows/cols with tab bar and outer padding applied.
    fn calc_grid_size(&self, width: u32, height: u32) -> (usize, usize) {
        let tab_bar_height = self.backend.tab_bar_height_px();
        let window_padding = self.backend.window_padding_px();
        let rows = height.saturating_sub(tab_bar_height + window_padding * 2) as usize
            / self.backend.cell_height() as usize;
        let cols =
            width.saturating_sub(window_padding * 2) as usize / self.backend.cell_width() as usize;
        (rows.max(1), cols.max(1))
    }

    /// Returns the active tab as mutable reference.
    fn active_tab_mut(&mut self) -> Option<&mut TabState> {
        self.tabs.get_mut(self.active_tab)
    }

    /// Returns the active tab as shared reference.
    fn active_tab_ref(&self) -> Option<&TabState> {
        self.tabs.get(self.active_tab)
    }

    /// Returns the focused pane leaf of the active tab (immutable).
    fn active_leaf_ref(&self) -> Option<&pane::PaneLeaf> {
        self.active_tab_ref().and_then(|t| t.focused_leaf())
    }

    /// Returns the focused pane leaf of the active tab (mutable).
    fn active_leaf_mut(&mut self) -> Option<&mut pane::PaneLeaf> {
        self.active_tab_mut().and_then(|t| t.focused_leaf_mut())
    }

    fn compose_window_title(&self, update: Option<&crate::update::AvailableRelease>) -> String {
        let base = self
            .active_tab_ref()
            .map(|tab| tab.title.as_str())
            .unwrap_or("Ferrum");
        match update {
            Some(release) => format!("{base} - Update {} available", release.tag_name),
            None => base.to_string(),
        }
    }

    pub(super) fn sync_window_title(&mut self, update: Option<&crate::update::AvailableRelease>) {
        let next_title = self.compose_window_title(update);
        if self.window_title != next_title {
            self.window.set_title(&next_title);
            self.window_title = next_title;
        }
    }

    /// Toggles the pinned (always-on-top) state of this window.
    pub(super) fn toggle_pin(&mut self) {
        #[cfg(target_os = "macos")]
        {
            let current = platform::macos::is_window_pinned(&self.window);
            let next = !current;
            platform::macos::set_native_tab_group_pin_state(&self.window, next);
            self.pinned = next;
        }

        #[cfg(not(target_os = "macos"))]
        {
            self.pinned = !self.pinned;
            let level = if self.pinned {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            };
            self.window.set_window_level(level);
        }
    }
}

impl App {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel::<PtyEvent>();
        let (update_tx, update_rx) = mpsc::channel::<update::AvailableRelease>();
        update::spawn_update_checker(update_tx);
        App {
            windows: std::collections::HashMap::new(),
            context: None,
            next_tab_id: 0,
            tx,
            rx,
            update_rx,
            available_release: None,
        }
    }

    /// Creates a new Ferrum window and registers it. Returns the WindowId.
    fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        position: Option<winit::dpi::PhysicalPosition<i32>>,
    ) -> Option<WindowId> {
        let context = self.context.as_ref()?;

        // Use default metrics for minimum window size calculation.
        let tmp = CpuRenderer::new();
        let cw = tmp.cell_width();
        let ch = tmp.cell_height();
        #[cfg(target_os = "macos")]
        let min_size = winit::dpi::LogicalSize::new(
            (cw * MIN_WINDOW_COLS + WINDOW_PADDING * 2) as f64,
            (ch * MIN_WINDOW_ROWS + WINDOW_PADDING * 2) as f64,
        );
        #[cfg(not(target_os = "macos"))]
        let min_size = winit::dpi::LogicalSize::new(
            (cw * MIN_WINDOW_COLS + WINDOW_PADDING * 2) as f64,
            (ch * MIN_WINDOW_ROWS + TAB_BAR_HEIGHT + WINDOW_PADDING * 2) as f64,
        );
        let mut attrs = Window::default_attributes()
            .with_title("Ferrum")
            .with_min_inner_size(min_size);

        // On macOS: use standard native decorations + native tab bar.
        // On other platforms: no decorations (custom tab bar handles chrome).
        #[cfg(not(target_os = "macos"))]
        {
            attrs = attrs.with_decorations(false);
        }
        #[cfg(target_os = "windows")]
        {
            attrs = attrs
                .with_corner_preference(CornerPreference::Round)
                .with_undecorated_shadow(true);
        }

        if let Some(pos) = position {
            attrs = attrs.with_position(pos);
        }

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(err) => {
                eprintln!("Failed to create window: {err}");
                return None;
            }
        };

        window.set_cursor(CursorIcon::Default);

        // Configure native macOS tab grouping.
        #[cfg(target_os = "macos")]
        platform::macos::configure_native_tabs(&window);

        // Set up macOS titlebar pin button.
        #[cfg(target_os = "macos")]
        platform::macos::setup_toolbar(&window);

        let id = window.id();
        let mut ferrum_win = FerrumWindow::new(window, context);
        ferrum_win.sync_window_title(self.available_release.as_ref());
        self.windows.insert(id, ferrum_win);
        Some(id)
    }

    /// Creates a new window with a single detached tab.
    /// Immediately starts OS-level window drag (mouse button still held).
    #[cfg(not(target_os = "macos"))]
    fn create_window_with_tab(
        &mut self,
        event_loop: &ActiveEventLoop,
        tab: Box<TabState>,
        position: Option<winit::dpi::PhysicalPosition<i32>>,
    ) {
        let Some(win_id) = self.create_window(event_loop, position) else {
            return;
        };
        if let Some(win) = self.windows.get_mut(&win_id) {
            win.tabs.push(*tab);
            win.active_tab = 0;
            win.refresh_tab_bar_visibility();
            // Mouse button is still held — initiate OS drag so the window follows cursor.
            let _ = win.window.drag_window();
            win.window.request_redraw();
        }
    }
}

pub fn run() {
    let event_loop = match EventLoop::new() {
        Ok(loop_) => loop_,
        Err(err) => {
            eprintln!("Failed to create event loop: {err}");
            return;
        }
    };
    let mut app = App::new();
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("Application error: {err}");
    }
}
