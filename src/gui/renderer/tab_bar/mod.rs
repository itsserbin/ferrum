
mod buttons;
mod drag_overlay;
mod layout;
mod primitives;
mod rename_field;
mod tab_content;

use super::shared::tab_math;

use super::*;
use super::RenderTarget;
use super::RoundedShape;
use super::types::{TabBarDrawParams, TabSlot};

// Tab-bar palette constants (BAR_BG, TAB_TEXT_ACTIVE, INSERTION_COLOR, etc.)
// are centralized in the parent `renderer/mod.rs` and imported via `use super::*`.
// WIN_BTN_WIDTH comes from `shared::tab_math`.

impl CpuRenderer {
    /// Draws top tab bar including tabs, controls, and separators.
    pub fn draw_tab_bar(
        &mut self,
        target: &mut RenderTarget<'_>,
        params: &TabBarDrawParams<'_>,
    ) {
        let tabs = params.tabs;
        let tab_bar_height = self.tab_bar_height_px();
        let bar_h = tab_bar_height as usize;
        let buf_width = target.width;
        let tw = self.tab_width(tabs.len(), buf_width as u32);
        let use_numbers = self.should_show_number(tw);

        // Tab bar background.
        {
            let bar_radius = self.scaled_px(10);
            self.draw_top_rounded_rect(
                target,
                &RoundedShape {
                    x: 0,
                    y: 0,
                    w: buf_width as u32,
                    h: tab_bar_height,
                    radius: bar_radius,
                    color: self.palette.bar_bg.to_pixel(),
                    alpha: 255,
                },
            );
        }

        for (i, tab) in tabs.iter().enumerate() {
            let anim_offset = params.tab_offsets.and_then(|o| o.get(i)).copied().unwrap_or(0.0);
            let tab_x = (self.tab_origin_x(i, tw) as f32 + anim_offset).round() as u32;
            let is_hovered = params.hovered_tab == Some(i);

            let slot = TabSlot {
                index: i,
                tab,
                x: tab_x,
                width: tw,
                is_hovered,
            };

            self.draw_tab_background(target, &slot, tab_bar_height);

            if tab.is_renaming {
                self.draw_tab_rename_field(target, &slot);
            } else if use_numbers {
                self.draw_tab_number(target, &slot);
            } else {
                self.draw_tab_content(target, &slot);
            }
        }

        // Plus button.
        {
            let plus_rect = self.plus_button_rect(tabs.len(), tw);
            let plus_hover = tab_math::point_in_rect(params.mouse_pos.0, params.mouse_pos.1, plus_rect);
            if plus_hover {
                let (px, py, pw, ph) = plus_rect;
                self.draw_button_hover_bg(target, px, py, pw, ph);
            }
            let plus_fg = if plus_hover {
                self.palette.tab_text_active
            } else {
                self.palette.tab_text_inactive
            };
            self.draw_tab_plus_icon(target, plus_rect, plus_fg);
        }

        #[cfg(not(target_os = "macos"))]
        self.draw_pin_button(target, params.mouse_pos, params.pinned);

        #[cfg(not(target_os = "macos"))]
        self.draw_gear_button(target, params.mouse_pos, params.settings_open);

        #[cfg(target_os = "macos")]
        let _ = params.pinned;

        #[cfg(target_os = "macos")]
        let _ = params.settings_open;

        #[cfg(not(target_os = "macos"))]
        self.draw_window_buttons(target, params.mouse_pos);

        // Bottom separator.
        if bar_h > 0 {
            let py = bar_h - 1;
            for px in 0..target.width {
                let idx = py * target.width + px;
                if idx < target.buffer.len() {
                    target.buffer[idx] =
                        super::blend_rgb(target.buffer[idx], self.palette.tab_border.to_pixel(), 180);
                }
            }
        }
    }

    /// Draws active/inactive/hover tab background fill.
    fn draw_tab_background(
        &self,
        target: &mut RenderTarget<'_>,
        slot: &TabSlot,
        tab_bar_height: u32,
    ) {
        let hover_t = slot.tab.hover_progress.clamp(0.0, 1.0);

        if slot.tab.is_active {
            // Active tab: flat fill that merges with terminal (fully opaque).
            fill_tab_rect(target, slot.x, slot.width, tab_bar_height, self.palette.active_tab_bg.to_pixel(), 255);
        } else if hover_t > 0.01 {
            // Inactive tab hover: blended fill highlight.
            let alpha = (hover_t * 220.0).round().clamp(0.0, 255.0) as u8;
            fill_tab_rect(target, slot.x, slot.width, tab_bar_height, self.palette.inactive_tab_hover.to_pixel(), alpha);
        }
        // Inactive non-hovered: no background (BAR_BG shows through).
    }
}

/// Fills a tab-width column of the pixel buffer from row 0 up to `height` rows,
/// blending `color` over the existing pixels with `alpha` (255 = opaque).
fn fill_tab_rect(
    target: &mut RenderTarget<'_>,
    tab_x: u32,
    tab_width: u32,
    height: u32,
    color: u32,
    alpha: u8,
) {
    let bar_h = target.height;
    let buf_width = target.width;
    for py in 0..height as usize {
        if py >= bar_h {
            break;
        }
        for dx in 0..tab_width as usize {
            let px = tab_x as usize + dx;
            if px < buf_width {
                let idx = py * buf_width + px;
                if idx < target.buffer.len() {
                    target.buffer[idx] = super::blend_rgb(target.buffer[idx], color, alpha);
                }
            }
        }
    }
}
