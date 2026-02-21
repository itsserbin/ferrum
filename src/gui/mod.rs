mod events;
mod input;
mod interaction;
mod lifecycle;
mod menus;
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
use crate::gui::renderer::{CpuRenderer, Renderer as _, RendererBackend, SecurityPopup};
use crate::pty;
use crate::update;

/// Minimum number of columns for the terminal window.
const MIN_WINDOW_COLS: u32 = 40;
/// Minimum number of rows for the terminal window.
const MIN_WINDOW_ROWS: u32 = 10;

use self::state::{
    App, ClosedTabInfo, DividerDragState, DragState, FerrumWindow, MenuContext, PtyEvent,
    RenameState, ScrollbarState, SelectionDragMode, TabReorderAnimation, TabState, WindowRequest,
};

impl FerrumWindow {
    /// Creates a new FerrumWindow wrapping an already-created winit window and renderer backend.
    fn new(
        window: Arc<Window>,
        context: &Context<winit::event_loop::OwnedDisplayHandle>,
        proxy: &winit::event_loop::EventLoopProxy<()>,
        config: &crate::config::AppConfig,
    ) -> Self {
        let mut backend = RendererBackend::new(window.clone(), context, config);
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
            pending_menu_context: None,
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
            divider_drag: None,
            last_cwd_poll: std::time::Instant::now(),
            cursor_blink_interval_ms: config.terminal.cursor_blink_interval_ms,
            settings_tx: std::sync::mpsc::channel().0,
            event_proxy: proxy.clone(),
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

    /// Returns the terminal content rectangle (area below tab bar, inside padding).
    fn terminal_content_rect(&self) -> pane::PaneRect {
        let size = self.window.inner_size();
        let tab_bar_h = self.backend.tab_bar_height_px();
        let padding = self.backend.window_padding_px();
        pane::PaneRect {
            x: padding,
            y: tab_bar_h + padding,
            width: size.width.saturating_sub(padding * 2),
            height: size.height.saturating_sub(tab_bar_h + padding * 2),
        }
    }

    fn compose_window_title(&self, update: Option<&crate::update::AvailableRelease>) -> String {
        let base = self
            .active_tab_ref()
            .map(|tab| {
                if tab.is_renamed {
                    tab.title.clone()
                } else {
                    tab.focused_leaf()
                        .and_then(|leaf| leaf.cwd())
                        .map(|cwd| renderer::shared::path_display::replace_home_prefix(&cwd))
                        .unwrap_or_else(|| tab.title.clone())
                }
            })
            .unwrap_or_else(|| "Ferrum".to_string());
        match update {
            Some(release) => format!("{base} - Update {} available", release.tag_name),
            None => base,
        }
    }

    pub(super) fn sync_window_title(&mut self, update: Option<&crate::update::AvailableRelease>) {
        let next_title = self.compose_window_title(update);
        if self.window_title != next_title {
            self.window.set_title(&next_title);
            self.window_title = next_title;
        }
    }

    /// Polls CWD via OS API for panes that haven't received OSC 7,
    /// and updates `terminal.cwd` so the tab title auto-updates.
    pub(super) fn poll_cwd_for_tabs(&mut self) {
        for tab in &mut self.tabs {
            if tab.is_renamed {
                continue;
            }
            let focused = tab.focused_pane;
            let leaf = match tab.pane_tree.find_leaf_mut(focused) {
                Some(l) => l,
                None => continue,
            };
            // Skip if we already have CWD from OSC 7
            if leaf.terminal.cwd.is_some() {
                continue;
            }
            let pid = match leaf.session.as_ref().and_then(|s| s.process_id()) {
                Some(p) => p,
                None => continue,
            };
            if let Some(cwd) = crate::pty::cwd::get_process_cwd(pid) {
                leaf.terminal.cwd = Some(cwd);
            }
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
    fn new(proxy: winit::event_loop::EventLoopProxy<()>) -> Self {
        let (tx, rx) = mpsc::channel::<PtyEvent>();
        let (update_tx, update_rx) = mpsc::channel::<update::AvailableRelease>();
        update::spawn_update_checker(update_tx);
        let config = crate::config::load_config();
        let (settings_tx, settings_rx) = std::sync::mpsc::channel();
        App {
            windows: std::collections::HashMap::new(),
            context: None,
            next_tab_id: 0,
            tx,
            rx,
            proxy,
            update_rx,
            available_release: None,
            config,
            settings_tx,
            settings_rx,
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
        let tmp = CpuRenderer::new(&self.config);
        let cw = tmp.cell_width();
        let ch = tmp.cell_height();
        let wp = self.config.layout.window_padding;
        #[cfg(target_os = "macos")]
        let min_size = winit::dpi::LogicalSize::new(
            (cw * MIN_WINDOW_COLS + wp * 2) as f64,
            (ch * MIN_WINDOW_ROWS + wp * 2) as f64,
        );
        #[cfg(not(target_os = "macos"))]
        let min_size = winit::dpi::LogicalSize::new(
            (cw * MIN_WINDOW_COLS + wp * 2) as f64,
            (ch * MIN_WINDOW_ROWS + self.config.layout.tab_bar_height + wp * 2) as f64,
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
        let mut ferrum_win = FerrumWindow::new(window, context, &self.proxy, &self.config);
        ferrum_win.settings_tx = self.settings_tx.clone();
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
    let event_loop = match EventLoop::<()>::with_user_event().build() {
        Ok(loop_) => loop_,
        Err(err) => {
            eprintln!("Failed to create event loop: {err}");
            return;
        }
    };
    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy);
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("Application error: {err}");
    }
}
