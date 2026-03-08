//! Shared helpers for the CPU and GPU render paths.
//!
//! Extracts common frame-preparation logic that was previously duplicated
//! verbatim in `render_cpu.rs` and `render_gpu.rs`.

use crate::core::terminal::CursorStyle;
use crate::gui::pane::{DIVIDER_WIDTH, PaneLeaf, PaneNode, PaneRect, SplitDirection, split_rect};
use crate::gui::renderer::traits::Renderer;
use crate::gui::renderer::{RenderTarget, ScrollbarState};
use crate::gui::renderer::shared::banner_layout::UpdateBannerLayout;
use crate::gui::*;

#[cfg(not(target_os = "macos"))]
use crate::gui::renderer::TabInfo;

/// Opacity of the inactive-pane dim overlay.
const INACTIVE_PANE_DIM_ALPHA: f32 = 0.18;

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
    pub index: usize,
    pub is_active: bool,
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
            #[cfg(not(target_os = "macos"))]
            index: self.index,
            #[cfg(not(target_os = "macos"))]
            is_active: self.is_active,
            #[cfg(not(target_os = "macos"))]
            hover_progress: self.hover_progress,
            #[cfg(not(target_os = "macos"))]
            close_hover_progress: self.close_hover_progress,
            is_renaming: self.is_renaming,
            #[cfg(not(target_os = "macos"))]
            rename_text: self.rename_text.as_deref(),
            #[cfg(not(target_os = "macos"))]
            rename_cursor: self.rename_cursor,
            #[cfg(not(target_os = "macos"))]
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
    pub tab: Option<&'a crate::gui::state::TabState>,
    pub cursor_blink_start: std::time::Instant,
    pub cursor_blink_interval_ms: u64,
    /// When `true`, the terminal text cursor is not drawn.
    /// Set during resize so the cursor does not visually jump to an intermediate
    /// position while the shell hasn't yet redrawn the prompt via SIGWINCH.
    pub suppress_cursor: bool,
    #[cfg(not(target_os = "macos"))]
    pub hovered_tab: Option<usize>,
    #[cfg(not(target_os = "macos"))]
    pub mouse_pos: (f64, f64),
    #[cfg(not(target_os = "macos"))]
    pub pinned: bool,
    /// Pre-computed update banner geometry, or `None` when the banner is not shown.
    pub update_banner: Option<UpdateBannerLayout>,
}

/// Window-level inputs for [`build_frame_params`], grouping the fields that are
/// borrowed from `FerrumWindow` by name.
///
/// Using a struct keeps [`build_frame_params`] within the clippy argument-count limit
/// while still avoiding a whole-`self` borrow (which would conflict with the
/// simultaneous mutable borrow of `self.backend`).
pub(in crate::gui::events) struct FrameParamsInput<'a> {
    pub tabs: &'a [crate::gui::state::TabState],
    pub active_tab: usize,
    pub cursor_blink_start: std::time::Instant,
    pub cursor_blink_interval_ms: u64,
    pub sigwinch_deadline: Option<std::time::Instant>,
    #[cfg(not(target_os = "macos"))]
    pub hovered_tab: Option<usize>,
    #[cfg(not(target_os = "macos"))]
    pub mouse_pos: (f64, f64),
    #[cfg(not(target_os = "macos"))]
    pub pinned: bool,
    pub update_banner_dismissed: bool,
    pub update_install_state: &'a crate::gui::state::UpdateInstallState,
    pub pending_update_tag: Option<&'a str>,
}

/// Constructs a [`FrameParamsInput`] from a `FerrumWindow` reference.
///
/// Using a macro instead of a method avoids a whole-`self` borrow conflict
/// when `self.backend` is already mutably borrowed by the caller.
macro_rules! make_frame_params_input {
    ($self:ident) => {
        $crate::gui::events::render_shared::FrameParamsInput {
            tabs: &$self.tabs,
            active_tab: $self.active_tab,
            cursor_blink_start: $self.cursor_blink_start,
            cursor_blink_interval_ms: $self.cursor_blink_interval_ms,
            sigwinch_deadline: $self.sigwinch_deadline,
            #[cfg(not(target_os = "macos"))]
            hovered_tab: $self.hovered_tab,
            #[cfg(not(target_os = "macos"))]
            mouse_pos: $self.mouse_pos,
            #[cfg(not(target_os = "macos"))]
            pinned: $self.pinned,
            update_banner_dismissed: $self.update_banner_dismissed,
            update_install_state: &$self.update_install_state,
            pending_update_tag: $self.pending_update_tag.as_deref(),
        }
    };
}
pub(in crate::gui::events) use make_frame_params_input;

