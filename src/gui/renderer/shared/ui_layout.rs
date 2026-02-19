//! Shared UI layout helpers used by both CPU and GPU renderers.
//!
//! This module contains pure, side-effect-free geometry and layout functions
//! for UI elements that are rendered identically by both backends: the shield
//! icon, the pushpin button, window control buttons, and rename-field selection.

#![cfg_attr(target_os = "macos", allow(dead_code))]

// ── Shield icon ──────────────────────────────────────────────────────

/// Computes the horizontal span (left, right) for each row of the shield icon.
///
/// The shield is drawn as three vertical zones:
/// 1. Top third — widening outward
/// 2. Middle third — full width
/// 3. Bottom third — tapering to a point
///
/// Returns a `Vec` of `(left_offset, right_offset)` pairs, one per row from
/// `dy = 0` to `dy = size - 1`, where offsets are relative to the icon origin.
pub fn shield_icon_spans(size: u32) -> Vec<(u32, u32)> {
    let mid = size / 2;
    let top_third = (size / 3).max(1);
    let bottom_start = (size * 2 / 3).max(top_third + 1);

    (0..size)
        .map(|dy| {
            let half_span = if dy < top_third {
                1 + dy / 2
            } else if dy < bottom_start {
                mid.saturating_sub(1).max(1)
            } else {
                let progress = dy - bottom_start;
                let denom = (size - bottom_start).max(1);
                let shrink = progress * mid.saturating_sub(1) / denom;
                mid.saturating_sub(shrink).max(1)
            };

            let left = mid.saturating_sub(half_span);
            let right = (mid + half_span).min(size.saturating_sub(1));
            (left, right)
        })
        .collect()
}

// ── Pin icon ─────────────────────────────────────────────────────────

/// Layout geometry for a Bootstrap-style vertical pushpin icon.
///
/// All coordinates are in physical pixels.  The rectangles are described as
/// `(x, y, w, h)` tuples in `f32` so that both the CPU (which casts to `i32`)
/// and the GPU renderer (which uses `f32` directly) can consume them.
pub struct PinIconLayout {
    /// Top head rectangle `(x, y, w, h)`.
    pub head: (f32, f32, f32, f32),
    /// Body rectangle `(x, y, w, h)`.
    pub body: (f32, f32, f32, f32),
    /// Platform/base rectangle `(x, y, w, h)`.
    pub platform: (f32, f32, f32, f32),
    /// Needle line: `(x_start, y_start, x_end, y_end)`.
    pub needle: (f32, f32, f32, f32),
    /// Needle stroke thickness.
    pub needle_thickness: f32,
    /// Resolved icon color as a 0xRRGGBB `u32`.
    pub color: u32,
}

/// Computes the pushpin icon layout centered in the given rectangle.
///
/// # Arguments
/// * `cx`, `cy` — center of the bounding rect (physical pixels).
/// * `scale` — UI scale factor (e.g. 1.0, 2.0).
/// * `pinned` — whether the window is currently pinned (always-on-top).
/// * `hovered` — whether the mouse is over the pin button.
/// * `pin_active_color` — color when pinned.
/// * `hover_color` — color when hovered but not pinned.
/// * `inactive_color` — color when neither pinned nor hovered.
#[allow(clippy::too_many_arguments)]
pub fn pin_icon_layout(
    cx: f32,
    cy: f32,
    scale: f32,
    pinned: bool,
    hovered: bool,
    pin_active_color: u32,
    hover_color: u32,
    inactive_color: u32,
) -> PinIconLayout {
    let head_w = 6.0 * scale;
    let head_h = 2.0 * scale;
    let body_w = 3.0 * scale;
    let body_h = 4.0 * scale;
    let platform_w = 7.0 * scale;
    let platform_h = 1.5 * scale;
    let needle_h = 4.0 * scale;
    let t = (1.2 * scale).clamp(1.0, 2.0);

    let top = cy - 6.0 * scale;

    let head = (cx - head_w / 2.0, top, head_w, head_h);

    let body_top = top + head_h;
    let body = (cx - body_w / 2.0, body_top, body_w, body_h);

    let platform_top = body_top + body_h;
    let platform = (cx - platform_w / 2.0, platform_top, platform_w, platform_h);

    let needle_top = platform_top + platform_h;
    let needle = (cx, needle_top, cx, needle_top + needle_h);

    let color = if pinned {
        pin_active_color
    } else if hovered {
        hover_color
    } else {
        inactive_color
    };

    PinIconLayout {
        head,
        body,
        platform,
        needle,
        needle_thickness: t,
        color,
    }
}

// ── Window buttons ───────────────────────────────────────────────────

