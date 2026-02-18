#[cfg(all(feature = "gpu", not(target_os = "macos")))]
use crate::gui::renderer::TabInfo;
#[cfg(feature = "gpu")]
use crate::gui::renderer::backend::RendererBackend;
#[cfg(feature = "gpu")]
use crate::gui::renderer::traits::Renderer;
#[cfg(feature = "gpu")]
use crate::gui::*;

#[cfg(feature = "gpu")]
impl FerrumWindow {
    /// GPU rendering path: issues draw commands through the GPU renderer
    /// and presents the frame via wgpu.
    pub(super) fn render_gpu_frame(&mut self, w: NonZeroU32, h: NonZeroU32, bw: usize, bh: usize) {
        // Build tab bar metadata (not needed on macOS -- native tab bar).
        #[cfg(not(target_os = "macos"))]
        let (tab_infos, tab_tooltip, drag_info, tab_offsets, show_tooltip) = {
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
                        hover_progress: self.tab_hover_progress.get(i).copied().unwrap_or(0.0),
                        close_hover_progress: self
                            .close_hover_progress
                            .get(i)
                            .copied()
                            .unwrap_or(0.0),
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

            let tab_tooltip: Option<String> = self
                .backend
                .tab_hover_tooltip(&tab_infos, self.hovered_tab, bw as u32)
                .map(|s| s.to_owned());

            // Collect drag/overlay state needed during rendering.
            // Smooth the insertion indicator position with lerp.
            let drag_info = self.dragging_tab.as_mut().and_then(|drag| {
                if drag.is_active {
                    let insert_idx = self.backend.tab_insert_index_from_x(
                        drag.current_x,
                        self.tabs.len(),
                        bw as u32,
                    );
                    let tw = self.backend.tab_width(self.tabs.len(), bw as u32);
                    let target_x = self.backend.tab_origin_x(insert_idx, tw) as f32;
                    if drag.indicator_x < 0.0 {
                        drag.indicator_x = target_x;
                    } else {
                        drag.indicator_x += (target_x - drag.indicator_x) * 0.3;
                    }
                    Some((drag.source_index, drag.current_x, drag.indicator_x))
                } else {
                    None
                }
            });

            // Compute per-tab animation offsets (slide after reorder).
            let tab_offsets = self.tab_animation_offsets();

            // Clean up finished animation.
            if let Some(ref anim) = self.tab_reorder_animation {
                let elapsed = anim.started.elapsed().as_millis() as u32;
                if elapsed >= anim.duration_ms {
                    self.tab_reorder_animation = None;
                }
            }

            let dragging_active = self
                .dragging_tab
                .as_ref()
                .is_some_and(|drag| drag.is_active);
            let show_tooltip =
                !dragging_active && self.context_menu.is_none() && self.security_popup.is_none();

            (tab_infos, tab_tooltip, drag_info, tab_offsets, show_tooltip)
        };
        #[cfg(not(target_os = "macos"))]
        let tab_bar_visible = self.backend.tab_bar_height_px() > 0;

        let RendererBackend::Gpu(gpu) = &mut self.backend else {
            return;
        };

        gpu.resize(w.get(), h.get());

        // Dummy buffer -- GPU renderer ignores buffer params.
        let mut dummy = [];

        // 1) Render terminal grid.
        if let Some(tab) = self.tabs.get(self.active_tab) {
            let viewport_start = tab.terminal.scrollback.len().saturating_sub(tab.scroll_offset);
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
            if tab.scroll_offset == 0 && tab.terminal.cursor_visible {
                let style = tab.terminal.cursor_style;
                let show = if style.is_blinking() {
                    let ms = self.cursor_blink_start.elapsed().as_millis();
                    ms < 500 || (ms / 500).is_multiple_of(2)
                } else {
                    true
                };
                if show {
                    gpu.draw_cursor(
                        &mut dummy,
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

        // 3) Draw scrollbar overlay.
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
            if tab_bar_visible {
                gpu.draw_tab_bar(
                    &mut dummy,
                    bw,
                    bh,
                    &tab_infos,
                    self.hovered_tab,
                    self.mouse_pos,
                    tab_offsets.as_deref(),
                    self.pinned,
                );

                // 5) Draw drag overlay.
                if let Some((source_index, current_x, indicator_x)) = drag_info {
                    gpu.draw_tab_drag_overlay(
                        &mut dummy,
                        bw,
                        bh,
                        &tab_infos,
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
        if show_tooltip && tab_bar_visible {
            if let Some(ref title) = tab_tooltip {
                gpu.draw_tab_tooltip(&mut dummy, bw, bh, self.mouse_pos, title);
            }
        }

        // 7) Present the frame via wgpu.
        gpu.present_frame();
    }
}
