#![cfg_attr(not(feature = "gpu"), allow(irrefutable_let_patterns))]

use crate::core::Color;
use crate::gui::renderer::backend::RendererBackend;
use crate::gui::*;

use super::render_shared::{scrollbar_opacity, should_show_cursor};

impl FerrumWindow {
    /// CPU rendering path: draws terminal, cursor, scrollbar, tab bar, popups
    /// into the softbuffer surface and presents the frame.
    pub(super) fn render_cpu_frame(&mut self, w: NonZeroU32, h: NonZeroU32, bw: usize, bh: usize) {
        // Build tab bar metadata (not needed on macOS -- native tab bar).
        #[cfg(not(target_os = "macos"))]
        let state = self.build_tab_bar_state(bw);
        #[cfg(not(target_os = "macos"))]
        let frame_tab_infos = state.render_tab_infos();

        let RendererBackend::Cpu { renderer, surface } = &mut self.backend else {
            return;
        };

        // Ensure surface size matches window.
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };

        // 1) Clear the full frame.
        buffer.fill(Color::DEFAULT_BG.to_pixel());

        // 2) Draw active tab terminal content.
        if let Some(tab) = self.tabs.get(self.active_tab) {
            let viewport_start = tab
                .terminal
                .scrollback
                .len()
                .saturating_sub(tab.scroll_offset);
            if tab.scroll_offset == 0 {
                renderer.render(
                    &mut buffer,
                    bw,
                    bh,
                    &tab.terminal.grid,
                    tab.selection.as_ref(),
                    viewport_start,
                );
            } else {
                let display = tab.terminal.build_display(tab.scroll_offset);
                renderer.render(
                    &mut buffer,
                    bw,
                    bh,
                    &display,
                    tab.selection.as_ref(),
                    viewport_start,
                );
            }

            // 3) Draw cursor on top of terminal cells.
            if tab.scroll_offset == 0
                && tab.terminal.cursor_visible
                && should_show_cursor(self.cursor_blink_start, tab.terminal.cursor_style)
            {
                renderer.draw_cursor(
                    &mut buffer,
                    bw,
                    bh,
                    tab.terminal.cursor_row,
                    tab.terminal.cursor_col,
                    &tab.terminal.grid,
                    tab.terminal.cursor_style,
                );
            }
        }

        // 4) Draw scrollbar overlay.
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
                    renderer.render_scrollbar(
                        &mut buffer,
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

        // 5) Draw tab bar (not on macOS -- native tab bar).
        #[cfg(not(target_os = "macos"))]
        {
            if state.tab_bar_visible {
                renderer.draw_tab_bar(
                    &mut buffer,
                    bw,
                    bh,
                    &frame_tab_infos,
                    self.hovered_tab,
                    self.mouse_pos,
                    state.tab_offsets.as_deref(),
                    self.pinned,
                );

                // 6) Draw drag overlay.
                if let Some((source_index, current_x, indicator_x)) = state.drag_info {
                    renderer.draw_tab_drag_overlay(
                        &mut buffer,
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

        // 7) Draw popups/menus.
        if let Some(ref popup) = self.security_popup {
            renderer.draw_security_popup(&mut buffer, bw, bh, popup);
        }

        if let Some(ref menu) = self.context_menu {
            renderer.draw_context_menu(&mut buffer, bw, bh, menu);
        }

        #[cfg(not(target_os = "macos"))]
        if state.show_tooltip
            && state.tab_bar_visible
            && let Some(ref title) = state.tab_tooltip
        {
            renderer.draw_tab_tooltip(&mut buffer, bw, bh, self.mouse_pos, title);
        }

        let _ = buffer.present();
    }
}