/// Kind of window control button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowButtonKind {
    Minimize,
    Maximize,
    Close,
}

/// Pre-computed layout for a single window control button.
pub struct WindowButtonLayout {
    /// Button x-origin (physical pixels).
    pub x: u32,
    /// Button y-origin (always 0 — top of bar).
    pub y: u32,
    /// Button width (physical pixels).
    pub w: u32,
    /// Button height (physical pixels, equals bar height).
    pub h: u32,
    /// Whether the mouse is currently over this button.
    pub hovered: bool,
    /// Which button this is.
    pub kind: WindowButtonKind,
}

/// Computes layout for the three window control buttons (Minimize, Maximize, Close).
///
/// Buttons are positioned at the right edge of the tab bar: `[Min][Max][Close]`.
///
/// # Arguments
/// * `buf_width` — surface width (physical pixels).
/// * `bar_height` — tab bar height (physical pixels).
/// * `btn_width` — width of a single button (physical pixels).
/// * `mouse_pos` — current pointer position `(x, y)`.
pub fn window_buttons_layout(
    buf_width: u32,
    bar_height: u32,
    btn_width: u32,
    mouse_pos: (f64, f64),
) -> [WindowButtonLayout; 3] {
    let kinds = [
        (buf_width.saturating_sub(btn_width * 3), WindowButtonKind::Minimize),
        (buf_width.saturating_sub(btn_width * 2), WindowButtonKind::Maximize),
        (buf_width.saturating_sub(btn_width), WindowButtonKind::Close),
    ];

    kinds.map(|(btn_x, kind)| {
        let hovered = mouse_pos.0 >= btn_x as f64
            && mouse_pos.0 < (btn_x + btn_width) as f64
            && mouse_pos.1 >= 0.0
            && mouse_pos.1 < bar_height as f64;
        WindowButtonLayout {
            x: btn_x,
            y: 0,
            w: btn_width,
            h: bar_height,
            hovered,
            kind,
        }
    })
}

// ── Rename selection ─────────────────────────────────────────────────

