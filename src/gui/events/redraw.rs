use crate::core::Color;
use crate::gui::renderer::backend::RendererBackend;
use crate::gui::renderer::TabInfo;
use crate::gui::*;
use std::time::{Duration, Instant};

const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(500);
const BLINK_WAKE_TOLERANCE: Duration = Duration::from_millis(20);
const SCROLLBAR_FADE_START: Duration = Duration::from_millis(1500);
const SCROLLBAR_FADE_END: Duration = Duration::from_millis(1800);
const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Quadratic ease-out: fast start, smooth deceleration.
fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

impl FerrumWindow {
    pub(in crate::gui) fn animation_schedule(&self, now: Instant) -> Option<(Instant, bool)> {
        let cursor = self.cursor_animation_schedule(now);
        let scrollbar = self.scrollbar_animation_schedule(now);
        let tab_anim = self.tab_animation_schedule(now);

        let schedules = [cursor, scrollbar, tab_anim];
        let mut result: Option<(Instant, bool)> = None;
        for s in schedules.into_iter().flatten() {
            result = Some(match result {
                None => s,
                Some((deadline, redraw)) => (deadline.min(s.0), redraw || s.1),
            });
        }
        result
    }

    fn tab_animation_schedule(&self, now: Instant) -> Option<(Instant, bool)> {
        let anim = self.tab_reorder_animation.as_ref()?;
        let elapsed = now.saturating_duration_since(anim.started);
        let duration = Duration::from_millis(anim.duration_ms as u64);
        if elapsed >= duration {
            return None; // Animation finished, will be cleaned up on next redraw.
        }
        Some((now + ANIMATION_FRAME_INTERVAL, true))
    }

    /// Returns current animation offsets for tab slide animation (or None if not animating).
    pub(in crate::gui) fn tab_animation_offsets(&self) -> Option<Vec<f32>> {
        let anim = self.tab_reorder_animation.as_ref()?;
        let elapsed = anim.started.elapsed().as_secs_f32();
        let duration = anim.duration_ms as f32 / 1000.0;
        let t = (elapsed / duration).min(1.0);
        let factor = 1.0 - ease_out(t);
        let offsets: Vec<f32> = anim.offsets.iter().map(|o| o * factor).collect();
        Some(offsets)
    }

    fn cursor_animation_schedule(&self, now: Instant) -> Option<(Instant, bool)> {
        let tab = self.active_tab_ref()?;
        if tab.scroll_offset != 0
            || !tab.terminal.cursor_visible
            || !tab.terminal.cursor_style.is_blinking()
        {
            return None;
        }

        let interval_ms = CURSOR_BLINK_INTERVAL.as_millis();
        let elapsed_ms = now
            .saturating_duration_since(self.cursor_blink_start)
            .as_millis();
        let phase = elapsed_ms / interval_ms;

        let prev_boundary_ms = phase * interval_ms;
        let next_boundary_ms = (phase + 1) * interval_ms;

        let prev_boundary =
            self.cursor_blink_start + Duration::from_millis(prev_boundary_ms as u64);
        let next_boundary =
            self.cursor_blink_start + Duration::from_millis(next_boundary_ms as u64);

        let redraw_now = now >= prev_boundary
            && now.saturating_duration_since(prev_boundary) <= BLINK_WAKE_TOLERANCE;

        Some((next_boundary, redraw_now))
    }

    fn scrollbar_animation_schedule(&self, now: Instant) -> Option<(Instant, bool)> {
        let tab = self.active_tab_ref()?;
        if tab.terminal.scrollback.is_empty() || tab.scrollbar.hover || tab.scrollbar.dragging {
            return None;
        }

        let elapsed = now.saturating_duration_since(tab.scrollbar.last_activity);
        if elapsed < SCROLLBAR_FADE_START {
            return Some((tab.scrollbar.last_activity + SCROLLBAR_FADE_START, false));
        }
        if elapsed < SCROLLBAR_FADE_END {
            return Some((now + ANIMATION_FRAME_INTERVAL, true));
        }
        None
    }

    pub(in crate::gui) fn apply_pending_resize(&mut self) {
        if let Some((rows, cols)) = self.pending_grid_resize.take() {
            self.resize_all_tabs(rows, cols);
        }
    }

