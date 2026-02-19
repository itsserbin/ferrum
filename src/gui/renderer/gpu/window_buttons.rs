//! Window control button rendering for the GPU renderer (non-macOS).

#![cfg(not(target_os = "macos"))]

use super::super::shared::{tab_math, ui_layout};
use super::super::{
    CLOSE_HOVER_BG_COLOR, INACTIVE_TAB_HOVER, TAB_TEXT_ACTIVE, TAB_TEXT_INACTIVE,
};

impl super::GpuRenderer {
    pub(super) fn draw_window_buttons_commands(&mut self, buf_width: u32, mouse_pos: (f64, f64)) {
        let bar_h = self.metrics.tab_bar_height_px();
        let btn_w = self.metrics.scaled_px(tab_math::WIN_BTN_WIDTH);

        let buttons = ui_layout::window_buttons_layout(buf_width, bar_h, btn_w, mouse_pos);

        for btn in &buttons {
            if btn.hovered {
                let hover_bg = if btn.kind == ui_layout::WindowButtonKind::Close {
                    CLOSE_HOVER_BG_COLOR
                } else {
                    INACTIVE_TAB_HOVER
                };
                self.push_rect(btn.x as f32, 0.0, btn.w as f32, btn.h as f32, hover_bg, 1.0);
            }

            let icon_color =
                if btn.hovered && btn.kind == ui_layout::WindowButtonKind::Close {
                    TAB_TEXT_ACTIVE
                } else {
                    TAB_TEXT_INACTIVE
                };

            let center_x = btn.x as f32 + btn.w as f32 / 2.0;
            let center_y = btn.h as f32 / 2.0;
            let thickness = (1.25 * self.metrics.ui_scale as f32).clamp(1.15, 2.2);

            match btn.kind {
                ui_layout::WindowButtonKind::Minimize => {
                    self.push_minimize_icon(center_x, center_y, thickness, icon_color)
                }
                ui_layout::WindowButtonKind::Maximize => {
                    self.push_maximize_icon(center_x, center_y, thickness, icon_color)
                }
                ui_layout::WindowButtonKind::Close => {
                    self.push_close_icon(center_x, center_y, thickness, icon_color)
                }
            }
        }
    }

    fn push_minimize_icon(&mut self, cx: f32, cy: f32, thickness: f32, color: u32) {
        let half_w = self.metrics.scaled_px(5) as f32;
        self.push_line(cx - half_w, cy, cx + half_w, cy, thickness, color, 1.0);
    }

    fn push_maximize_icon(&mut self, cx: f32, cy: f32, thickness: f32, color: u32) {
        let half = self.metrics.scaled_px(5) as f32;
        let x0 = cx - half;
        let y0 = cy - half;
        let x1 = cx + half;
        let y1 = cy + half;
        self.push_line(x0, y0, x1, y0, thickness, color, 1.0);
        self.push_line(x0, y1, x1, y1, thickness, color, 1.0);
        self.push_line(x0, y0, x0, y1, thickness, color, 1.0);
        self.push_line(x1, y0, x1, y1, thickness, color, 1.0);
    }

    fn push_close_icon(&mut self, cx: f32, cy: f32, thickness: f32, color: u32) {
        let half = self.metrics.scaled_px(5) as f32 * 0.7;
        self.push_line(
            cx - half,
            cy - half,
            cx + half,
            cy + half,
            thickness,
            color,
            1.0,
        );
        self.push_line(
            cx + half,
            cy - half,
            cx - half,
            cy + half,
            thickness,
            color,
            1.0,
        );
    }
}
