use super::RenderTarget;
use crate::core::Color;
use crate::gui::settings::SettingsOverlay;
use crate::gui::settings::layout::{compute_settings_layout, ItemControlLayout};

impl super::CpuRenderer {
    /// Draws the settings overlay panel using shared layout computation.
    pub fn draw_settings_overlay(&mut self, target: &mut RenderTarget<'_>, overlay: &SettingsOverlay) {
        let layout = compute_settings_layout(
            target.width as u32,
            target.height as u32,
            self.metrics.cell_width,
            self.metrics.cell_height,
            self.ui_scale(),
            overlay,
            self.palette.menu_bg.to_pixel(),
            self.palette.active_accent.to_pixel(),
            self.palette.tab_text_active.to_pixel(),
            self.palette.tab_text_inactive.to_pixel(),
            self.palette.bar_bg.to_pixel(),
        );

        // Dim background.
        self.draw_flat_rect_cmd(target, &layout.dim_bg);

        // Panel.
        self.draw_rounded_rect_cmd(target, &layout.panel_bg);
        self.draw_rounded_rect_cmd(target, &layout.panel_border);

        // Title.
        self.draw_text_cmd(target, &layout.title);

        // Title separator.
        self.draw_flat_rect_cmd(target, &layout.title_separator);

        // Sidebar separator.
        self.draw_flat_rect_cmd(target, &layout.sidebar_separator);

        // Categories.
        for cat in &layout.categories {
            if cat.bg.opacity > 0.0 {
                self.draw_flat_rect_cmd(target, &cat.bg);
            }
            self.draw_text_cmd(target, &cat.text);
        }

        // Items (non-dropdown parts first).
        for item in &layout.items {
            self.draw_text_cmd(target, &item.label);
            match &item.controls {
                ItemControlLayout::Stepper {
                    minus_btn,
                    minus_text,
                    value_text,
                    plus_btn,
                    plus_text,
                } => {
                    self.draw_rounded_rect_cmd(target, minus_btn);
                    self.draw_text_cmd(target, minus_text);
                    self.draw_text_cmd(target, value_text);
                    self.draw_rounded_rect_cmd(target, plus_btn);
                    self.draw_text_cmd(target, plus_text);
                }
                ItemControlLayout::Dropdown {
                    button,
                    button_text,
                    arrow_text,
                    ..
                } => {
                    self.draw_rounded_rect_cmd(target, button);
                    self.draw_text_cmd(target, button_text);
                    self.draw_text_cmd(target, arrow_text);
                }
            }
        }

        // Dropdown options (drawn last, on top of everything).
        for item in &layout.items {
            if let ItemControlLayout::Dropdown { options, .. } = &item.controls {
                for opt in options {
                    self.draw_flat_rect_cmd(target, &opt.bg);
                    self.draw_text_cmd(target, &opt.text);
                }
            }
        }

        // Close hint.
        self.draw_text_cmd(target, &layout.close_hint);

        // Close button (X).
        self.draw_rounded_rect_cmd(target, &layout.close_button);
        let (ax0, ay0, ax1, ay1) = layout.close_icon_line_a;
        let (bx0, by0, bx1, by1) = layout.close_icon_line_b;
        Self::draw_stroked_line(target, (ax0, ay0), (ax1, ay1), 1.5, layout.close_icon_color);
        Self::draw_stroked_line(target, (bx0, by0), (bx1, by1), 1.5, layout.close_icon_color);
    }

    /// Draws a text command (helper to avoid code duplication).
    fn draw_text_cmd(&mut self, target: &mut RenderTarget<'_>, cmd: &super::types::TextCmd) {
        let fg = Color::from_pixel(cmd.color);
        let tx = cmd.x as u32;
        let ty = cmd.y as u32;
        for (i, ch) in cmd.text.chars().enumerate() {
            let x = tx + i as u32 * self.metrics.cell_width;
            self.draw_char(target, x, ty, ch, fg);
        }
    }
}
