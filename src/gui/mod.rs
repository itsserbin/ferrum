mod events;
mod input;
mod interaction;
mod lifecycle;
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
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{CursorIcon, ResizeDirection, Window, WindowId};

use crate::core::terminal::Terminal;
use crate::core::{MouseMode, Position, SecurityGuard, Selection};
use crate::gui::renderer::{ContextMenu, CpuRenderer, RendererBackend, SecurityPopup, WINDOW_PADDING};
#[cfg(not(target_os = "macos"))]
use crate::gui::renderer::TAB_BAR_HEIGHT;
use crate::pty;

use self::state::{
    App, ClosedTabInfo, DragState, FerrumWindow, PtyEvent, RenameState, ScrollbarState,
    SelectionDragMode, TabReorderAnimation, TabState, WindowRequest,
};

impl FerrumWindow {
    /// Creates a new FerrumWindow wrapping an already-created winit window and renderer backend.
    fn new(
        window: Arc<Window>,
        context: &Context<winit::event_loop::OwnedDisplayHandle>,
    ) -> Self {
        let mut backend = RendererBackend::new(window.clone(), context);
        backend.set_scale(window.scale_factor());

        #[cfg(target_os = "macos")]
        let _window_controller = platform::macos::create_window_controller(&window);

        FerrumWindow {
            window,
            #[cfg(target_os = "macos")]
            _window_controller,
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
            selection_drag_mode: SelectionDragMode::Character,
            hovered_tab: None,
            context_menu: None,
            security_popup: None,
            closed_tabs: Vec::new(),
            renaming_tab: None,
            dragging_tab: None,
            tab_reorder_animation: None,
            last_tab_click: None,
            last_topbar_empty_click: None,
            resize_direction: None,
            cursor_blink_start: std::time::Instant::now(),
            suppress_click_to_cursor_once: false,
            scroll_accumulator: 0.0,
            pending_requests: Vec::new(),
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
}

impl App {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel::<PtyEvent>();
        App {
            windows: std::collections::HashMap::new(),
            context: None,
            next_tab_id: 0,
            tx,
            rx,
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
        #[cfg(target_os = "macos")]
        let min_size = winit::dpi::LogicalSize::new(
            (tmp.cell_width * 40 + WINDOW_PADDING * 2) as f64,
            (tmp.cell_height * 10 + WINDOW_PADDING * 2) as f64,
        );
        #[cfg(not(target_os = "macos"))]
        let min_size = winit::dpi::LogicalSize::new(
            (tmp.cell_width * 40 + WINDOW_PADDING * 2) as f64,
            (tmp.cell_height * 10 + TAB_BAR_HEIGHT + WINDOW_PADDING * 2) as f64,
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

        let id = window.id();
        let ferrum_win = FerrumWindow::new(window, context);
        self.windows.insert(id, ferrum_win);
        Some(id)
    }

    /// Creates a new window with a single detached tab.
    /// Immediately starts OS-level window drag (mouse button still held).
    #[cfg(not(target_os = "macos"))]
    fn create_window_with_tab(
        &mut self,
        event_loop: &ActiveEventLoop,
        tab: TabState,
        position: Option<winit::dpi::PhysicalPosition<i32>>,
    ) {
        let Some(win_id) = self.create_window(event_loop, position) else {
            return;
        };
        if let Some(win) = self.windows.get_mut(&win_id) {
            win.tabs.push(tab);
            win.active_tab = 0;
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
