//! Shared helpers for the CPU and GPU render paths.
//!
//! Extracts common frame-preparation logic that was previously duplicated
//! verbatim in `render_cpu.rs` and `render_gpu.rs`.

use crate::core::terminal::CursorStyle;
use crate::gui::*;

#[cfg(not(target_os = "macos"))]
use crate::gui::renderer::TabInfo;

/// Pre-computed tab bar state needed by both CPU and GPU render paths.
///
/// Built once per frame via `FerrumWindow::build_tab_bar_state`, then passed
/// by value to the renderer-specific drawing code.
#[cfg(not(target_os = "macos"))]
pub(in crate::gui) struct TabBarFrameState<'a> {
    pub tab_infos: Vec<TabInfo<'a>>,
    pub tab_tooltip: Option<String>,
    pub drag_info: Option<(usize, f64, f32)>,
    pub tab_offsets: Option<Vec<f32>>,
    pub show_tooltip: bool,
    pub tab_bar_visible: bool,
}

impl FerrumWindow {
    /// Builds the per-frame tab bar metadata shared by both render paths.
    ///
    /// On macOS this is a no-op (native tab bar), so the return type is
    /// behind `#[cfg(not(target_os = "macos"))]`.
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::events) fn build_tab_bar_state(&mut self, bw: usize) -> TabBarFrameState<'_> {
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