/// Converts a byte-range selection to a character-range selection, clamped to
/// `max_chars`.
///
/// Returns `None` when the selection is empty or `start >= end`.
pub fn rename_selection_chars(
    text: &str,
    selection: Option<(usize, usize)>,
    max_chars: usize,
) -> Option<(usize, usize)> {
    selection.and_then(|(start, end)| {
        if start >= end {
            return None;
        }
        let start_chars = text
            .get(..start)
            .map_or(0, |prefix| prefix.chars().count());
        let end_chars = text
            .get(..end)
            .map_or(start_chars, |prefix| prefix.chars().count());
        Some((start_chars.min(max_chars), end_chars.min(max_chars)))
    })
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── shield_icon_spans ────────────────────────────────────────────

    #[test]
    fn shield_spans_length_matches_size() {
        let size = 12;
        let spans = shield_icon_spans(size);
        assert_eq!(spans.len(), size as usize);
    }

    #[test]
    fn shield_spans_symmetric_around_midpoint() {
        let size = 15;
        let mid = size / 2;
        let spans = shield_icon_spans(size);
        for &(left, right) in &spans {
            // The span should be roughly symmetric around `mid`.
            assert!(left <= mid);
            assert!(right >= mid);
        }
    }

    #[test]
    fn shield_spans_no_overflow() {
        let size = 10;
        let spans = shield_icon_spans(size);
        for &(left, right) in &spans {
            assert!(right < size, "right={right} must be < size={size}");
            assert!(left <= right);
        }
    }

    #[test]
    fn shield_spans_tapers_at_bottom() {
        let size = 18;
        let spans = shield_icon_spans(size);
        let last = spans.last().unwrap();
        let mid_row = &spans[size as usize / 2];
        // The bottom row should be narrower than or equal to the middle row.
        assert!((last.1 - last.0) <= (mid_row.1 - mid_row.0));
    }

    #[test]
    fn shield_spans_small_size() {
        // Edge case: very small shield.
        let spans = shield_icon_spans(3);
        assert_eq!(spans.len(), 3);
        for &(left, right) in &spans {
            assert!(left <= right);
        }
    }

    // ── pin_icon_layout ──────────────────────────────────────────────

    #[test]
    fn pin_layout_pinned_color() {
        let layout = pin_icon_layout(50.0, 50.0, 1.0, true, false, 0xAA, 0xBB, 0xCC);
        assert_eq!(layout.color, 0xAA);
    }

    #[test]
    fn pin_layout_hovered_color() {
        let layout = pin_icon_layout(50.0, 50.0, 1.0, false, true, 0xAA, 0xBB, 0xCC);
        assert_eq!(layout.color, 0xBB);
    }

    #[test]
    fn pin_layout_inactive_color() {
        let layout = pin_icon_layout(50.0, 50.0, 1.0, false, false, 0xAA, 0xBB, 0xCC);
        assert_eq!(layout.color, 0xCC);
    }

    #[test]
    fn pin_layout_head_above_body() {
        let layout = pin_icon_layout(100.0, 100.0, 2.0, false, false, 0, 0, 0);
        // Head y should be above body y.
        assert!(layout.head.1 < layout.body.1);
        // Body y should be above platform y.
        assert!(layout.body.1 < layout.platform.1);
    }

    #[test]
    fn pin_layout_needle_below_platform() {
        let layout = pin_icon_layout(100.0, 100.0, 1.0, false, false, 0, 0, 0);
        // Needle y_start should be at platform bottom.
        let platform_bottom = layout.platform.1 + layout.platform.3;
        assert!((layout.needle.1 - platform_bottom).abs() < 0.01);
    }

    #[test]
    fn pin_layout_thickness_clamped() {
        // Very small scale — thickness should clamp to 1.0.
        let layout = pin_icon_layout(50.0, 50.0, 0.5, false, false, 0, 0, 0);
        assert!((layout.needle_thickness - 1.0).abs() < 0.01);
        // Very large scale — thickness should clamp to 2.0.
        let layout = pin_icon_layout(50.0, 50.0, 4.0, false, false, 0, 0, 0);
        assert!((layout.needle_thickness - 2.0).abs() < 0.01);
    }

    // ── window_buttons_layout ────────────────────────────────────────

    #[test]
    fn window_buttons_order() {
        let btns = window_buttons_layout(1200, 36, 46, (-1.0, -1.0));
        assert_eq!(btns[0].kind, WindowButtonKind::Minimize);
        assert_eq!(btns[1].kind, WindowButtonKind::Maximize);
        assert_eq!(btns[2].kind, WindowButtonKind::Close);
    }

    #[test]
    fn window_buttons_hover_detection() {
        let btn_w = 46u32;
        let bar_h = 36u32;
        // Mouse over the Close button (rightmost).
        let close_x = (1200 - btn_w) as f64 + 10.0;
        let btns = window_buttons_layout(1200, bar_h, btn_w, (close_x, 10.0));
        assert!(!btns[0].hovered, "Minimize should not be hovered");
        assert!(!btns[1].hovered, "Maximize should not be hovered");
        assert!(btns[2].hovered, "Close should be hovered");
    }

    #[test]
    fn window_buttons_no_hover_outside_bar() {
        // Mouse y below bar height.
        let btns = window_buttons_layout(1200, 36, 46, (1170.0, 40.0));
        assert!(!btns[0].hovered);
        assert!(!btns[1].hovered);
        assert!(!btns[2].hovered);
    }

    #[test]
    fn window_buttons_positioning() {
        let btn_w = 46u32;
        let btns = window_buttons_layout(1200, 36, btn_w, (-1.0, -1.0));
        assert_eq!(btns[2].x, 1200 - btn_w);
        assert_eq!(btns[1].x, 1200 - btn_w * 2);
        assert_eq!(btns[0].x, 1200 - btn_w * 3);
    }

    // ── rename_selection_chars ────────────────────────────────────────

    #[test]
    fn rename_selection_none_when_no_selection() {
        assert_eq!(rename_selection_chars("hello", None, 10), None);
    }

    #[test]
    fn rename_selection_none_when_empty_range() {
        assert_eq!(rename_selection_chars("hello", Some((2, 2)), 10), None);
    }

    #[test]
    fn rename_selection_none_when_inverted_range() {
        assert_eq!(rename_selection_chars("hello", Some((4, 2)), 10), None);
    }

    #[test]
    fn rename_selection_ascii() {
        // "hello" — select bytes 1..4 = "ell" = chars 1..4
        let result = rename_selection_chars("hello", Some((1, 4)), 10);
        assert_eq!(result, Some((1, 4)));
    }

    #[test]
    fn rename_selection_clamped_to_max_chars() {
        let result = rename_selection_chars("hello", Some((0, 5)), 3);
        assert_eq!(result, Some((0, 3)));
    }

    #[test]
    fn rename_selection_multibyte() {
        // "aбв" — 'a' is 1 byte, 'б' is 2 bytes, 'в' is 2 bytes.
        // Byte offsets: a=0..1, б=1..3, в=3..5
        // Select bytes 1..5 = "бв" = chars 1..3
        let text = "aбв";
        let result = rename_selection_chars(text, Some((1, 5)), 10);
        assert_eq!(result, Some((1, 3)));
    }
}
