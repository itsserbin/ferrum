#![cfg_attr(target_os = "macos", allow(dead_code))]

use crate::core::Color;
use crate::gui::renderer::CpuRenderer;
use crate::gui::renderer::shared::ui_layout;
use crate::gui::renderer::types::RenderTarget;

impl CpuRenderer {
    pub(in crate::gui::renderer) fn draw_tab_plus_icon(
        &self,
        target: &mut RenderTarget<'_>,
        rect: (u32, u32, u32, u32),
        color: Color,
    ) {
        let layout = ui_layout::compute_plus_icon_layout(rect, self.ui_scale());
        let pixel = color.to_pixel();

        let (x1, y1, x2, y2) = layout.h_line;
        Self::draw_stroked_line(target, (x1, y1), (x2, y2), layout.thickness, pixel);
        let (x1, y1, x2, y2) = layout.v_line;
        Self::draw_stroked_line(target, (x1, y1), (x2, y2), layout.thickness, pixel);
    }
}
