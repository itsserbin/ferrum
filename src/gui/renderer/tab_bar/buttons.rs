
use super::super::CpuRenderer;
#[cfg(not(target_os = "macos"))]
use super::super::traits::Renderer;

#[cfg(not(target_os = "macos"))]
use super::super::RoundedShape;
#[cfg(not(target_os = "macos"))]
use super::super::shared::{tab_math, ui_layout};
#[cfg(not(target_os = "macos"))]
use super::super::types::{PinColors, RenderTarget};

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
            self.draw_rounded_rect(
                target,
                &RoundedShape {
                    x: pin_x as i32,
                    y: pin_y as i32,
                    w: pin_w,
                    h: pin_h,
                    radius: self.scaled_px(5),
                    color: self.palette.inactive_tab_hover.to_pixel(),
                    alpha: 255,
                },
            );
        }

        let cx = pin_x as f32 + pin_w as f32 / 2.0;
        let cy = pin_y as f32 + pin_h as f32 / 2.0;
        let colors = PinColors {
            active: self.palette.pin_active_color.to_pixel(),
            hover: self.palette.tab_text_active.to_pixel(),
            inactive: self.palette.tab_text_inactive.to_pixel(),
        };
        let layout = ui_layout::pin_icon_layout(
            cx,
            cy,
            self.ui_scale() as f32,
            pinned,
            is_hovered,
            &colors,
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

        // Helper to draw filled rect from (x, y, w, h) in f32.
        let draw_rect = |buf: &mut [u32], bw: usize, bh: usize, r: (f32, f32, f32, f32)| {
            let rx = r.0 as i32;
            let ry = r.1 as i32;
            let rw = r.2 as i32;
            let rh = r.3 as i32;
            for py in ry.max(0)..(ry + rh).min(bh as i32) {
                for px in rx.max(0)..(rx + rw).min(bw as i32) {
                    let idx = py as usize * bw + px as usize;
                    if idx < buf.len() {
                        buf[idx] = color;
                    }
                }
            }
        };

        let bw = target.width;
        let bh = target.height;
        draw_rect(target.buffer, bw, bh, layout.head);
        draw_rect(target.buffer, bw, bh, layout.body);
        draw_rect(target.buffer, bw, bh, layout.platform);

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
            let (bg, alpha) = if settings_open {
                (self.palette.active_accent.to_pixel(), 60)
            } else {
                (self.palette.inactive_tab_hover.to_pixel(), 255)
            };
            self.draw_rounded_rect(
                target,
                &RoundedShape {
                    x: gx as i32,
                    y: gy as i32,
                    w: gw,
                    h: gh,
                    radius: self.scaled_px(5),
                    color: bg,
                    alpha,
                },
            );
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
        let outer_r = layout.ring_outer_radius;
        let inner_r = layout.ring_inner_radius;
        let cx = layout.ring_cx;
        let cy = layout.ring_cy;

        // Draw ring (filled annulus) with anti-aliased edges.
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

        // Draw teeth (filled rects).
        for &(tx, ty, tw, th) in &layout.teeth {
            let rx = tx as i32;
            let ry = ty as i32;
            let rw = tw as i32;
            let rh = th as i32;
            for py in ry.max(0)..(ry + rh).min(bh as i32) {
                for px in rx.max(0)..(rx + rw).min(bw as i32) {
                    let idx = py as usize * bw + px as usize;
                    if idx < target.buffer.len() {
                        target.buffer[idx] = color;
                    }
                }
            }
        }

        // Cut out center hole with anti-aliased edge.
        let hole_r = layout.hole_radius;
        let hole_color = self.palette.bar_bg.to_pixel();
        let h_min_x = (layout.hole_cx - hole_r - 1.0).max(0.0) as usize;
        let h_max_x = ((layout.hole_cx + hole_r + 1.0) as usize).min(bw);
        let h_min_y = (layout.hole_cy - hole_r - 1.0).max(0.0) as usize;
        let h_max_y = ((layout.hole_cy + hole_r + 1.0) as usize).min(bh);
        for py in h_min_y..h_max_y {
            for px in h_min_x..h_max_x {
                let dx = px as f32 + 0.5 - layout.hole_cx;
                let dy = py as f32 + 0.5 - layout.hole_cy;
                let dist = (dx * dx + dy * dy).sqrt();
                let coverage = (hole_r + 0.5 - dist).clamp(0.0, 1.0);
                if coverage <= 0.0 {
                    continue;
                }
                let idx = py * bw + px;
                if idx < target.buffer.len() {
                    let alpha = (coverage * 255.0).round() as u8;
                    target.buffer[idx] = crate::gui::renderer::blend_rgb(target.buffer[idx], hole_color, alpha);
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