/// Builds the read-only [`FrameParams`] snapshot used by both CPU and GPU render paths.
///
/// Accepts individual fields from `FerrumWindow` (via [`FrameParamsInput`]) rather than
/// `&self`, so callers can hold a simultaneous mutable borrow of `self.backend` without
/// triggering a split-borrow conflict.
///
/// `tab_layout_metrics` and `tab_bar_h` must be read from `self.backend` before the caller
/// pattern-matches the backend variant mutably.
pub(in crate::gui::events) fn build_frame_params<'a>(
    input: FrameParamsInput<'a>,
    tab_layout_metrics: &crate::gui::renderer::shared::tab_math::TabLayoutMetrics,
    tab_bar_h: u32,
    bw: u32,
    bh: u32,
) -> FrameParams<'a> {
    FrameParams {
        tab: input.tabs.get(input.active_tab),
        cursor_blink_start: input.cursor_blink_start,
        cursor_blink_interval_ms: input.cursor_blink_interval_ms,
        suppress_cursor: input.sigwinch_deadline.is_some(),
        #[cfg(not(target_os = "macos"))]
        hovered_tab: input.hovered_tab,
        #[cfg(not(target_os = "macos"))]
        mouse_pos: input.mouse_pos,
        #[cfg(not(target_os = "macos"))]
        pinned: input.pinned,
        update_banner: compute_banner(
            input.update_banner_dismissed,
            input.update_install_state,
            input.pending_update_tag,
            tab_layout_metrics,
            tab_bar_h,
            bw,
            bh,
        ),
    }
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
                let display_title = if t.is_renamed {
                    t.title.clone()
                } else {
                    t.focused_leaf()
                        .and_then(|leaf| leaf.cwd())
                        .unwrap_or_else(|| t.title.clone())
                };
                TabBarFrameTabInfo {
                    title: display_title,
                    index: i,
                    is_active: i == self.active_tab,
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
        let show_tooltip = !dragging_active;

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

/// Computes the optional update banner layout for a frame.
///
/// Extracted from both CPU and GPU render paths to eliminate the duplicated
/// banner-visibility guard and `compute_update_banner_layout` call.
///
/// # Arguments
/// * `dismissed` — `true` when the user has dismissed the banner.
/// * `install_state` — current install phase; `Done` hides the banner.
/// * `pending_tag` — version tag string when an update is pending, `None` otherwise.
/// * `tab_layout_metrics` — pre-computed renderer metrics.
/// * `tab_bar_h` — tab bar height in physical pixels.
/// * `bw`, `bh` — frame buffer width/height in physical pixels.
pub(in crate::gui::events) fn compute_banner(
    dismissed: bool,
    install_state: &crate::gui::state::UpdateInstallState,
    pending_tag: Option<&str>,
    tab_layout_metrics: &crate::gui::renderer::shared::tab_math::TabLayoutMetrics,
    tab_bar_h: u32,
    bw: u32,
    bh: u32,
) -> Option<UpdateBannerLayout> {
    use crate::gui::renderer::shared::banner_layout::compute_update_banner_layout;
    use crate::gui::state::UpdateInstallState;

    if dismissed || *install_state == UpdateInstallState::Done {
        return None;
    }
    pending_tag.and_then(|tag| {
        compute_update_banner_layout(tag, tab_layout_metrics, bw, bh, tab_bar_h).map(|mut layout| {
            layout.installing = *install_state == UpdateInstallState::Installing;
            layout
        })
    })
}

/// Draws the complete terminal frame content using the given renderer.
///
/// This is the unified render sequence shared by both CPU and GPU paths:
/// terminal grid, cursor, scrollbar, tab bar, drag overlay, tooltip.
///
/// For tabs with multiple panes, iterates the pane tree and renders each leaf
/// into its assigned sub-rectangle, with dividers between panes and a dim
/// overlay on inactive panes.
pub(in crate::gui::events) fn draw_frame_content(
    renderer: &mut dyn Renderer,
    buffer: &mut [u32],
    bw: usize,
    bh: usize,
    params: &FrameParams<'_>,
    #[cfg(not(target_os = "macos"))] tab_bar: &TabBarFrameState,
    #[cfg(not(target_os = "macos"))] frame_tab_infos: &[TabInfo<'_>],
) {
    let mut target = RenderTarget { buffer, width: bw, height: bh };

    // 1) Draw terminal content — single-pane fast path or multi-pane loop.
    if let Some(tab) = params.tab {
        if tab.has_multiple_panes() {
            // Multi-pane: compute layout and render each pane into its rect.
            let tab_bar_h = renderer.tab_bar_height_px();
            let padding = renderer.window_padding_px();
            let terminal_rect = PaneRect {
                x: padding,
                y: tab_bar_h + padding,
                width: (bw as u32).saturating_sub(padding * 2),
                height: (bh as u32).saturating_sub(tab_bar_h + padding * 2),
            };
            let divider_px = DIVIDER_WIDTH;
            let pane_pad = renderer.pane_inner_padding_px();
            let pane_layout = tab.pane_tree.layout(terminal_rect, divider_px);

            for &(pane_id, rect) in &pane_layout {
                if let Some(leaf) = tab.pane_tree.find_leaf(pane_id) {
                    let is_focused = pane_id == tab.focused_pane;
                    let fg_dim = if is_focused {
                        0.0
                    } else {
                        INACTIVE_PANE_DIM_ALPHA
                    };

                    let content = rect.inset(pane_pad);

                    // Render terminal content into pane area.
                    renderer.render_in_rect(
                        &mut target,
                        &leaf.terminal.screen,
                        leaf.selection.as_ref(),
                        leaf.scroll_offset,
                        content,
                        fg_dim,
                    );

                    // Cursor.
                    if cursor_should_draw(params, leaf, is_focused) {
                        renderer.draw_cursor_in_rect(
                            &mut target,
                            leaf.terminal.cursor_row(),
                            leaf.terminal.cursor_col(),
                            &leaf.terminal.screen,
                            leaf.terminal.cursor_style,
                            content,
                        );
                    }

                    // Scrollbar within the full pane rect (not inset).
                    if let Some(sb) = leaf_scrollbar_state(leaf) {
                        renderer.render_scrollbar_in_rect(&mut target, &sb, rect);
                    }
                }
            }

            // Draw dividers between panes.
            if !target.buffer.is_empty() {
                let divider_color = renderer.split_divider_color_pixel();
                draw_dividers(
                    target.buffer,
                    target.width,
                    target.height,
                    &tab.pane_tree,
                    terminal_rect,
                    divider_px,
                    divider_color,
                );
            } else {
                draw_dividers_with_renderer(renderer, &tab.pane_tree, terminal_rect, divider_px);
            }
        } else {
            // Single-pane: use the original render path (faster, no rect clipping).
            if let Some(leaf) = tab.focused_leaf() {
                renderer.render(
                    &mut target,
                    &leaf.terminal.screen,
                    leaf.selection.as_ref(),
                    leaf.scroll_offset,
                );

                // Cursor.
                if cursor_should_draw(params, leaf, true) {
                    renderer.draw_cursor(
                        &mut target,
                        leaf.terminal.cursor_row(),
                        leaf.terminal.cursor_col(),
                        &leaf.terminal.screen,
                        leaf.terminal.cursor_style,
                    );
                }

                // Scrollbar.
                if let Some(sb) = leaf_scrollbar_state(leaf) {
                    renderer.render_scrollbar(&mut target, &sb);
                }
            }
        }
    }

    // 4) Draw tab bar (not on macOS -- native tab bar).
    #[cfg(not(target_os = "macos"))]
    {
        if tab_bar.tab_bar_visible {
            let tab_bar_params = crate::gui::renderer::TabBarDrawParams {
                tabs: frame_tab_infos,
                hovered_tab: params.hovered_tab,
                mouse_pos: params.mouse_pos,
                tab_offsets: tab_bar.tab_offsets.as_deref(),
                pinned: params.pinned,
                settings_open: crate::gui::platform::is_settings_window_open(),
            };
            renderer.draw_tab_bar(
                &mut target,
                &tab_bar_params,
            );

            // 5) Draw drag overlay.
            if let Some((source_index, current_x, indicator_x)) = tab_bar.drag_info {
                renderer.draw_tab_drag_overlay(
                    &mut target,
                    frame_tab_infos,
                    source_index,
                    current_x,
                    indicator_x,
                );
            }
        }
    }

    // 6) Draw tooltip.
    #[cfg(not(target_os = "macos"))]
    if tab_bar.show_tooltip
        && tab_bar.tab_bar_visible
        && let Some(ref title) = tab_bar.tab_tooltip
    {
        renderer.draw_tab_tooltip(&mut target, params.mouse_pos, title);
    }

    // 7) Draw update banner (when available).
    if let Some(ref banner) = params.update_banner {
        renderer.draw_update_banner(&mut target, banner);
    }
}

/// Recursively draws divider lines between split panes.
fn draw_dividers(
    buffer: &mut [u32],
    bw: usize,
    bh: usize,
    tree: &PaneNode,
    rect: PaneRect,
    divider_px: u32,
    divider_color: u32,
) {
    if let PaneNode::Split(split) = tree {
        let (first_rect, second_rect) = split_rect(rect, split.direction, split.ratio, divider_px);

        match split.direction {
            SplitDirection::Horizontal => {
                let div_x = first_rect.x + first_rect.width;
                for py in rect.y..(rect.y + rect.height).min(bh as u32) {
                    for dx in 0..divider_px {
                        let px = div_x + dx;
                        if (px as usize) < bw {
                            let idx = py as usize * bw + px as usize;
                            if idx < buffer.len() {
                                buffer[idx] = divider_color;
                            }
                        }
                    }
                }
            }
            SplitDirection::Vertical => {
                let div_y = first_rect.y + first_rect.height;
                for dy in 0..divider_px {
                    let py = div_y + dy;
                    if (py as usize) < bh {
                        for px in rect.x..(rect.x + rect.width).min(bw as u32) {
                            let idx = py as usize * bw + px as usize;
                            if idx < buffer.len() {
                                buffer[idx] = divider_color;
                            }
                        }
                    }
                }
            }
        }

        // Recurse into children.
        draw_dividers(buffer, bw, bh, &split.first, first_rect, divider_px, divider_color);
        draw_dividers(buffer, bw, bh, &split.second, second_rect, divider_px, divider_color);
    }
}

/// Recursively emits split divider rectangles to the renderer (GPU path).
fn draw_dividers_with_renderer(
    renderer: &mut dyn Renderer,
    tree: &PaneNode,
    rect: PaneRect,
    divider_px: u32,
) {
    if let PaneNode::Split(split) = tree {
        let (first_rect, second_rect) = split_rect(rect, split.direction, split.ratio, divider_px);

        let divider_rect = match split.direction {
            SplitDirection::Horizontal => PaneRect {
                x: first_rect.x + first_rect.width,
                y: rect.y,
                width: divider_px,
                height: rect.height,
            },
            SplitDirection::Vertical => PaneRect {
                x: rect.x,
                y: first_rect.y + first_rect.height,
                width: rect.width,
                height: divider_px,
            },
        };
        renderer.draw_pane_divider(divider_rect);

        draw_dividers_with_renderer(renderer, &split.first, first_rect, divider_px);
        draw_dividers_with_renderer(renderer, &split.second, second_rect, divider_px);
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
    interval_ms: u64,
) -> bool {
    if style.is_blinking() {
        let interval = interval_ms as u128;
        if interval == 0 {
            return true;
        }
        let ms = blink_start.elapsed().as_millis();
        ms < interval || (ms / interval).is_multiple_of(2)
    } else {
        true
    }
}

/// Returns whether the cursor should be drawn for `leaf` this frame.
fn cursor_should_draw(params: &FrameParams<'_>, leaf: &PaneLeaf, is_focused: bool) -> bool {
    !params.suppress_cursor
        && leaf.scroll_offset == 0
        && leaf.terminal.cursor_visible
        && is_focused
        && should_show_cursor(
            params.cursor_blink_start,
            leaf.terminal.cursor_style,
            params.cursor_blink_interval_ms,
        )
}

/// Builds the scrollbar render state for `leaf`, or returns `None` when the
/// scrollbar should not be drawn (no scrollback, or fully faded out).
fn leaf_scrollbar_state(leaf: &PaneLeaf) -> Option<ScrollbarState> {
    let scrollback_len = leaf.terminal.screen.scrollback_len();
    if scrollback_len == 0 {
        return None;
    }
    let hover = leaf.scrollbar.hover || leaf.scrollbar.dragging;
    let opacity = scrollbar_opacity(
        leaf.scrollbar.hover,
        leaf.scrollbar.dragging,
        leaf.scrollbar.last_activity,
    );
    if opacity > 0.0 {
        Some(ScrollbarState {
            scroll_offset: leaf.scroll_offset,
            scrollback_len,
            grid_rows: leaf.terminal.screen.viewport_rows(),
            opacity,
            hover,
        })
    } else {
        None
    }
}
