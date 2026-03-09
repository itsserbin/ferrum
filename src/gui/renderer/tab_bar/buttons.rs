
use super::super::CpuRenderer;
#[cfg(not(target_os = "macos"))]
use super::super::traits::Renderer;

#[cfg(not(target_os = "macos"))]
use super::super::RoundedShape;
#[cfg(not(target_os = "macos"))]
use super::super::shared::{tab_math, ui_layout};
#[cfg(not(target_os = "macos"))]
use super::super::types::RenderTarget;

/// Fills a rectangle defined by `(x, y, w, h)` in f32 pixels into a raw pixel buffer.
#[cfg(not(target_os = "macos"))]
fn fill_rect_f32(buf: &mut [u32], bw: usize, bh: usize, x: f32, y: f32, w: f32, h: f32, color: u32) {
    let rx = x as i32;
    let ry = y as i32;
    let rw = w as i32;
    let rh = h as i32;
    for py in ry.max(0)..(ry + rh).min(bh as i32) {
        for px in rx.max(0)..(rx + rw).min(bw as i32) {
            let idx = py as usize * bw + px as usize;
            if idx < buf.len() {
                buf[idx] = color;
            }
        }
    }
}

impl CpuRenderer {
    /// Draws the pin button at the left of the tab bar (non-macOS).
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_pin_button(
        &self,
        target: &mut RenderTarget<'_>,
        mouse_pos: (f64, f64),
        pinned: bool,
    ) {
        let (pin_x, pin_y, pin_w, pin_h) = self.pin_button_rect();
        let is_hovered =
            tab_math::point_in_rect(mouse_pos.0, mouse_pos.1, (pin_x, pin_y, pin_w, pin_h));

        // Draw hover background.
        if is_hovered {
            self.draw_button_hover_bg(target, pin_x, pin_y, pin_w, pin_h);
        }

        let layout = ui_layout::compute_pin_button_layout(
            pin_x,
            pin_y,
            pin_w,
            pin_h,
            self.ui_scale() as f32,
            pinned,
            is_hovered,
            self.palette.pin_active_color.to_pixel(),
            self.palette.tab_text_active.to_pixel(),
            self.palette.tab_text_inactive.to_pixel(),
        );

        self.draw_pin_icon(target, &layout);
    }

    /// Draws a Bootstrap-style vertical pushpin icon from pre-computed layout.
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_pin_icon(
        &self,
        target: &mut RenderTarget<'_>,
        layout: &ui_layout::PinIconLayout,
    ) {
        let color = layout.color;
        let bw = target.width;
        let bh = target.height;
        for &(x, y, w, h) in &[layout.head, layout.body, layout.platform] {
            fill_rect_f32(target.buffer, bw, bh, x, y, w, h, color);
        }

        // Needle (thin line).
        let (x0, y0, x1, y1) = layout.needle;
        Self::draw_stroked_line(target, (x0, y0), (x1, y1), layout.needle_thickness, color);
    }

