use crate::gui::settings::SettingsOverlay;
use crate::gui::settings::layout::{compute_settings_layout, ItemControlLayout};

impl super::GpuRenderer {
    /// Draws the settings overlay via GPU command accumulation.
    pub(super) fn draw_settings_overlay_impl(
        &mut self,
        buf_width: usize,
        buf_height: usize,
        overlay: &SettingsOverlay,
    ) {
        let layout = compute_settings_layout(
            buf_width as u32,
            buf_height as u32,
            self.metrics.cell_width,
            self.metrics.cell_height,
            self.metrics.ui_scale,
            overlay,
            self.palette.menu_bg.to_pixel(),
            self.palette.active_accent.to_pixel(),
            self.palette.tab_text_active.to_pixel(),
            self.palette.tab_text_inactive.to_pixel(),
            self.palette.bar_bg.to_pixel(),
        );

        // Dim background.
        self.push_rect(
            layout.dim_bg.x,
            layout.dim_bg.y,
            layout.dim_bg.w,
            layout.dim_bg.h,
            layout.dim_bg.color,
            layout.dim_bg.opacity,
        );

        // Panel.
        self.push_rounded_rect_cmd(&layout.panel_bg);
        self.push_rounded_rect_cmd(&layout.panel_border);

        // Title.
        self.push_text(
            layout.title.x, layout.title.y,
            &layout.title.text, layout.title.color, layout.title.opacity,
        );

        // Title separator.
        self.push_rect(
            layout.title_separator.x, layout.title_separator.y,
            layout.title_separator.w, layout.title_separator.h,
            layout.title_separator.color, layout.title_separator.opacity,
        );

        // Sidebar separator.
        self.push_rect(
            layout.sidebar_separator.x, layout.sidebar_separator.y,
            layout.sidebar_separator.w, layout.sidebar_separator.h,
            layout.sidebar_separator.color, layout.sidebar_separator.opacity,
        );

        // Categories.
        for cat in &layout.categories {
            if cat.bg.opacity > 0.0 {
                self.push_rect(cat.bg.x, cat.bg.y, cat.bg.w, cat.bg.h, cat.bg.color, cat.bg.opacity);
            }
            self.push_text(cat.text.x, cat.text.y, &cat.text.text, cat.text.color, cat.text.opacity);
        }

        // Items (non-dropdown parts first).
        for item in &layout.items {
            self.push_text(
                item.label.x, item.label.y,
                &item.label.text, item.label.color, item.label.opacity,
            );
            match &item.controls {
                ItemControlLayout::Stepper {
                    minus_btn,
                    minus_text,
                    value_text,
                    plus_btn,
                    plus_text,
                } => {
                    self.push_rounded_rect_cmd(minus_btn);
                    self.push_text(
                        minus_text.x, minus_text.y,
                        &minus_text.text, minus_text.color, minus_text.opacity,
                    );
                    self.push_text(
                        value_text.x, value_text.y,
                        &value_text.text, value_text.color, value_text.opacity,
                    );
                    self.push_rounded_rect_cmd(plus_btn);
                    self.push_text(
                        plus_text.x, plus_text.y,
                        &plus_text.text, plus_text.color, plus_text.opacity,
                    );
                }
                ItemControlLayout::Dropdown {
                    button,
                    button_text,
                    arrow_text,
                    ..
                } => {
                    self.push_rounded_rect_cmd(button);
                    self.push_text(
                        button_text.x, button_text.y,
                        &button_text.text, button_text.color, button_text.opacity,
                    );
                    self.push_text(
                        arrow_text.x, arrow_text.y,
                        &arrow_text.text, arrow_text.color, arrow_text.opacity,
                    );
                }
            }
        }

        // Dropdown options (drawn last, on top of everything).
        for item in &layout.items {
            if let ItemControlLayout::Dropdown { options, .. } = &item.controls {
                for opt in options {
                    self.push_rect(
                        opt.bg.x, opt.bg.y, opt.bg.w, opt.bg.h,
                        opt.bg.color, opt.bg.opacity,
                    );
                    self.push_text(
                        opt.text.x, opt.text.y,
                        &opt.text.text, opt.text.color, opt.text.opacity,
                    );
                }
            }
        }

        // Close hint.
        self.push_text(
            layout.close_hint.x, layout.close_hint.y,
            &layout.close_hint.text, layout.close_hint.color, layout.close_hint.opacity,
        );

        // Close button (X).
        self.push_rounded_rect_cmd(&layout.close_button);
        let (ax0, ay0, ax1, ay1) = layout.close_icon_line_a;
        let (bx0, by0, bx1, by1) = layout.close_icon_line_b;
        self.push_line((ax0, ay0), (ax1, ay1), 1.5, layout.close_icon_color, 1.0);
        self.push_line((bx0, by0), (bx1, by1), 1.5, layout.close_icon_color, 1.0);
    }
}
