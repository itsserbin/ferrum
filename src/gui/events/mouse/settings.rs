use crate::config::{FontFamily, ThemeChoice};
use crate::gui::settings::layout::{compute_settings_layout, ItemControlLayout};
use crate::gui::settings::{SettingItem, SettingsCategory};
use crate::gui::*;

impl FerrumWindow {
    /// Handles left-click on the settings overlay.
    /// Returns `true` if the click was consumed (overlay is open).
    pub(super) fn handle_settings_left_click(
        &mut self,
        state: ElementState,
        mx: f64,
        my: f64,
    ) -> bool {
        if state != ElementState::Pressed {
            return self.settings_overlay.is_some();
        }
        let Some(overlay) = &self.settings_overlay else {
            return false;
        };

        // Compute layout to determine what was clicked.
        let size = self.window.inner_size();
        let layout = compute_settings_layout(
            size.width,
            size.height,
            self.backend.cell_width(),
            self.backend.cell_height(),
            self.backend.ui_scale(),
            overlay,
            self.backend.palette_menu_bg(),
            self.backend.palette_active_accent(),
            self.backend.palette_text_active(),
            self.backend.palette_text_inactive(),
            self.backend.palette_bar_bg(),
        );

        // Check if click is outside the panel -> close overlay.
        let panel = &layout.panel_bg;
        if mx < panel.x as f64
            || mx > (panel.x + panel.w) as f64
            || my < panel.y as f64
            || my > (panel.y + panel.h) as f64
        {
            self.close_settings_overlay();
            return true;
        }

        // Check close button click.
        let cb = &layout.close_button;
        if mx >= cb.x as f64
            && mx < (cb.x + cb.w) as f64
            && my >= cb.y as f64
            && my < (cb.y + cb.h) as f64
        {
            self.close_settings_overlay();
            return true;
        }

        // Check category clicks.
        for (i, cat_layout) in layout.categories.iter().enumerate() {
            let bg = &cat_layout.bg;
            if mx >= bg.x as f64
                && mx < (bg.x + bg.w) as f64
                && my >= bg.y as f64
                && my < (bg.y + bg.h) as f64
            {
                if let Some(overlay) = self.settings_overlay.as_mut() {
                    overlay.active_category = SettingsCategory::CATEGORIES[i];
                    overlay.hovered_item = None;
                    overlay.scroll_offset = 0;
                    overlay.open_dropdown = None;
                    overlay.hovered_dropdown_option = None;
                }
                self.window.request_redraw();
                return true;
            }
        }

        // Check open dropdown option clicks first (they render on top).
        let items = overlay.items();
        for (i, item_layout) in layout.items.iter().enumerate() {
            if let ItemControlLayout::Dropdown { options, .. } = &item_layout.controls {
                for (j, opt) in options.iter().enumerate() {
                    if mx >= opt.bg.x as f64
                        && mx < (opt.bg.x + opt.bg.w) as f64
                        && my >= opt.bg.y as f64
                        && my < (opt.bg.y + opt.bg.h) as f64
                    {
                        self.apply_enum_selection(i, j, &items);
                        if let Some(ref mut ov) = self.settings_overlay {
                            ov.open_dropdown = None;
                            ov.hovered_dropdown_option = None;
                            let config = ov.editing_config.clone();
                            self.apply_config_change(&config);
                        }
                        self.window.request_redraw();
                        return true;
                    }
                }
            }
        }

        // Check item control clicks (stepper buttons and dropdown buttons).
        for (i, item_layout) in layout.items.iter().enumerate() {
            match &item_layout.controls {
                ItemControlLayout::Stepper {
                    minus_btn,
                    plus_btn,
                    ..
                } => {
                    if hit_test_rounded_rect(minus_btn, mx, my) {
                        self.apply_stepper_change(i, -1, &items);
                        if let Some(ref overlay) = self.settings_overlay {
                            let config = overlay.editing_config.clone();
                            self.apply_config_change(&config);
                        }
                        self.window.request_redraw();
                        return true;
                    }
                    if hit_test_rounded_rect(plus_btn, mx, my) {
                        self.apply_stepper_change(i, 1, &items);
                        if let Some(ref overlay) = self.settings_overlay {
                            let config = overlay.editing_config.clone();
                            self.apply_config_change(&config);
                        }
                        self.window.request_redraw();
                        return true;
                    }
                }
                ItemControlLayout::Dropdown { button, .. } => {
                    if hit_test_rounded_rect(button, mx, my) {
                        if let Some(ref mut ov) = self.settings_overlay {
                            if ov.open_dropdown == Some(i) {
                                ov.open_dropdown = None;
                                ov.hovered_dropdown_option = None;
                            } else {
                                ov.open_dropdown = Some(i);
                                ov.hovered_dropdown_option = None;
                            }
                        }
                        self.window.request_redraw();
                        return true;
                    }
                }
            }
        }

        // Click inside panel but not on any control -> close any open dropdown.
        if let Some(ref mut ov) = self.settings_overlay {
            if ov.open_dropdown.is_some() {
                ov.open_dropdown = None;
                ov.hovered_dropdown_option = None;
                self.window.request_redraw();
            }
        }

        true // Consume click inside panel even if nothing specific was hit.
    }

