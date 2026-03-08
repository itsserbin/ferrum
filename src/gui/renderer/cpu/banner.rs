use super::super::shared::banner_layout::UpdateBannerLayout;
use super::super::types::RenderTarget;
use super::CpuRenderer;

impl CpuRenderer {
    /// Draws the update-available banner overlay using the CPU renderer.
    pub fn draw_update_banner(
        &mut self,
        target: &mut RenderTarget<'_>,
        layout: &UpdateBannerLayout,
    ) {
        self.draw_banner_bg(target, layout);
        self.draw_banner_labels(target, layout);
    }

    /// Draws the banner background (and border) rounded rect.
    ///
    /// On non-macOS, uses `draw_rounded_rect` with palette tab colors.
    /// On macOS, writes directly to the pixel buffer using `default_bg`.
    fn draw_banner_bg(&self, target: &mut RenderTarget<'_>, layout: &UpdateBannerLayout) {
        #[cfg(not(target_os = "macos"))]
        self.draw_overlay_box(target, layout.bg_x, layout.bg_y, layout.bg_w, layout.bg_h, layout.radius);

        #[cfg(target_os = "macos")]
        {
            let bg_pixel = self.palette.default_bg.to_pixel();
            let x0 = layout.bg_x.max(0) as usize;
            let y0 = layout.bg_y.max(0) as usize;
            let x1 = (layout.bg_x + layout.bg_w as i32).clamp(0, target.width as i32) as usize;
            let y1 = (layout.bg_y + layout.bg_h as i32).clamp(0, target.height as i32) as usize;
            for py in y0..y1 {
                for px in x0..x1 {
                    let idx = py * target.width + px;
                    if idx < target.buffer.len() {
                        target.buffer[idx] = bg_pixel;
                    }
                }
            }
        }
    }

    /// Draws the banner button backgrounds (non-macOS only).
    #[cfg(not(target_os = "macos"))]
    fn draw_banner_button_bg(
        &self,
        target: &mut RenderTarget<'_>,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        radius: u32,
    ) {
        use super::super::RoundedShape;

        self.draw_rounded_rect(
            target,
            &RoundedShape {
                x: x as i32,
                y: y as i32,
                w,
                h,
                radius,
                color: self.palette.tab_border.to_pixel(),
                alpha: 120,
            },
        );
    }

    /// Draws the label text and button labels for the banner.
    fn draw_banner_labels(&mut self, target: &mut RenderTarget<'_>, layout: &UpdateBannerLayout) {
        // Label text.
        let (label_text, label_x, label_y) = layout.label();
        self.draw_text_at(target, label_x, label_y, label_text, self.palette.default_fg);

        if layout.installing {
            return;
        }

        let t = crate::i18n::t();
        let cell_height = self.metrics.cell_height;
        let cell_width = self.metrics.cell_width;
        let (_, _, _, btn_h) = layout.details_rect();

        let buttons = [
            (layout.details_rect(), t.update_details),
            (layout.install_rect(), t.update_install),
            (layout.dismiss_rect(), "✕"),
        ];

        for ((bx, by, bw, _), text) in buttons {
            #[cfg(not(target_os = "macos"))]
            self.draw_banner_button_bg(target, bx, by, bw, btn_h, layout.radius);
            let (tx, ty) = crate::gui::renderer::shared::centered_button_text_origin(
                bx, by, bw, btn_h, text, cell_width, cell_height,
            );
            self.draw_text_at(target, tx, ty, text, self.palette.default_fg);
        }
    }
}
