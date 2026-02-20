#![cfg_attr(target_os = "macos", allow(dead_code))]

use crate::gui::renderer::CpuRenderer;
use crate::gui::renderer::RoundedShape;
use crate::gui::renderer::types::RenderTarget;

impl CpuRenderer {
    /// Draws a rounded rect with only the top corners rounded (bottom corners square).
    /// Used for active/hovered tab shapes that merge with the terminal below.
    pub(in crate::gui::renderer) fn draw_top_rounded_rect(
        &self,
        target: &mut RenderTarget<'_>,
        shape: &RoundedShape,
    ) {
        Self::draw_rounded_impl(target, shape, Self::top_rounded_coverage);
    }

    /// Coverage function for a rect with only the top two corners rounded.
    pub(in crate::gui::renderer) fn top_rounded_coverage(
        px: i32,
        py: i32,
        w: i32,
        h: i32,
        r: i32,
    ) -> f32 {
        if px < 0 || py < 0 || px >= w || py >= h {
            return 0.0;
        }
        if r <= 0 {
            return 1.0;
        }

        let in_tl = px < r && py < r;
        let in_tr = px >= w - r && py < r;
        // Bottom corners are NOT rounded.
        if !(in_tl || in_tr) {
            return 1.0;
        }

        let cx = if in_tl {
            r as f32 - 0.5
        } else {
            (w - r) as f32 - 0.5
        };
        let cy = r as f32 - 0.5;

        let dx = px as f32 + 0.5 - cx;
        let dy = py as f32 + 0.5 - cy;
        let rr = r as f32;
        let dist = (dx * dx + dy * dy).sqrt();
        (rr + 0.5 - dist).clamp(0.0, 1.0)
    }
}
