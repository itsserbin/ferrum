#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::shared::tab_math;
use super::super::{INACTIVE_TAB_HOVER, TAB_TEXT_ACTIVE, TAB_TEXT_INACTIVE};
#[cfg(not(target_os = "macos"))]
use super::super::PIN_ACTIVE_COLOR;

impl super::GpuRenderer {
    /// Draws the new-tab (+) button with hover highlight.
    pub(super) fn plus_button_commands(
        &mut self,
        tab_count: usize,
        tw: u32,
        mouse_pos: (f64, f64),
    ) {
        let plus_rect = self.plus_button_rect(tab_count, tw);
        let plus_hover = tab_math::point_in_rect(mouse_pos.0, mouse_pos.1, plus_rect);
        if plus_hover {
            let (px, py, pw, ph) = plus_rect;
            self.push_rounded_rect(
                px as f32,
                py as f32,
                pw as f32,
                ph as f32,
                self.metrics.scaled_px(5) as f32,
                INACTIVE_TAB_HOVER,
                1.0,
            );
        }
        let plus_fg = if plus_hover {
            TAB_TEXT_ACTIVE
        } else {
            TAB_TEXT_INACTIVE
        };
        let (px, py, pw, ph) = plus_rect;
        let center_x = px as f32 + pw as f32 * 0.5;
        let center_y = py as f32 + ph as f32 * 0.5;
        let half = (pw.min(ph) as f32 * 0.25).clamp(2.5, 5.0);
        let thickness = (1.25 * self.metrics.ui_scale as f32).clamp(1.15, 2.2);
        self.push_line(
            center_x - half,
            center_y,
            center_x + half,
            center_y,
            thickness,
            plus_fg,
            1.0,
        );
        self.push_line(
            center_x,
            center_y - half,
            center_x,
            center_y + half,
            thickness,
            plus_fg,
            1.0,
        );
    }

    /// Draws the pin button at the left of the tab bar (non-macOS).
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_pin_button_commands(&mut self, mouse_pos: (f64, f64), pinned: bool) {
        let (pin_x, pin_y, pin_w, pin_h) = self.pin_button_rect();
        let is_hovered =
            tab_math::point_in_rect(mouse_pos.0, mouse_pos.1, (pin_x, pin_y, pin_w, pin_h));

        // Draw hover background.
        if is_hovered {
            self.push_rounded_rect(
                pin_x as f32,
                pin_y as f32,
                pin_w as f32,
                pin_h as f32,
                self.metrics.scaled_px(5) as f32,
                INACTIVE_TAB_HOVER,
                1.0,
            );
        }

        // Icon color: active (lavender) when pinned, inactive otherwise.
        let icon_color = if pinned {
            PIN_ACTIVE_COLOR
        } else if is_hovered {
            TAB_TEXT_ACTIVE
        } else {
            TAB_TEXT_INACTIVE
        };

        // Draw Bootstrap-style vertical pushpin icon.
        let cx = pin_x as f32 + pin_w as f32 / 2.0;
        let cy = pin_y as f32 + pin_h as f32 / 2.0;
        let s = self.metrics.ui_scale as f32;
        let t = (1.2 * s).clamp(1.0, 2.0);

        // Dimensions (scaled).
        let head_w = 6.0 * s;
        let head_h = 2.0 * s;
        let body_w = 3.0 * s;
        let body_h = 4.0 * s;
        let platform_w = 7.0 * s;
        let platform_h = 1.5 * s;
        let needle_h = 4.0 * s;

        let top = cy - 6.0 * s;

        // 1. Top head (wide rectangle).
        self.push_rect(cx - head_w / 2.0, top, head_w, head_h, icon_color, 1.0);

        // 2. Body (narrower rectangle below head).
        let body_top = top + head_h;
        self.push_rect(cx - body_w / 2.0, body_top, body_w, body_h, icon_color, 1.0);

        // 3. Platform/base (wider rectangle where pin enters surface).
        let platform_top = body_top + body_h;
        self.push_rect(
            cx - platform_w / 2.0,
            platform_top,
            platform_w,
            platform_h,
            icon_color,
            1.0,
        );

        // 4. Needle (thin line pointing down).
        let needle_top = platform_top + platform_h;
        self.push_line(
            cx,
            needle_top,
            cx,
            needle_top + needle_h,
            t,
            icon_color,
            1.0,
        );
    }
}