    pub(crate) fn on_scale_factor_changed(&mut self, scale_factor: f64) {
        let prev_scale = self.backend.ui_scale();
        self.backend.set_scale(scale_factor);
        if (self.backend.ui_scale() - prev_scale).abs() < f64::EPSILON {
            return;
        }

        let size = self.window.inner_size();
        let (rows, cols) = self.calc_grid_size(size.width, size.height);
        self.pending_grid_resize = Some((rows, cols));
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
        let bw = w.get() as usize;
        let bh = h.get() as usize;

        // Build tab bar metadata (not needed on macOS — native tab bar).
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

            let tab_tooltip: Option<String> =
                self.backend
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
            let show_tooltip = !dragging_active
                && self.context_menu.is_none()
                && self.security_popup.is_none();

            (tab_infos, tab_tooltip, drag_info, tab_offsets, show_tooltip)
        };

        // Render through the backend enum.
        match &mut self.backend {
            RendererBackend::Cpu { renderer, surface } => {
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
                    if tab.scroll_offset == 0 {
                        renderer.render(
                            &mut buffer,
                            bw,
                            bh,
                            &tab.terminal.grid,
                            tab.selection.as_ref(),
                        );
                    } else {
                        let display = tab.terminal.build_display(tab.scroll_offset);
                        renderer.render(&mut buffer, bw, bh, &display, tab.selection.as_ref());
                    }

                    // 3) Draw cursor on top of terminal cells.
                    if tab.scroll_offset == 0 && tab.terminal.cursor_visible {
                        let style = tab.terminal.cursor_style;
                        let show = if style.is_blinking() {
                            let ms = self.cursor_blink_start.elapsed().as_millis();
                            ms < 500 || (ms / 500).is_multiple_of(2)
                        } else {
                            true
                        };
                        if show {
                            renderer.draw_cursor(
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

                // 4) Draw scrollbar overlay.
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

                // 5) Draw tab bar (not on macOS — native tab bar).
                #[cfg(not(target_os = "macos"))]
                {
                    renderer.draw_tab_bar(
                        &mut buffer,
                        bw,
                        bh,
                        &tab_infos,
                        self.hovered_tab,
                        self.mouse_pos,
                        tab_offsets.as_deref(),
                    );

                    // 6) Draw drag overlay.
                    if let Some((source_index, current_x, indicator_x)) = drag_info {
                        renderer.draw_tab_drag_overlay(
                            &mut buffer,
                            bw,
                            bh,
                            &tab_infos,
                            source_index,
                            current_x,
                            indicator_x,
                        );
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
                if show_tooltip {
                    if let Some(ref title) = tab_tooltip {
                        renderer.draw_tab_tooltip(
                            &mut buffer,
                            bw,
                            bh,
                            self.mouse_pos,
                            title,
                        );
                    }
                }

                let _ = buffer.present();
            }

            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => {
                use crate::gui::renderer::traits::Renderer;

                gpu.resize(w.get(), h.get());

                // Dummy buffer — GPU renderer ignores buffer params.
                let mut dummy = [];

                // 1) Render terminal grid.
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if tab.scroll_offset == 0 {
                        gpu.render(
                            &mut dummy,
                            bw,
                            bh,
                            &tab.terminal.grid,
                            tab.selection.as_ref(),
                        );
                    } else {
                        let display = tab.terminal.build_display(tab.scroll_offset);
                        gpu.render(&mut dummy, bw, bh, &display, tab.selection.as_ref());
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

                // 4) Draw tab bar (not on macOS — native tab bar).
                #[cfg(not(target_os = "macos"))]
                {
                    gpu.draw_tab_bar(
                        &mut dummy,
                        bw,
                        bh,
                        &tab_infos,
                        self.hovered_tab,
                        self.mouse_pos,
                        tab_offsets.as_deref(),
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

                // 6) Draw popups/menus.
                if let Some(ref popup) = self.security_popup {
                    gpu.draw_security_popup(&mut dummy, bw, bh, popup);
                }

                if let Some(ref menu) = self.context_menu {
                    gpu.draw_context_menu(&mut dummy, bw, bh, menu);
                }

                #[cfg(not(target_os = "macos"))]
                if show_tooltip {
                    if let Some(ref title) = tab_tooltip {
                        gpu.draw_tab_tooltip(&mut dummy, bw, bh, self.mouse_pos, title);
                    }
                }

                // 7) Present the frame via wgpu.
                gpu.present_frame();
            }
        }
    }
}
