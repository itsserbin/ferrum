use crate::core::Color;

/// Tab context menu actions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextAction {
    CloseTab,
    RenameTab,
    DuplicateTab,
    CopySelection,
    Paste,
    ClearSelection,
}

/// Context menu origin/target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextMenuTarget {
    Tab { tab_index: usize },
    TerminalSelection,
}

/// Context menu state.
pub struct ContextMenu {
    pub x: u32,
    pub y: u32,
    pub target: ContextMenuTarget,
    pub items: Vec<(ContextAction, &'static str)>,
    pub hover_index: Option<usize>,
    pub hover_progress: Vec<f32>,
    pub opened_at: std::time::Instant,
}

impl ContextMenu {
    /// Pure hit-test: returns the hovered item index given pointer coordinates
    /// and the cell dimensions used to compute menu geometry.
    pub fn hit_test(&self, x: f64, y: f64, cell_width: u32, cell_height: u32) -> Option<usize> {
        let mw = self.width(cell_width);
        let ih = self.item_height(cell_height);
        let mh = self.height(cell_height);

        if x < self.x as f64
            || x >= (self.x + mw) as f64
            || y < self.y as f64
            || y >= (self.y + mh) as f64
        {
            return None;
        }

        let rel_y = (y - self.y as f64 - 2.0) as u32;
        let idx = rel_y / ih;
        if (idx as usize) < self.items.len() {
            Some(idx as usize)
        } else {
            None
        }
    }

    /// Computes the full visual layout for this context menu.
    ///
    /// The returned [`ContextMenuLayout`] contains every rounded rect and text
    /// span needed to draw the menu -- renderers just iterate and issue their
    /// backend-specific draw calls.
    pub fn layout(&self, cell_width: u32, cell_height: u32, ui_scale: f64) -> ContextMenuLayout {
        let mw = self.width(cell_width);
        let ih = self.item_height(cell_height);
        let mh = self.height(cell_height);
        let mx = self.x as f32;
        let my = self.y as f32;
        let radius = scaled_px(6, ui_scale) as f32;

        let open_t = (self.opened_at.elapsed().as_secs_f32() / 0.14).clamp(0.0, 1.0);
        let open_ease = 1.0 - (1.0 - open_t) * (1.0 - open_t);
        let panel_opacity = (0.894 + open_ease * 0.08).clamp(0.0, 1.0);

        let bg = RoundedRectCmd {
            x: mx,
            y: my,
            w: mw as f32,
            h: mh as f32,
            radius,
            color: super::MENU_BG,
            opacity: panel_opacity,
        };
        let border = RoundedRectCmd {
            x: mx,
            y: my,
            w: mw as f32,
            h: mh as f32,
            radius,
            color: 0xFFFFFF,
            opacity: 0.118,
        };

        let pad2 = scaled_px(2, ui_scale) as f32;
        let pad4 = scaled_px(4, ui_scale) as f32;
        let pad8 = scaled_px(8, ui_scale) as f32;
        let pad1 = scaled_px(1, ui_scale) as f32;

        let items = self
            .items
            .iter()
            .enumerate()
            .map(|(i, (action, label))| {
                let item_y = my + pad2 + i as f32 * ih as f32;
                let hover_t = self
                    .hover_progress
                    .get(i)
                    .copied()
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0);

                let hover_rect = if hover_t > 0.01 {
                    let hover_x = mx + pad4;
                    let hover_w = mw as f32 - pad8;
                    let hover_h = ih as f32 - pad1;
                    Some(RoundedRectCmd {
                        x: hover_x,
                        y: item_y,
                        w: hover_w,
                        h: hover_h,
                        radius,
                        color: super::MENU_HOVER_BG,
                        opacity: 0.47 + hover_t * 0.39,
                    })
                } else {
                    None
                };

                let fg = if *action == ContextAction::CloseTab {
                    super::DESTRUCTIVE_COLOR.to_pixel()
                } else {
                    Color::DEFAULT_FG.to_pixel()
                };

                let text_x = mx + cell_width as f32;
                let text_y = item_y + pad2;

                ContextMenuItemLayout {
                    hover_rect,
                    text: TextCmd {
                        x: text_x,
                        y: text_y,
                        text: (*label).to_string(),
                        color: fg,
                        opacity: 1.0,
                    },
                }
            })
            .collect();

