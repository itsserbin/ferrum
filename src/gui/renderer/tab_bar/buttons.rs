#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::CpuRenderer;
#[cfg(not(target_os = "macos"))]
use super::super::traits::Renderer;

#[cfg(not(target_os = "macos"))]
use super::super::RoundedShape;
#[cfg(not(target_os = "macos"))]
use super::super::shared::{tab_math, ui_layout};
#[cfg(not(target_os = "macos"))]
use super::super::types::{PinColors, RenderTarget};
#[cfg(not(target_os = "macos"))]
use super::{INACTIVE_TAB_HOVER, PIN_ACTIVE_COLOR, TAB_TEXT_ACTIVE, TAB_TEXT_INACTIVE};

#[cfg(not(target_os = "macos"))]
use super::WIN_BTN_CLOSE_HOVER;

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
                    color: INACTIVE_TAB_HOVER,
                    alpha: 255,
                },
            );
        }

        let cx = pin_x as f32 + pin_w as f32 / 2.0;
        let cy = pin_y as f32 + pin_h as f32 / 2.0;
        let colors = PinColors {
            active: PIN_ACTIVE_COLOR,
            hover: TAB_TEXT_ACTIVE,
            inactive: TAB_TEXT_INACTIVE,
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
                INACTIVE_TAB_HOVER,
                WIN_BTN_CLOSE_HOVER,
                TAB_TEXT_INACTIVE,
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
