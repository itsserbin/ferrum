
use super::super::RenderTarget;
use super::super::shared::overlay_layout;
use super::super::traits::Renderer;

impl super::super::CpuRenderer {
    /// Draws a small tooltip with full tab title near the pointer.
    #[cfg(not(target_os = "macos"))]
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

        self.draw_overlay_box(target, layout.bg_x, layout.bg_y, layout.bg_w, layout.bg_h, layout.radius);

        self.draw_text_at(target, layout.text_x, layout.text_y, &layout.display_text, self.palette.default_fg);
    }
}