        ContextMenuLayout { bg, border, items }
    }
}

/// Render-time tab metadata.
pub struct TabInfo<'a> {
    pub title: &'a str,
    pub is_active: bool,
    pub security_count: usize,
    pub hover_progress: f32,
    pub close_hover_progress: f32,
    pub is_renaming: bool,
    pub rename_text: Option<&'a str>,
    pub rename_cursor: usize,
    pub rename_selection: Option<(usize, usize)>, // Byte range within rename_text.
}

pub struct SecurityPopup {
    pub tab_index: usize,
    pub x: u32,
    pub y: u32,
    pub title: &'static str,
    pub lines: Vec<String>,
}

impl SecurityPopup {
    /// Computes the full visual layout for this security popup.
    ///
    /// The returned [`SecurityPopupLayout`] contains every rect and text span
    /// needed to draw the popup -- renderers just iterate and issue their
    /// backend-specific draw calls.
    pub fn layout(
        &self,
        cell_width: u32,
        cell_height: u32,
        ui_scale: f64,
        buf_width: u32,
        buf_height: u32,
    ) -> SecurityPopupLayout {
        let (rx, ry, rw, rh) = self.clamped_rect(cell_width, cell_height, buf_width, buf_height);
        let x = rx as f32;
        let y = ry as f32;
        let w = rw as f32;
        let h = rh as f32;
        let radius = scaled_px(6, ui_scale) as f32;
        let line_h = self.line_height(cell_height) as f32;
        let accent_pixel = super::SECURITY_ACCENT.to_pixel();

        let bg = RoundedRectCmd {
            x,
            y,
            w,
            h,
            radius,
            color: super::MENU_BG,
            opacity: 0.973,
        };
        let border = RoundedRectCmd {
            x,
            y,
            w,
            h,
            radius,
            color: 0xFFFFFF,
            opacity: 0.078,
        };

        let pad2 = scaled_px(2, ui_scale) as f32;
        let pad3 = scaled_px(3, ui_scale) as f32;
        let pad4 = scaled_px(4, ui_scale) as f32;
        let half_cw = cell_width as f32 / 2.0;

        let title = TextCmd {
            x: x + half_cw,
            y: y + pad2,
            text: self.title.to_string(),
            color: accent_pixel,
            opacity: 1.0,
        };

        let sep_y = y + line_h;
        let separator = FlatRectCmd {
            x: x + pad3,
            y: sep_y,
            w: w - pad3 * 2.0,
            h: 1.0,
            color: accent_pixel,
            opacity: 0.47,
        };

        let lines = self
            .lines
            .iter()
            .enumerate()
            .map(|(i, line)| TextCmd {
                x: x + half_cw,
                y: y + line_h + pad4 + i as f32 * line_h,
                text: format!("\u{2022} {}", line),
                color: Color::DEFAULT_FG.to_pixel(),
                opacity: 1.0,
            })
            .collect();

        SecurityPopupLayout {
            bg,
            border,
            title,
            separator,
            lines,
        }
    }
}

// ── Layout structs ──────────────────────────────────────────────────

/// Scales a base pixel value by the given UI scale factor.
///
/// Mirrors `CpuRenderer::scaled_px` / `FontMetrics::scaled_px`.
fn scaled_px(base: u32, ui_scale: f64) -> u32 {
    if base == 0 {
        0
    } else {
        ((base as f64 * ui_scale).round() as u32).max(1)
    }
}

/// A rounded rectangle in the context-menu / security-popup overlay.
pub struct RoundedRectCmd {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
    pub color: u32,
    pub opacity: f32,
}

