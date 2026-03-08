
use super::super::shared::{tab_math, ui_layout};
use super::super::traits::Renderer;
use super::super::types::RoundedRectCmd;

impl super::GpuRenderer {
    /// Draws a standard hover-highlight rounded rect for a tab-bar button.
    fn push_button_hover_bg(&mut self, x: u32, y: u32, w: u32, h: u32) {
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x: x as f32,
            y: y as f32,
            w: w as f32,
            h: h as f32,
            radius: self.metrics.scaled_px(5) as f32,
            color: self.palette.inactive_tab_hover.to_pixel(),
            opacity: 1.0,
        });
    }

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
            self.push_button_hover_bg(px, py, pw, ph);
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
            self.push_button_hover_bg(pin_x, pin_y, pin_w, pin_h);
        }

        let layout = ui_layout::compute_pin_button_layout(
            pin_x,
            pin_y,
            pin_w,
            pin_h,
            self.metrics.ui_scale as f32,
            pinned,
            is_hovered,
            self.palette.pin_active_color.to_pixel(),
            self.palette.tab_text_active.to_pixel(),
            self.palette.tab_text_inactive.to_pixel(),
        );

        // Draw Bootstrap-style vertical pushpin icon from layout.
        let color = layout.color;
        for &(x, y, w, h) in &[layout.head, layout.body, layout.platform] {
            self.push_rect(x, y, w, h, color, 1.0);
        }
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

        let icon_size = gw as f32 * 0.5;
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

        // Cut out inner ring and center hole with background color.
        let bar_bg = self.palette.bar_bg.to_pixel();
        for (cx, cy, r) in [
            (layout.ring_cx, layout.ring_cy, layout.ring_inner_radius),
            (layout.hole_cx, layout.hole_cy, layout.hole_radius),
        ] {
            let size = r * 2.0;
            self.push_rounded_rect_cmd(&RoundedRectCmd {
                x: cx - r,
                y: cy - r,
                w: size,
                h: size,
                radius: r,
                color: bar_bg,
                opacity: 1.0,
            });
        }
    }
}
