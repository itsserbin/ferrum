use crate::core::Color;
use crate::gui::renderer::TabInfo;
use crate::gui::*;

impl FerrumWindow {
    pub(in crate::gui) fn apply_pending_resize(&mut self) {
        if let Some((rows, cols)) = self.pending_grid_resize.take() {
            self.resize_all_tabs(rows, cols);
        }
    }

    pub(crate) fn on_resized(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let (rows, cols) = self.calc_grid_size(size.width, size.height);
        // Coalesce rapid OS resize events and apply only the latest grid size on redraw.
        self.pending_grid_resize = Some((rows, cols));
        self.window.request_redraw();
    }

    pub(crate) fn on_redraw_requested(&mut self) {
        self.apply_pending_resize();

        let size = self.window.inner_size();

        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        let size_key = (w.get(), h.get());
        if self.last_surface_size != Some(size_key) {
            if self.surface.resize(w, h).is_err() {
                return;
            }
            self.last_surface_size = Some(size_key);
        }
        if let Ok(mut buffer) = self.surface.buffer_mut() {
            let bw = w.get() as usize;
            let bh = h.get() as usize;

            // 1) Clear the full frame.
            buffer.fill(Color::DEFAULT_BG.to_pixel());

            // 2) Draw tab bar first.
            let renaming = self.renaming_tab.as_ref().map(|rename| {
                let selection = rename.selection_anchor.and_then(|anchor| {
                    if anchor == rename.cursor {
                        None
                    } else {
                        Some((anchor.min(rename.cursor), anchor.max(rename.cursor)))
                    }
                });
                (
                    rename.tab_index,
                    rename.text.as_str(),
                    rename.cursor,
                    selection,
                )
            });
            let tab_infos: Vec<TabInfo> = self
                .tabs
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let is_renaming = renaming.as_ref().is_some_and(|(ri, _, _, _)| *ri == i);
                    let security_count = if t.security.has_events() {
                        t.security.active_event_count()
                    } else {
                        0
                    };
                    TabInfo {
                        title: &t.title,
                        is_active: i == self.active_tab,
                        security_count,
                        is_renaming,
                        rename_text: if is_renaming {
                            renaming.as_ref().map(|(_, text, _, _)| *text)
                        } else {
                            None
                        },
                        rename_cursor: if is_renaming {
                            renaming.as_ref().map_or(0, |(_, _, c, _)| *c)
                        } else {
                            0
                        },
                        rename_selection: if is_renaming {
                            renaming
                                .as_ref()
                                .and_then(|(_, _, _, selection)| *selection)
                        } else {
                            None
                        },
                    }
                })
                .collect();
            self.renderer
                .draw_tab_bar(&mut buffer, bw, bh, &tab_infos, self.hovered_tab, self.mouse_pos);

            // 3) Draw active tab terminal content.
            if let Some(tab) = self.tabs.get(self.active_tab) {
                if tab.scroll_offset == 0 {
                    self.renderer.render(
                        &mut buffer,
                        bw,
                        bh,
                        &tab.terminal.grid,
                        tab.selection.as_ref(),
                    );
                } else {
                    let display = tab.terminal.build_display(tab.scroll_offset);
                    self.renderer
                        .render(&mut buffer, bw, bh, &display, tab.selection.as_ref());
                }

                // 4) Draw cursor on top of terminal cells.
                if tab.scroll_offset == 0 && tab.terminal.cursor_visible {
                    let style = tab.terminal.cursor_style;
                    let show = if style.is_blinking() {
                        let ms = self.cursor_blink_start.elapsed().as_millis();
                        ms < 500 || (ms / 500).is_multiple_of(2)
                    } else {
                        true // Steady cursor is always visible.
                    };
                    if show {
                        self.renderer.draw_cursor(
                            &mut buffer,
                            bw,
                            bh,
                            tab.terminal.cursor_row,
                            tab.terminal.cursor_col,
                            &tab.terminal.grid,
                            style,
                        );
                    }
                }
            }

            // 4b) Draw scrollbar overlay.
            if let Some(tab) = self.tabs.get(self.active_tab) {
                let scrollback_len = tab.terminal.scrollback.len();
                if scrollback_len > 0 {
                    let hover = tab.scrollbar.hover || tab.scrollbar.dragging;
                    let opacity = if hover {
                        1.0_f32
                    } else {
                        let elapsed = tab.scrollbar.last_activity.elapsed().as_secs_f32();
                        if elapsed < 1.5 {
                            1.0
                        } else if elapsed < 1.8 {
                            1.0 - (elapsed - 1.5) / 0.3
                        } else {
                            0.0
                        }
                    };

                    if opacity > 0.0 {
                        self.renderer.render_scrollbar(
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

            // 4c) Draw drag overlay ON TOP of terminal content.
            if let Some(ref drag) = self.dragging_tab {
                if drag.is_active {
                    let insert_idx = {
                        let tw = self.renderer.tab_width(self.tabs.len(), bw as u32);
                        let mut idx = self.tabs.len();
                        for i in 0..self.tabs.len() {
                            let center = i as f64 * tw as f64 + tw as f64 / 2.0;
                            if drag.current_x < center {
                                idx = i;
                                break;
                            }
                        }
                        idx
                    };
                    self.renderer.draw_tab_drag_overlay(
                        &mut buffer,
                        bw,
                        bh,
                        &tab_infos,
                        drag.source_index,
                        drag.current_x,
                        insert_idx,
                    );
                }
            }

            // 5) Draw context menu overlay last.
            if let Some(ref popup) = self.security_popup {
                self.renderer
                    .draw_security_popup(&mut buffer, bw, bh, popup);
            }

            if let Some(ref menu) = self.context_menu {
                self.renderer.draw_context_menu(&mut buffer, bw, bh, menu);
            }

            let _ = buffer.present();
        }
    }
}