/// A flat (non-rounded) rectangle command.
pub struct FlatRectCmd {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: u32,
    pub opacity: f32,
}

/// A single text span to draw.
pub struct TextCmd {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub color: u32,
    /// Text opacity (used by the GPU renderer; the CPU renderer draws at full opacity).
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    pub opacity: f32,
}

/// Pre-computed layout for a single context-menu item row.
pub struct ContextMenuItemLayout {
    /// Hover highlight rectangle; `None` when the item has no hover animation.
    pub hover_rect: Option<RoundedRectCmd>,
    pub text: TextCmd,
}

/// Pre-computed layout for the entire context menu overlay.
pub struct ContextMenuLayout {
    /// Background panel (fill).
    pub bg: RoundedRectCmd,
    /// Border overlay drawn on top of the background.
    pub border: RoundedRectCmd,
    /// Per-item layout, in display order.
    pub items: Vec<ContextMenuItemLayout>,
}

/// Pre-computed layout for the security popup overlay.
pub struct SecurityPopupLayout {
    /// Background panel (fill).
    pub bg: RoundedRectCmd,
    /// Border overlay drawn on top of the background.
    pub border: RoundedRectCmd,
    /// Title text.
    pub title: TextCmd,
    /// Horizontal separator line below the title.
    pub separator: FlatRectCmd,
    /// Content lines (each prefixed with a bullet).
    pub lines: Vec<TextCmd>,
}

/// Result of tab-bar hit testing.
#[derive(Debug)]
pub enum TabBarHit {
    /// Clicked on a tab by index.
    Tab(usize),
    /// Clicked on a tab close button by index.
    CloseTab(usize),
    /// Clicked on the new-tab button.
    NewTab,
    /// Clicked on the pin button (non-macOS).
    #[cfg(not(target_os = "macos"))]
    PinButton,
    /// Clicked on a window control button (non-macOS).
    #[cfg(not(target_os = "macos"))]
    WindowButton(WindowButton),
    /// Clicked empty bar area (window drag).
    Empty,
}

/// Window control button type (non-macOS).
#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowButton {
    Minimize,
    Maximize,
    Close,
}

