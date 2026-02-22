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
        {
            use super::super::RoundedShape;

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
        }

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
        for (ci, ch) in label_text.chars().enumerate() {
            let cx = label_x + ci as u32 * self.metrics.cell_width;
            self.draw_char(target, cx, label_y, ch, self.palette.default_fg);
        }

        if layout.installing {
            return;
        }

        let t = crate::i18n::t();
        let cell_height = self.metrics.cell_height;

        // [Details] button.
        let (details_x, details_y, details_w, btn_h) = layout.details_rect();
        #[cfg(not(target_os = "macos"))]
        self.draw_banner_button_bg(target, details_x, details_y, details_w, btn_h, layout.radius);
        let details_text = t.update_details;
        let details_text_w = details_text.chars().count() as u32 * self.metrics.cell_width;
        let details_text_x = details_x + (details_w.saturating_sub(details_text_w)) / 2;
        let details_text_y = details_y + (btn_h.saturating_sub(cell_height)) / 2;
        for (ci, ch) in details_text.chars().enumerate() {
            let cx = details_text_x + ci as u32 * self.metrics.cell_width;
            self.draw_char(target, cx, details_text_y, ch, self.palette.default_fg);
        }

        // [Install] button.
        let (install_x, install_y, install_w, _) = layout.install_rect();
        #[cfg(not(target_os = "macos"))]
        self.draw_banner_button_bg(target, install_x, install_y, install_w, btn_h, layout.radius);
        let install_text = t.update_install;
        let install_text_w = install_text.chars().count() as u32 * self.metrics.cell_width;
        let install_text_x = install_x + (install_w.saturating_sub(install_text_w)) / 2;
        let install_text_y = install_y + (btn_h.saturating_sub(cell_height)) / 2;
        for (ci, ch) in install_text.chars().enumerate() {
            let cx = install_text_x + ci as u32 * self.metrics.cell_width;
            self.draw_char(target, cx, install_text_y, ch, self.palette.default_fg);
        }

        // [✕] dismiss button.
        let (dismiss_x, dismiss_y, dismiss_w, _) = layout.dismiss_rect();
        #[cfg(not(target_os = "macos"))]
        self.draw_banner_button_bg(
            target,
            dismiss_x,
            dismiss_y,
            dismiss_w,
            btn_h,
            layout.radius,
        );
        let dismiss_text = "✕";
        let dismiss_text_w = dismiss_text.chars().count() as u32 * self.metrics.cell_width;
        let dismiss_text_x = dismiss_x + (dismiss_w.saturating_sub(dismiss_text_w)) / 2;
        let dismiss_text_y = dismiss_y + (btn_h.saturating_sub(cell_height)) / 2;
        for (ci, ch) in dismiss_text.chars().enumerate() {
            let cx = dismiss_text_x + ci as u32 * self.metrics.cell_width;
            self.draw_char(target, cx, dismiss_text_y, ch, self.palette.default_fg);
        }
    }
}
