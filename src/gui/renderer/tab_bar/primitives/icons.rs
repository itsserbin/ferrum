#![cfg_attr(target_os = "macos", allow(dead_code))]

use crate::core::Color;
use crate::gui::renderer::shared::ui_layout;

impl super::super::super::CpuRenderer {
    pub(in crate::gui::renderer) fn draw_tab_plus_icon(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        rect: (u32, u32, u32, u32),
        color: Color,
    ) {
        let layout = ui_layout::compute_plus_icon_layout(rect, self.ui_scale());
        let pixel = color.to_pixel();

        let (x1, y1, x2, y2) = layout.h_line;
        Self::draw_stroked_line(
            buffer, buf_width, buf_height, x1, y1, x2, y2, layout.thickness, pixel,
        );
        let (x1, y1, x2, y2) = layout.v_line;
        Self::draw_stroked_line(
            buffer, buf_width, buf_height, x1, y1, x2, y2, layout.thickness, pixel,
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
        let thickness = ui_layout::icon_stroke_thickness(self.ui_scale());
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
