//! Shared helpers for the CPU and GPU render paths.
//!
//! Extracts common frame-preparation logic that was previously duplicated
//! verbatim in `render_cpu.rs` and `render_gpu.rs`.

use crate::core::terminal::CursorStyle;
use crate::gui::renderer::traits::Renderer;
use crate::gui::*;

#[cfg(not(target_os = "macos"))]
use crate::gui::renderer::TabInfo;

/// Pre-computed tab bar state needed by both CPU and GPU render paths.
///
/// Built once per frame via `FerrumWindow::build_tab_bar_state`, then passed
/// by value to the renderer-specific drawing code.
#[cfg(not(target_os = "macos"))]
pub(in crate::gui) struct TabBarFrameState {
    pub tab_infos: Vec<TabBarFrameTabInfo>,
    pub tab_tooltip: Option<String>,
    pub drag_info: Option<(usize, f64, f32)>,
    pub tab_offsets: Option<Vec<f32>>,
    pub show_tooltip: bool,
    pub tab_bar_visible: bool,
}

/// Owned tab metadata captured for a single rendered frame.
#[cfg(not(target_os = "macos"))]
pub(in crate::gui) struct TabBarFrameTabInfo {
    pub title: String,
    pub is_active: bool,
    pub security_count: usize,
    pub hover_progress: f32,
    pub close_hover_progress: f32,
    pub is_renaming: bool,
    pub rename_text: Option<String>,
    pub rename_cursor: usize,
    pub rename_selection: Option<(usize, usize)>,
}

#[cfg(not(target_os = "macos"))]
impl TabBarFrameTabInfo {
    fn as_tab_info(&self) -> TabInfo<'_> {
        TabInfo {
            title: &self.title,
            is_active: self.is_active,
            security_count: self.security_count,
            hover_progress: self.hover_progress,
            close_hover_progress: self.close_hover_progress,
            is_renaming: self.is_renaming,
            rename_text: self.rename_text.as_deref(),
            rename_cursor: self.rename_cursor,
            rename_selection: self.rename_selection,
        }
    }
}

#[cfg(not(target_os = "macos"))]
impl TabBarFrameState {
    /// Converts owned frame tab metadata into renderer-facing borrowed `TabInfo` views.
    pub(in crate::gui::events) fn render_tab_infos(&self) -> Vec<TabInfo<'_>> {
        self.tab_infos
            .iter()
            .map(TabBarFrameTabInfo::as_tab_info)
            .collect()
    }
}

/// Read-only snapshot of per-frame state needed by `draw_frame_content`.
///
/// Constructed inline in each render path *after* pattern-matching
/// `self.backend`, enabling split borrows between the renderer and the
/// remaining `FerrumWindow` fields.
pub(in crate::gui::events) struct FrameParams<'a> {
    pub active_tab: Option<&'a TabState>,
    pub cursor_blink_start: std::time::Instant,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub hovered_tab: Option<usize>,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub mouse_pos: (f64, f64),
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub pinned: bool,
    pub security_popup: Option<&'a SecurityPopup>,
    pub context_menu: Option<&'a ContextMenu>,
}

impl FerrumWindow {
    /// Builds the per-frame tab bar metadata shared by both render paths.
    ///
    /// On macOS this is a no-op (native tab bar), so the return type is
    /// behind `#[cfg(not(target_os = "macos"))]`.
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::events) fn build_tab_bar_state(&mut self, bw: usize) -> TabBarFrameState {
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
                rename.text.clone(),
                rename.cursor,
                selection,
            )
        });
        let tab_infos: Vec<TabBarFrameTabInfo> = self
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
                TabBarFrameTabInfo {
                    title: t.title.clone(),
                    is_active: i == self.active_tab,
                    security_count,
                    hover_progress: self.tab_hover_progress.get(i).copied().unwrap_or(0.0),
                    close_hover_progress: self.close_hover_progress.get(i).copied().unwrap_or(0.0),
                    is_renaming,
                    rename_text: if is_renaming {
                        renaming.as_ref().map(|(_, text, _, _)| text.clone())
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

        let render_tab_infos: Vec<TabInfo<'_>> = tab_infos
            .iter()
            .map(TabBarFrameTabInfo::as_tab_info)
            .collect();
        let tab_tooltip: Option<String> = self
            .backend
            .tab_hover_tooltip(&render_tab_infos, self.hovered_tab, bw as u32)
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

        let tab_bar_visible = self.backend.tab_bar_height_px() > 0;

        TabBarFrameState {
            tab_infos,
            tab_tooltip,
            drag_info,
            tab_offsets,
            show_tooltip,
            tab_bar_visible,
        }
    }
}

