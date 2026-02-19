#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::shared::tab_math;
#[cfg(not(target_os = "macos"))]
use super::super::shared::ui_layout;
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

        let cx = pin_x as f32 + pin_w as f32 / 2.0;
        let cy = pin_y as f32 + pin_h as f32 / 2.0;
        let layout = ui_layout::pin_icon_layout(
            cx,
            cy,
            self.metrics.ui_scale as f32,
            pinned,
            is_hovered,
            PIN_ACTIVE_COLOR,
            TAB_TEXT_ACTIVE,
            TAB_TEXT_INACTIVE,
        );

        // Draw Bootstrap-style vertical pushpin icon from layout.
        let color = layout.color;
        self.push_rect(layout.head.0, layout.head.1, layout.head.2, layout.head.3, color, 1.0);
        self.push_rect(layout.body.0, layout.body.1, layout.body.2, layout.body.3, color, 1.0);
        self.push_rect(
            layout.platform.0,
            layout.platform.1,
            layout.platform.2,
            layout.platform.3,
            color,
            1.0,
        );
        self.push_line(
            layout.needle.0,
            layout.needle.1,
            layout.needle.2,
            layout.needle.3,
            layout.needle_thickness,
            color,
            1.0,
        );
    }
}