pub(super) struct GlyphBitmap {
    pub(super) data: Vec<u8>,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) left: i32,
    pub(super) top: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ContextMenu layout tests ────────────────────────────────────

    fn make_tab_menu() -> ContextMenu {
        ContextMenu::for_tab(100, 200, 0)
    }

    fn make_selection_menu() -> ContextMenu {
        ContextMenu::for_terminal_selection(50, 80)
    }

    #[test]
    fn context_menu_layout_bg_matches_menu_position() {
        let menu = make_tab_menu();
        let layout = menu.layout(8, 16, 1.0);
        assert_eq!(layout.bg.x, 100.0);
        assert_eq!(layout.bg.y, 200.0);
        assert!(layout.bg.w > 0.0);
        assert!(layout.bg.h > 0.0);
    }

    #[test]
    fn context_menu_layout_border_matches_bg() {
        let menu = make_tab_menu();
        let layout = menu.layout(8, 16, 1.0);
        assert_eq!(layout.border.x, layout.bg.x);
        assert_eq!(layout.border.y, layout.bg.y);
        assert_eq!(layout.border.w, layout.bg.w);
        assert_eq!(layout.border.h, layout.bg.h);
        // Border should be semi-transparent white overlay.
        assert_eq!(layout.border.color, 0xFFFFFF);
        assert!(layout.border.opacity < 0.2);
    }

    #[test]
    fn context_menu_layout_item_count_matches_menu() {
        let menu = make_tab_menu();
        let layout = menu.layout(8, 16, 1.0);
        assert_eq!(layout.items.len(), menu.items.len());
    }

    #[test]
    fn context_menu_layout_close_item_has_destructive_color() {
        let menu = make_tab_menu();
        let layout = menu.layout(8, 16, 1.0);
        // "Close" is the last item in a tab context menu.
        let close_item = &layout.items[2];
        let destructive_pixel = super::super::DESTRUCTIVE_COLOR.to_pixel();
        assert_eq!(close_item.text.color, destructive_pixel);
    }

    #[test]
    fn context_menu_layout_non_destructive_items_use_default_fg() {
        let menu = make_tab_menu();
        let layout = menu.layout(8, 16, 1.0);
        let default_fg = Color::DEFAULT_FG.to_pixel();
        // "Rename" and "Duplicate" are non-destructive.
        assert_eq!(layout.items[0].text.color, default_fg);
        assert_eq!(layout.items[1].text.color, default_fg);
    }

    #[test]
    fn context_menu_layout_no_hover_rects_without_progress() {
        let menu = make_tab_menu();
        let layout = menu.layout(8, 16, 1.0);
        for item in &layout.items {
            assert!(item.hover_rect.is_none());
        }
    }

    #[test]
    fn context_menu_layout_hover_rect_present_when_progress_nonzero() {
        let mut menu = make_tab_menu();
        menu.hover_progress[0] = 0.5;
        let layout = menu.layout(8, 16, 1.0);
        assert!(layout.items[0].hover_rect.is_some());
        assert!(layout.items[1].hover_rect.is_none());
    }

    #[test]
    fn context_menu_layout_text_labels_match() {
        let menu = make_selection_menu();
        let layout = menu.layout(8, 16, 1.0);
        assert_eq!(layout.items[0].text.text, "Copy");
        assert_eq!(layout.items[1].text.text, "Paste");
        assert_eq!(layout.items[2].text.text, "Clear Selection");
    }

    #[test]
    fn context_menu_layout_hidpi_scales_radius() {
        let menu = make_tab_menu();
        let layout_1x = menu.layout(8, 16, 1.0);
        let layout_2x = menu.layout(16, 32, 2.0);
        assert!(layout_2x.bg.radius > layout_1x.bg.radius);
    }

    #[test]
    fn context_menu_layout_bg_uses_menu_bg_color() {
        let menu = make_tab_menu();
        let layout = menu.layout(8, 16, 1.0);
        assert_eq!(layout.bg.color, super::super::MENU_BG);
    }

    // ── SecurityPopup layout tests ──────────────────────────────────

    fn make_popup() -> SecurityPopup {
        SecurityPopup {
            tab_index: 0,
            x: 50,
            y: 40,
            title: "Security Warning",
            lines: vec!["Line one".to_string(), "Line two".to_string()],
        }
    }

    #[test]
    fn security_popup_layout_bg_position_clamped() {
        let popup = make_popup();
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        assert!(layout.bg.x >= 0.0);
        assert!(layout.bg.y >= 0.0);
        assert!(layout.bg.w > 0.0);
        assert!(layout.bg.h > 0.0);
    }

    #[test]
    fn security_popup_layout_border_matches_bg() {
        let popup = make_popup();
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        assert_eq!(layout.border.x, layout.bg.x);
        assert_eq!(layout.border.y, layout.bg.y);
        assert_eq!(layout.border.w, layout.bg.w);
        assert_eq!(layout.border.h, layout.bg.h);
    }

    #[test]
    fn security_popup_layout_title_uses_accent_color() {
        let popup = make_popup();
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        assert_eq!(layout.title.color, super::super::SECURITY_ACCENT.to_pixel());
        assert_eq!(layout.title.text, "Security Warning");
    }

    #[test]
    fn security_popup_layout_lines_match_content() {
        let popup = make_popup();
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        assert_eq!(layout.lines.len(), 2);
        assert!(layout.lines[0].text.starts_with('\u{2022}'));
        assert!(layout.lines[0].text.contains("Line one"));
        assert!(layout.lines[1].text.contains("Line two"));
    }

    #[test]
    fn security_popup_layout_lines_use_default_fg() {
        let popup = make_popup();
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        let default_fg = Color::DEFAULT_FG.to_pixel();
        for line in &layout.lines {
            assert_eq!(line.color, default_fg);
        }
    }

    #[test]
    fn security_popup_layout_separator_between_title_and_lines() {
        let popup = make_popup();
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        // Separator y should be below title y and above first line y.
        assert!(layout.separator.y > layout.title.y);
        if !layout.lines.is_empty() {
            assert!(layout.separator.y < layout.lines[0].y);
        }
    }

    #[test]
    fn security_popup_layout_hidpi_scales_radius() {
        let popup = make_popup();
        let layout_1x = popup.layout(8, 16, 1.0, 800, 600);
        let layout_2x = popup.layout(16, 32, 2.0, 800, 600);
        assert!(layout_2x.bg.radius > layout_1x.bg.radius);
    }

    #[test]
    fn security_popup_layout_clamped_to_buffer_bounds() {
        // Place popup near the bottom-right corner.
        let popup = SecurityPopup {
            tab_index: 0,
            x: 790,
            y: 590,
            title: "Test Title",
            lines: vec!["A line".to_string()],
        };
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        // The popup should be fully within bounds.
        assert!((layout.bg.x + layout.bg.w) <= 800.0);
        assert!((layout.bg.y + layout.bg.h) <= 600.0);
    }

    // ── SecurityPopup::hit_test tests ─────────────────────────────

    #[test]
    fn security_popup_hit_test_inside_returns_true() {
        let popup = make_popup();
        // Point inside the popup's clamped rectangle.
        let (px, py, pw, ph) = popup.clamped_rect(8, 16, 800, 600);
        let cx = px as f64 + pw as f64 / 2.0;
        let cy = py as f64 + ph as f64 / 2.0;
        assert!(popup.hit_test(cx, cy, 8, 16, 800, 600));
    }

    #[test]
    fn security_popup_hit_test_outside_returns_false() {
        let popup = make_popup();
        // Point far outside the popup.
        assert!(!popup.hit_test(0.0, 0.0, 8, 16, 800, 600));
        assert!(!popup.hit_test(799.0, 599.0, 8, 16, 800, 600));
    }

    #[test]
    fn security_popup_hit_test_top_left_edge_returns_true() {
        let popup = make_popup();
        let (px, py, _, _) = popup.clamped_rect(8, 16, 800, 600);
        assert!(popup.hit_test(px as f64, py as f64, 8, 16, 800, 600));
    }

    #[test]
    fn security_popup_hit_test_bottom_right_edge_exclusive() {
        let popup = make_popup();
        let (px, py, pw, ph) = popup.clamped_rect(8, 16, 800, 600);
        // Exclusive end (just past the rect).
        assert!(!popup.hit_test((px + pw) as f64, (py + ph) as f64, 8, 16, 800, 600));
    }

    #[test]
    fn security_popup_hit_test_clamped_near_edge() {
        // Popup placed near bottom-right corner is clamped.
        let popup = SecurityPopup {
            tab_index: 0,
            x: 790,
            y: 590,
            title: "Test Title",
            lines: vec!["A line".to_string()],
        };
        let (px, py, pw, ph) = popup.clamped_rect(8, 16, 800, 600);
        let cx = px as f64 + pw as f64 / 2.0;
        let cy = py as f64 + ph as f64 / 2.0;
        assert!(popup.hit_test(cx, cy, 8, 16, 800, 600));
    }

    // ── scaled_px helper tests ──────────────────────────────────────

    #[test]
    fn scaled_px_identity_at_1x() {
        assert_eq!(scaled_px(6, 1.0), 6);
    }

    #[test]
    fn scaled_px_doubles_at_2x() {
        assert_eq!(scaled_px(6, 2.0), 12);
    }

    #[test]
    fn scaled_px_zero_returns_zero() {
        assert_eq!(scaled_px(0, 2.0), 0);
    }

    #[test]
    fn scaled_px_never_returns_zero_for_nonzero_input() {
        assert!(scaled_px(1, 0.01) >= 1);
    }
}
