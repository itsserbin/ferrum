//! Window control button rendering for the GPU renderer (non-macOS).

#![cfg(not(target_os = "macos"))]

use super::super::shared::{tab_math, ui_layout};

impl super::GpuRenderer {
    pub(super) fn draw_window_buttons_commands(&mut self, buf_width: u32, mouse_pos: (f64, f64)) {
        let bar_h = self.metrics.tab_bar_height_px();
        let btn_w = self.metrics.scaled_px(tab_math::WIN_BTN_WIDTH);
        let half_w_px = self.metrics.scaled_px(5);

        let buttons = ui_layout::window_buttons_layout(buf_width, bar_h, btn_w, mouse_pos);

        for btn in &buttons {
            let colors = ui_layout::window_button_colors(
                btn.kind,
                btn.hovered,
                self.palette.inactive_tab_hover.to_pixel(),
                self.palette.win_btn_close_hover.to_pixel(),
                self.palette.tab_text_inactive.to_pixel(),
                0xFFFFFF,
            );

            if let Some(hover_bg) = colors.hover_bg {
                self.push_rect(btn.x as f32, 0.0, btn.w as f32, btn.h as f32, hover_bg, 1.0);
            }

            let icon =
                ui_layout::compute_window_button_icon_lines(btn, self.metrics.ui_scale, half_w_px);

            for &(x1, y1, x2, y2) in &icon.lines {
                self.push_line((x1, y1), (x2, y2), icon.thickness, colors.icon_color, 1.0);
            }
        }
    }
}
