#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::shared::{tab_math, ui_layout};
use super::super::traits::Renderer;
use super::super::types::RoundedRectCmd;

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
            self.push_rounded_rect_cmd(&RoundedRectCmd {
                x: px as f32,
                y: py as f32,
                w: pw as f32,
                h: ph as f32,
                radius: self.metrics.scaled_px(5) as f32,
                color: self.palette.inactive_tab_hover.to_pixel(),
                opacity: 1.0,
            });
        }
        let plus_fg = if plus_hover {
            self.palette.tab_text_active.to_pixel()
        } else {
            self.palette.tab_text_inactive.to_pixel()
        };
        let icon = ui_layout::compute_plus_icon_layout(plus_rect, self.metrics.ui_scale);
        let (x1, y1, x2, y2) = icon.h_line;
        self.push_line((x1, y1), (x2, y2), icon.thickness, plus_fg, 1.0);
        let (x1, y1, x2, y2) = icon.v_line;
        self.push_line((x1, y1), (x2, y2), icon.thickness, plus_fg, 1.0);
    }

    /// Draws the pin button at the left of the tab bar (non-macOS).
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_pin_button_commands(&mut self, mouse_pos: (f64, f64), pinned: bool) {
        let (pin_x, pin_y, pin_w, pin_h) = self.pin_button_rect();
        let is_hovered =
            tab_math::point_in_rect(mouse_pos.0, mouse_pos.1, (pin_x, pin_y, pin_w, pin_h));

        // Draw hover background.
        if is_hovered {
            self.push_rounded_rect_cmd(&RoundedRectCmd {
                x: pin_x as f32,
                y: pin_y as f32,
                w: pin_w as f32,
                h: pin_h as f32,
                radius: self.metrics.scaled_px(5) as f32,
                color: self.palette.inactive_tab_hover.to_pixel(),
                opacity: 1.0,
            });
        }

        let cx = pin_x as f32 + pin_w as f32 / 2.0;
        let cy = pin_y as f32 + pin_h as f32 / 2.0;
        let colors = super::super::types::PinColors {
            active: self.palette.pin_active_color.to_pixel(),
            hover: self.palette.tab_text_active.to_pixel(),
            inactive: self.palette.tab_text_inactive.to_pixel(),
        };
        let layout = ui_layout::pin_icon_layout(
            cx,
            cy,
            self.metrics.ui_scale as f32,
            pinned,
            is_hovered,
            &colors,
        );

        // Draw Bootstrap-style vertical pushpin icon from layout.
        let color = layout.color;
        self.push_rect(
            layout.head.0,
            layout.head.1,
            layout.head.2,
            layout.head.3,
            color,
            1.0,
        );
        self.push_rect(
            layout.body.0,
            layout.body.1,
            layout.body.2,
            layout.body.3,
            color,
            1.0,
        );
        self.push_rect(
            layout.platform.0,
            layout.platform.1,
            layout.platform.2,
            layout.platform.3,
            color,
            1.0,
        );
        let (x0, y0, x1, y1) = layout.needle;
        self.push_line((x0, y0), (x1, y1), layout.needle_thickness, color, 1.0);
    }

    /// Draws the settings gear button in the tab bar (non-macOS).
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_gear_button_commands(&mut self, mouse_pos: (f64, f64), settings_open: bool) {
        let (gx, gy, gw, gh) = self.gear_button_rect();
        let is_hovered = tab_math::point_in_rect(mouse_pos.0, mouse_pos.1, (gx, gy, gw, gh));

        // Hover/active background.
        if is_hovered || settings_open {
            let (bg, opacity) = if settings_open {
                (self.palette.active_accent.to_pixel(), 0.24)
            } else {
                (self.palette.inactive_tab_hover.to_pixel(), 1.0)
            };
            self.push_rounded_rect_cmd(&RoundedRectCmd {
                x: gx as f32,
                y: gy as f32,
                w: gw as f32,
                h: gh as f32,
                radius: self.metrics.scaled_px(5) as f32,
                color: bg,
                opacity,
            });
        }

        let icon_color = if is_hovered || settings_open {
            self.palette.tab_text_active.to_pixel()
        } else {
            self.palette.tab_text_inactive.to_pixel()
        };

        let icon_size = gw as f32;
        let cx = gx as f32 + gw as f32 / 2.0;
        let cy = gy as f32 + gh as f32 / 2.0;
        let layout = ui_layout::gear_icon_layout(cx, cy, icon_size, icon_color);

        // Draw teeth as filled rects.
        for &(tx, ty, tw, th) in &layout.teeth {
            self.push_rect(tx, ty, tw, th, icon_color, 1.0);
        }

        // Draw outer ring as a circle (rounded rect with radius = half size).
        let ring_size = layout.ring_outer_radius * 2.0;
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x: layout.ring_cx - layout.ring_outer_radius,
            y: layout.ring_cy - layout.ring_outer_radius,
            w: ring_size,
            h: ring_size,
            radius: layout.ring_outer_radius,
            color: icon_color,
            opacity: 1.0,
        });

        // Cut out inner ring with background color.
        let inner_size = layout.ring_inner_radius * 2.0;
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x: layout.ring_cx - layout.ring_inner_radius,
            y: layout.ring_cy - layout.ring_inner_radius,
            w: inner_size,
            h: inner_size,
            radius: layout.ring_inner_radius,
            color: self.palette.bar_bg.to_pixel(),
            opacity: 1.0,
        });

        // Cut out center hole with background color.
        let hole_size = layout.hole_radius * 2.0;
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x: layout.hole_cx - layout.hole_radius,
            y: layout.hole_cy - layout.hole_radius,
            w: hole_size,
            h: hole_size,
            radius: layout.hole_radius,
            color: self.palette.bar_bg.to_pixel(),
            opacity: 1.0,
        });
    }
}
