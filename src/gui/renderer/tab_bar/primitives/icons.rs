#![cfg_attr(target_os = "macos", allow(dead_code))]

use crate::core::Color;

impl super::super::super::CpuRenderer {
    pub(in crate::gui::renderer) fn draw_tab_plus_icon(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        rect: (u32, u32, u32, u32),
        color: Color,
    ) {
        let (x, _y, w, h) = rect;
        let center_x = x as f32 + w as f32 * 0.5;
        let center_y = rect.1 as f32 + h as f32 * 0.5;
        let half = (w.min(h) as f32 * 0.25).clamp(2.5, 5.0);
        let thickness = (1.25_f32 * self.ui_scale() as f32).clamp(1.15, 2.2);
        let pixel = color.to_pixel();

        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x - half,
            center_y,
            center_x + half,
            center_y,
            thickness,
            pixel,
        );
        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x,
            center_y - half,
            center_x,
            center_y + half,
            thickness,
            pixel,
        );
    }

    pub(in crate::gui::renderer) fn draw_tab_close_icon(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        rect: (u32, u32, u32, u32),
        color: Color,
    ) {
        let (x, y, w, h) = rect;
        let center_x = x as f32 + w as f32 * 0.5;
        let center_y = y as f32 + h as f32 * 0.5;
        let half = (w.min(h) as f32 * 0.22).clamp(2.5, 4.5);
        let thickness = (1.25_f32 * self.ui_scale() as f32).clamp(1.15, 2.2);
        let pixel = color.to_pixel();

        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x - half,
            center_y - half,
            center_x + half,
            center_y + half,
            thickness,
            pixel,
        );
        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x + half,
            center_y - half,
            center_x - half,
            center_y + half,
            thickness,
            pixel,
        );
    }
}
