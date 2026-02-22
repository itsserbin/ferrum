
use super::super::RenderTarget;
use super::super::RoundedShape;
use super::super::shared::overlay_layout;
use super::super::traits::Renderer;

impl super::super::CpuRenderer {
    /// Draws a small tooltip with full tab title near the pointer.
    pub fn draw_tab_tooltip(
        &mut self,
        target: &mut RenderTarget<'_>,
        mouse_pos: (f64, f64),
        title: &str,
    ) {
        let m = self.tab_layout_metrics();
        let layout = match overlay_layout::compute_tooltip_layout(
            title,
            mouse_pos,
            &m,
            target.width as u32,
            target.height as u32,
        ) {
            Some(l) => l,
            None => return,
        };

        // Background fill.
        self.draw_rounded_rect(
            target,
            &RoundedShape {
                x: layout.bg_x,
                y: layout.bg_y,
                w: layout.bg_w,
                h: layout.bg_h,
                radius: layout.radius,
                color: self.palette.active_tab_bg.to_pixel(),
                alpha: 245,
            },
        );
        // Subtle border.
        self.draw_rounded_rect(
            target,
            &RoundedShape {
                x: layout.bg_x,
                y: layout.bg_y,
                w: layout.bg_w,
                h: layout.bg_h,
                radius: layout.radius,
                color: self.palette.tab_border.to_pixel(),
                alpha: 80,
            },
        );

        for (ci, ch) in layout.display_text.chars().enumerate() {
            let cx = layout.text_x + ci as u32 * self.metrics.cell_width;
            self.draw_char(target, cx, layout.text_y, ch, self.palette.default_fg);
        }
    }
}