    /// Handles mouse movement over the settings overlay.
    /// Returns `true` if the overlay is open (consume cursor update).
    pub(super) fn handle_settings_mouse_move(&mut self, mx: f64, my: f64) -> bool {
        let Some(overlay) = &self.settings_overlay else {
            return false;
        };

        let size = self.window.inner_size();
        let layout = compute_settings_layout(
            size.width,
            size.height,
            self.backend.cell_width(),
            self.backend.cell_height(),
            self.backend.ui_scale(),
            overlay,
            self.backend.palette_menu_bg(),
            self.backend.palette_active_accent(),
            self.backend.palette_text_active(),
            self.backend.palette_text_inactive(),
            self.backend.palette_bar_bg(),
        );

        let mut new_hovered_cat: Option<usize> = None;
        let mut new_hovered_item: Option<usize> = None;
        let mut new_hovered_dropdown_opt: Option<usize> = None;

        // Check category hover.
        for (i, cat_layout) in layout.categories.iter().enumerate() {
            let bg = &cat_layout.bg;
            if mx >= bg.x as f64
                && mx < (bg.x + bg.w) as f64
                && my >= bg.y as f64
                && my < (bg.y + bg.h) as f64
            {
                new_hovered_cat = Some(i);
                break;
            }
        }

        // Check item hover.
        for (i, item_layout) in layout.items.iter().enumerate() {
            let label_y = item_layout.label.y as f64;
            let row_bottom = label_y + self.backend.cell_height() as f64 * 2.5;
            if my >= label_y && my < row_bottom {
                new_hovered_item = Some(i);
                break;
            }
        }

        // Check dropdown option hover (when a dropdown is open).
        for item_layout in &layout.items {
            if let ItemControlLayout::Dropdown { options, .. } = &item_layout.controls {
                for (j, opt) in options.iter().enumerate() {
                    if mx >= opt.bg.x as f64
                        && mx < (opt.bg.x + opt.bg.w) as f64
                        && my >= opt.bg.y as f64
                        && my < (opt.bg.y + opt.bg.h) as f64
                    {
                        new_hovered_dropdown_opt = Some(j);
                        break;
                    }
                }
            }
        }

        let overlay = self.settings_overlay.as_mut().unwrap();
        let changed = overlay.hovered_category != new_hovered_cat
            || overlay.hovered_item != new_hovered_item
            || overlay.hovered_dropdown_option != new_hovered_dropdown_opt;
        overlay.hovered_category = new_hovered_cat;
        overlay.hovered_item = new_hovered_item;
        overlay.hovered_dropdown_option = new_hovered_dropdown_opt;

        if changed {
            self.window.request_redraw();
        }
        true
    }

    /// Applies an enum button selection to the editing config.
    fn apply_enum_selection(
        &mut self,
        item_index: usize,
        option_index: usize,
        items: &[SettingItem],
    ) {
        let Some(overlay) = self.settings_overlay.as_mut() else {
            return;
        };
        let Some(item) = items.get(item_index) else {
            return;
        };
        if let SettingItem::EnumChoice { label, .. } = item {
            match *label {
                "Font Family" => {
                    overlay.editing_config.font.family = match option_index {
                        0 => FontFamily::JetBrainsMono,
                        _ => FontFamily::FiraCode,
                    };
                }
                "Theme" => {
                    overlay.editing_config.theme = match option_index {
                        0 => ThemeChoice::FerrumDark,
                        _ => ThemeChoice::CatppuccinLatte,
                    };
                }
                _ => {}
            }
        }
    }

    /// Applies a stepper increment/decrement to the editing config.
    ///
    /// `direction` is +1 for increment, -1 for decrement.
    fn apply_stepper_change(
        &mut self,
        item_index: usize,
        direction: i32,
        items: &[SettingItem],
    ) {
        let Some(overlay) = self.settings_overlay.as_mut() else {
            return;
        };
        let Some(item) = items.get(item_index) else {
            return;
        };
        match item {
            SettingItem::FloatSlider {
                label,
                value,
                min,
                max,
                step,
            } => {
                let new_val = (*value + *step * direction as f32).clamp(*min, *max);
                if *label == "Font Size" {
                    overlay.editing_config.font.size = new_val;
                }
            }
            SettingItem::IntSlider {
                label, value, min, max, ..
            } => {
                let new_val = (*value as i64 + direction as i64).clamp(*min as i64, *max as i64) as u32;
                match *label {
                    "Line Padding" => overlay.editing_config.font.line_padding = new_val,
                    "Cursor Blink (ms)" => {
                        overlay.editing_config.terminal.cursor_blink_interval_ms = new_val as u64;
                    }
                    "Window Padding" => overlay.editing_config.layout.window_padding = new_val,
                    "Tab Bar Height" => overlay.editing_config.layout.tab_bar_height = new_val,
                    "Pane Padding" => overlay.editing_config.layout.pane_inner_padding = new_val,
                    "Scrollbar Width" => overlay.editing_config.layout.scrollbar_width = new_val,
                    _ => {}
                }
            }
            SettingItem::LargeIntSlider {
                label,
                value,
                min,
                max,
                step,
            } => {
                let delta = *step as i64 * direction as i64;
                let new_val = (*value as i64 + delta).clamp(*min as i64, *max as i64) as usize;
                if *label == "Max Scrollback" {
                    overlay.editing_config.terminal.max_scrollback = new_val;
                }
            }
            _ => {}
        }
    }
}

/// Hit tests a rounded rect command against mouse coordinates.
fn hit_test_rounded_rect(
    rect: &crate::gui::renderer::types::RoundedRectCmd,
    mx: f64,
    my: f64,
) -> bool {
    mx >= rect.x as f64
        && mx < (rect.x + rect.w) as f64
        && my >= rect.y as f64
        && my < (rect.y + rect.h) as f64
}
