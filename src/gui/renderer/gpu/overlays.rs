
#[cfg(not(target_os = "macos"))]
use super::super::shared::overlay_layout;
#[cfg(not(target_os = "macos"))]
use super::super::traits::Renderer;
#[cfg(not(target_os = "macos"))]
use super::super::types::DragPosition;
use super::super::types::RoundedRectCmd;
#[cfg(not(target_os = "macos"))]
use super::super::TabInfo;

impl super::GpuRenderer {
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_tab_drag_overlay_impl(
        &mut self,
        buf_width: usize,
        tabs: &[TabInfo],
        source_index: usize,
        current_x: f64,
        indicator_x: f32,
    ) {
        let m = self.tab_layout_metrics();
        let drag_pos = DragPosition { current_x, indicator_x };
        let layout = match overlay_layout::compute_drag_overlay_layout(
            &m,
            tabs.len(),
            source_index,
            tabs[source_index].title,
            &drag_pos,
            buf_width as u32,
        ) {
            Some(l) => l,
            None => return,
        };

        // Shadow.
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x: layout.shadow_x as f32,
            y: layout.shadow_y as f32,
            w: layout.rect_w as f32,
            h: layout.rect_h as f32,
            radius: layout.radius as f32,
            color: 0x000000,
            opacity: 0.24,
        });
        // Body.
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x: layout.body_x as f32,
            y: layout.body_y as f32,
            w: layout.rect_w as f32,
            h: layout.rect_h as f32,
            radius: layout.radius as f32,
            color: self.palette.active_tab_bg.to_pixel(),
            opacity: 0.86,
        });
        // Border.
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x: layout.body_x as f32,
            y: layout.body_y as f32,
            w: layout.rect_w as f32,
            h: layout.rect_h as f32,
            radius: layout.radius as f32,
            color: self.palette.tab_border.to_pixel(),
            opacity: 0.39,
        });

        // Ghost title.
        self.push_text(
            layout.title_x as f32,
            layout.title_y as f32,
            &layout.title_text,
            self.palette.tab_text_active.to_pixel(),
            1.0,
        );

        // Smooth insertion indicator at lerped position.
        self.push_rect(
            layout.indicator_x as f32,
            layout.indicator_y as f32,
            layout.indicator_w as f32,
            layout.indicator_h as f32,
            self.palette.insertion_color.to_pixel(),
            1.0,
        );
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_tab_tooltip_impl(
        &mut self,
        buf_width: usize,
        buf_height: usize,
        mouse_pos: (f64, f64),
        title: &str,
    ) {
        let m = self.tab_layout_metrics();
        let layout = match overlay_layout::compute_tooltip_layout(
            title,
            mouse_pos,
            &m,
            buf_width as u32,
            buf_height as u32,
        ) {
            Some(l) => l,
            None => return,
        };

        let (x, y) = (layout.bg_x as f32, layout.bg_y as f32);
        let (w, h) = (layout.bg_w as f32, layout.bg_h as f32);
        let r = layout.radius as f32;
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x, y, w, h, radius: r, color: self.palette.active_tab_bg.to_pixel(), opacity: 0.96,
        });
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x, y, w, h, radius: r, color: self.palette.tab_border.to_pixel(), opacity: 0.31,
        });

        self.push_text(
            layout.text_x as f32,
            layout.text_y as f32,
            &layout.display_text,
            self.palette.tab_text_active.to_pixel(),
            1.0,
        );
    }
}

impl super::GpuRenderer {
    pub(super) fn draw_update_banner_impl(
        &mut self,
        layout: &super::super::shared::banner_layout::UpdateBannerLayout,
    ) {
        let x = layout.bg_x as f32;
        let y = layout.bg_y as f32;
        let w = layout.bg_w as f32;
        let h = layout.bg_h as f32;
        let r = layout.radius as f32;

        // Background fill.
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x, y, w, h, radius: r, color: self.palette.active_tab_bg.to_pixel(), opacity: 0.96,
        });
        // Subtle border.
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x, y, w, h, radius: r, color: self.palette.tab_border.to_pixel(), opacity: 0.31,
        });

        // Label text.
        let (label_text, label_x, label_y) = layout.label();
        self.push_text(
            label_x as f32,
            label_y as f32,
            label_text,
            self.palette.tab_text_active.to_pixel(),
            1.0,
        );

        if layout.installing {
            return;
        }

        let t = crate::i18n::t();

        // [Details] button background.
        let (details_x, details_y, details_w, btn_h) = layout.details_rect();
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x: details_x as f32,
            y: details_y as f32,
            w: details_w as f32,
            h: btn_h as f32,
            radius: r,
            color: self.palette.tab_border.to_pixel(),
            opacity: 0.47,
        });
        let details_text = t.update_details;
        let details_text_w = details_text.chars().count() as u32 * self.metrics.cell_width;
        let details_text_x = details_x + (details_w.saturating_sub(details_text_w)) / 2;
        let details_text_y = details_y + (btn_h.saturating_sub(self.metrics.cell_height)) / 2;
        self.push_text(
            details_text_x as f32,
            details_text_y as f32,
            details_text,
            self.palette.tab_text_active.to_pixel(),
            1.0,
        );

        // [Install] button background.
        let (install_x, install_y, install_w, _) = layout.install_rect();
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x: install_x as f32,
            y: install_y as f32,
            w: install_w as f32,
            h: btn_h as f32,
            radius: r,
            color: self.palette.tab_border.to_pixel(),
            opacity: 0.47,
        });
        let install_text = t.update_install;
        let install_text_w = install_text.chars().count() as u32 * self.metrics.cell_width;
        let install_text_x = install_x + (install_w.saturating_sub(install_text_w)) / 2;
        let install_text_y = install_y + (btn_h.saturating_sub(self.metrics.cell_height)) / 2;
        self.push_text(
            install_text_x as f32,
            install_text_y as f32,
            install_text,
            self.palette.tab_text_active.to_pixel(),
            1.0,
        );

        // [✕] dismiss button background.
        let (dismiss_x, dismiss_y, dismiss_w, _) = layout.dismiss_rect();
        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x: dismiss_x as f32,
            y: dismiss_y as f32,
            w: dismiss_w as f32,
            h: btn_h as f32,
            radius: r,
            color: self.palette.tab_border.to_pixel(),
            opacity: 0.47,
        });
        let dismiss_text = "✕";
        let dismiss_text_w = dismiss_text.chars().count() as u32 * self.metrics.cell_width;
        let dismiss_text_x = dismiss_x + (dismiss_w.saturating_sub(dismiss_text_w)) / 2;
        let dismiss_text_y = dismiss_y + (btn_h.saturating_sub(self.metrics.cell_height)) / 2;
        self.push_text(
            dismiss_text_x as f32,
            dismiss_text_y as f32,
            dismiss_text,
            self.palette.tab_text_active.to_pixel(),
            1.0,
        );
    }
}