/// Draws the complete terminal frame content using the given renderer.
///
/// This is the unified render sequence shared by both CPU and GPU paths:
/// terminal grid, cursor, scrollbar, tab bar, drag overlay, popups, tooltip.
pub(in crate::gui::events) fn draw_frame_content(
    renderer: &mut dyn Renderer,
    buffer: &mut [u32],
    bw: usize,
    bh: usize,
    params: &FrameParams<'_>,
    #[cfg(not(target_os = "macos"))] tab_bar: &TabBarFrameState,
    #[cfg(not(target_os = "macos"))] frame_tab_infos: &[TabInfo<'_>],
) {
    // 1) Draw active tab terminal content.
    if let Some(tab) = params.active_tab {
        let viewport_start = tab
            .terminal
            .scrollback
            .len()
            .saturating_sub(tab.scroll_offset);
        if tab.scroll_offset == 0 {
            renderer.render(
                buffer,
                bw,
                bh,
                &tab.terminal.grid,
                tab.selection.as_ref(),
                viewport_start,
            );
        } else {
            let display = tab.terminal.build_display(tab.scroll_offset);
            renderer.render(
                buffer,
                bw,
                bh,
                &display,
                tab.selection.as_ref(),
                viewport_start,
            );
        }

        // 2) Draw cursor on top of terminal cells.
        if tab.scroll_offset == 0
            && tab.terminal.cursor_visible
            && should_show_cursor(params.cursor_blink_start, tab.terminal.cursor_style)
        {
            renderer.draw_cursor(
                buffer,
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
    if let Some(tab) = params.active_tab {
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
                    buffer,
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
        if tab_bar.tab_bar_visible {
            renderer.draw_tab_bar(
                buffer,
                bw,
                bh,
                frame_tab_infos,
                params.hovered_tab,
                params.mouse_pos,
                tab_bar.tab_offsets.as_deref(),
                params.pinned,
            );

            // 5) Draw drag overlay.
            if let Some((source_index, current_x, indicator_x)) = tab_bar.drag_info {
                renderer.draw_tab_drag_overlay(
                    buffer,
                    bw,
                    bh,
                    frame_tab_infos,
                    source_index,
                    current_x,
                    indicator_x,
                );
            }
        }
    }

    // 6) Draw popups/menus.
    if let Some(popup) = params.security_popup {
        renderer.draw_security_popup(buffer, bw, bh, popup);
    }

    if let Some(menu) = params.context_menu {
        renderer.draw_context_menu(buffer, bw, bh, menu);
    }

    // 7) Draw tooltip.
    #[cfg(not(target_os = "macos"))]
    if tab_bar.show_tooltip
        && tab_bar.tab_bar_visible
        && let Some(ref title) = tab_bar.tab_tooltip
    {
        renderer.draw_tab_tooltip(buffer, bw, bh, params.mouse_pos, title);
    }
}

/// Computes scrollbar opacity based on hover state and time since last activity.
///
/// Returns 0.0 when the scrollbar should be invisible, 1.0 when fully visible,
/// and a smooth fade-out between 1.5s and 1.8s of inactivity.
pub(in crate::gui::events) fn scrollbar_opacity(
    hover: bool,
    dragging: bool,
    last_activity: std::time::Instant,
) -> f32 {
    if hover || dragging {
        1.0
    } else {
        let elapsed = last_activity.elapsed().as_secs_f32();
        if elapsed < 1.5 {
            1.0
        } else if elapsed < 1.8 {
            1.0 - (elapsed - 1.5) / 0.3
        } else {
            0.0
        }
    }
}

/// Determines whether the cursor should be visible this frame, accounting
/// for blinking.
pub(in crate::gui::events) fn should_show_cursor(
    blink_start: std::time::Instant,
    style: CursorStyle,
) -> bool {
    if style.is_blinking() {
        let ms = blink_start.elapsed().as_millis();
        ms < 500 || (ms / 500).is_multiple_of(2)
    } else {
        true
    }
}