    /// Draws the settings gear button in the tab bar (non-macOS).
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_gear_button(
        &self,
        target: &mut RenderTarget<'_>,
        mouse_pos: (f64, f64),
        settings_open: bool,
    ) {
        let (gx, gy, gw, gh) = self.gear_button_rect();
        let is_hovered = tab_math::point_in_rect(mouse_pos.0, mouse_pos.1, (gx, gy, gw, gh));

        // Hover/active background.
        if is_hovered || settings_open {
            if settings_open {
                self.draw_rounded_rect(
                    target,
                    &RoundedShape {
                        x: gx as i32,
                        y: gy as i32,
                        w: gw,
                        h: gh,
                        radius: self.scaled_px(5),
                        color: self.palette.active_accent.to_pixel(),
                        alpha: 60,
                    },
                );
            } else {
                self.draw_button_hover_bg(target, gx, gy, gw, gh);
            }
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
        self.draw_gear_icon(target, &layout);
    }

    /// Draws a gear icon from pre-computed layout: ring (annulus), teeth, and center hole.
    #[cfg(not(target_os = "macos"))]
    fn draw_gear_icon(&self, target: &mut RenderTarget<'_>, layout: &ui_layout::GearIconLayout) {
        let color = layout.color;
        let bw = target.width;
        let bh = target.height;

        // Draw ring (filled annulus) with anti-aliased edges.
        Self::draw_antialiased_annulus(
            target,
            layout.ring_cx,
            layout.ring_cy,
            layout.ring_inner_radius,
            layout.ring_outer_radius,
            color,
        );

        // Draw teeth (filled rects).
        for &(tx, ty, tw, th) in &layout.teeth {
            fill_rect_f32(target.buffer, bw, bh, tx, ty, tw, th, color);
        }

        // Cut out center hole with anti-aliased edge.
        let hole_color = self.palette.bar_bg.to_pixel();
        Self::draw_antialiased_circle(target, layout.hole_cx, layout.hole_cy, layout.hole_radius, hole_color);
    }

    /// Draws a filled anti-aliased circle onto the pixel buffer (fully opaque).
    #[cfg(not(target_os = "macos"))]
    fn draw_antialiased_circle(target: &mut RenderTarget<'_>, cx: f32, cy: f32, r: f32, color: u32) {
        // Delegate to the shared filled-circle primitive with full opacity.
        Self::draw_filled_circle(target, cx as i32, cy as i32, r as u32, color, 255);
    }

    /// Draws a filled anti-aliased annulus (ring) onto the pixel buffer.
    #[cfg(not(target_os = "macos"))]
    fn draw_antialiased_annulus(
        target: &mut RenderTarget<'_>,
        cx: f32,
        cy: f32,
        inner_r: f32,
        outer_r: f32,
        color: u32,
    ) {
        let bw = target.width;
        let bh = target.height;
        let min_x = (cx - outer_r - 1.0).max(0.0) as usize;
        let max_x = ((cx + outer_r + 1.0) as usize).min(bw);
        let min_y = (cy - outer_r - 1.0).max(0.0) as usize;
        let max_y = ((cy + outer_r + 1.0) as usize).min(bh);
        for py in min_y..max_y {
            for px in min_x..max_x {
                let dx = px as f32 + 0.5 - cx;
                let dy = py as f32 + 0.5 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                // Pixel is inside the ring if dist is between inner_r and outer_r.
                let outer_cov = (outer_r + 0.5 - dist).clamp(0.0, 1.0);
                let inner_cov = (dist - inner_r + 0.5).clamp(0.0, 1.0);
                let coverage = outer_cov * inner_cov;
                if coverage <= 0.0 {
                    continue;
                }
                let idx = py * bw + px;
                if idx < target.buffer.len() {
                    let alpha = (coverage * 255.0).round() as u8;
                    target.buffer[idx] = crate::gui::renderer::blend_rgb(target.buffer[idx], color, alpha);
                }
            }
        }
    }

    /// Draws the 3 window control buttons at the right edge (non-macOS).
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_window_buttons(
        &self,
        target: &mut RenderTarget<'_>,
        mouse_pos: (f64, f64),
    ) {
        let bar_h = self.tab_bar_height_px();
        let btn_w = self.scaled_px(tab_math::WIN_BTN_WIDTH);
        let half_w_px = self.scaled_px(5);

        let buttons =
            ui_layout::window_buttons_layout(target.width as u32, bar_h, btn_w, mouse_pos);

        for btn in &buttons {
            let colors = ui_layout::window_button_colors(
                btn.kind,
                btn.hovered,
                self.palette.inactive_tab_hover.to_pixel(),
                self.palette.win_btn_close_hover.to_pixel(),
                self.palette.tab_text_inactive.to_pixel(),
                0xFFFFFF,
            );

            // Hover background.
            if let Some(hover_bg) = colors.hover_bg {
                for py in 0..btn.h as usize {
                    for px in btn.x as usize..(btn.x + btn.w) as usize {
                        if px < target.width && py < target.height {
                            let idx = py * target.width + px;
                            if idx < target.buffer.len() {
                                target.buffer[idx] = hover_bg;
                            }
                        }
                    }
                }
            }

            let icon = ui_layout::compute_window_button_icon_lines(btn, self.ui_scale(), half_w_px);

            for &(x1, y1, x2, y2) in &icon.lines {
                Self::draw_stroked_line(
                    target,
                    (x1, y1),
                    (x2, y2),
                    icon.thickness,
                    colors.icon_color,
                );
            }
        }
    }
}
