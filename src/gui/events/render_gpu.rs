#[cfg(feature = "gpu")]
use crate::gui::renderer::backend::RendererBackend;
#[cfg(feature = "gpu")]
use crate::gui::renderer::traits::Renderer;
#[cfg(feature = "gpu")]
use crate::gui::*;

#[cfg(feature = "gpu")]
use super::render_shared::{scrollbar_opacity, should_show_cursor};

#[cfg(feature = "gpu")]
impl FerrumWindow {
    /// GPU rendering path: issues draw commands through the GPU renderer
    /// and presents the frame via wgpu.
    pub(super) fn render_gpu_frame(&mut self, w: NonZeroU32, h: NonZeroU32, bw: usize, bh: usize) {
        // Build tab bar metadata (not needed on macOS -- native tab bar).
        #[cfg(not(target_os = "macos"))]
        let state = self.build_tab_bar_state(bw);
        #[cfg(not(target_os = "macos"))]
        let frame_tab_infos = state.render_tab_infos();

        let RendererBackend::Gpu(gpu) = &mut self.backend else {
            return;
        };

        gpu.resize(w.get(), h.get());

        // Dummy buffer -- GPU renderer ignores buffer params.
        let mut dummy = [];

        // 1) Render terminal grid.
        if let Some(tab) = self.tabs.get(self.active_tab) {
            let viewport_start = tab
                .terminal
                .scrollback
                .len()
                .saturating_sub(tab.scroll_offset);
            if tab.scroll_offset == 0 {
                gpu.render(
                    &mut dummy,
                    bw,
                    bh,
                    &tab.terminal.grid,
                    tab.selection.as_ref(),
                    viewport_start,
                );
            } else {
                let display = tab.terminal.build_display(tab.scroll_offset);
                gpu.render(
                    &mut dummy,
                    bw,
                    bh,
                    &display,
                    tab.selection.as_ref(),
                    viewport_start,
                );
            }

            // 2) Draw cursor.
            if tab.scroll_offset == 0
                && tab.terminal.cursor_visible
                && should_show_cursor(self.cursor_blink_start, tab.terminal.cursor_style)
            {
                gpu.draw_cursor(
                    &mut dummy,
                    bw,
                    bh,
                    tab.terminal.cursor_row,
                    tab.terminal.cursor_col,
                    &tab.terminal.grid,
                    tab.terminal.cursor_style,
                );
            }
        }

        // 3) Draw scrollbar overlay.
        if let Some(tab) = self.tabs.get(self.active_tab) {
            let scrollback_len = tab.terminal.scrollback.len();
            if scrollback_len > 0 {
                let hover = tab.scrollbar.hover || tab.scrollbar.dragging;
                let opacity = scrollbar_opacity(
                    tab.scrollbar.hover,
                    tab.scrollbar.dragging,
                    tab.scrollbar.last_activity,
                );

                if opacity > 0.0 {
                    gpu.render_scrollbar(
                        &mut dummy,
                        bw,
                        bh,
                        tab.scroll_offset,
                        scrollback_len,
                        tab.terminal.grid.rows,
                        opacity,
                        hover,
                    );
                }
            }
        }

        // 4) Draw tab bar (not on macOS -- native tab bar).
        #[cfg(not(target_os = "macos"))]
        {
            if state.tab_bar_visible {
                gpu.draw_tab_bar(
                    &mut dummy,
                    bw,
                    bh,
                    &frame_tab_infos,
                    self.hovered_tab,
                    self.mouse_pos,
                    state.tab_offsets.as_deref(),
                    self.pinned,
                );

                // 5) Draw drag overlay.
                if let Some((source_index, current_x, indicator_x)) = state.drag_info {
                    gpu.draw_tab_drag_overlay(
                        &mut dummy,
                        bw,
                        bh,
                        &frame_tab_infos,
                        source_index,
                        current_x,
                        indicator_x,
                    );
                }
            }
        }

        // 6) Draw popups/menus.
        if let Some(ref popup) = self.security_popup {
            gpu.draw_security_popup(&mut dummy, bw, bh, popup);
        }

        if let Some(ref menu) = self.context_menu {
            gpu.draw_context_menu(&mut dummy, bw, bh, menu);
        }

        #[cfg(not(target_os = "macos"))]
        if state.show_tooltip
            && state.tab_bar_visible
            && let Some(ref title) = state.tab_tooltip
        {
            gpu.draw_tab_tooltip(&mut dummy, bw, bh, self.mouse_pos, title);
        }

        // 7) Present the frame via wgpu.
        gpu.present_frame();
    }
}
