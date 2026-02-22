
//! Tab bar layout math and drawing helpers for the GPU renderer.

use super::super::shared::tab_math;
use super::super::traits::Renderer;
use super::super::types::{TabBarDrawParams, TabSlot};

impl super::GpuRenderer {
    // ── Tab bar rendering: orchestrator ─────────────────────────────────

    pub(super) fn draw_tab_bar_impl(
        &mut self,
        buf_width: usize,
        params: &TabBarDrawParams<'_>,
    ) {
        let tabs = params.tabs;
        let bw = buf_width as u32;
        let tw = self.tab_width(tabs.len(), bw);
        let m = self.tab_layout_metrics();
        let text_y = tab_math::tab_text_y(&m);
        let use_numbers = self.should_show_number(tw);

        self.tab_bar_background_commands(bw);

        for (i, tab) in tabs.iter().enumerate() {
            let anim_offset = params.tab_offsets.and_then(|o| o.get(i)).copied().unwrap_or(0.0);
            let tab_x = self.tab_origin_x(i, tw) as f32 + anim_offset;
            let is_hovered = params.hovered_tab == Some(i);

            let slot = TabSlot {
                index: i,
                tab,
                #[cfg(not(target_os = "macos"))]
                x: tab_x.round() as u32,
                #[cfg(not(target_os = "macos"))]
                width: tw,
                is_hovered,
            };

            self.tab_background_commands(tab, tab_x, tw);

            if tab.is_renaming {
                self.tab_rename_commands(tab, tab_x, tw, text_y);
            } else if use_numbers {
                self.tab_number_commands(&slot, tab_x, tw, text_y);
            } else {
                self.tab_content_commands(&slot, tab_x, tw, text_y);
            }
        }

        self.plus_button_commands(tabs.len(), tw, params.mouse_pos);

        #[cfg(not(target_os = "macos"))]
        self.draw_pin_button_commands(params.mouse_pos, params.pinned);

        #[cfg(not(target_os = "macos"))]
        self.draw_gear_button_commands(params.mouse_pos, params.settings_open);

        #[cfg(target_os = "macos")]
        let _ = params.pinned;

        #[cfg(target_os = "macos")]
        let _ = params.settings_open;

        #[cfg(not(target_os = "macos"))]
        self.draw_window_buttons_commands(bw, params.mouse_pos);

        // Bottom separator line.
        let tab_bar_h = self.metrics.tab_bar_height_px() as f32;
        let sep_y = tab_bar_h - 1.0;
        self.push_rect(0.0, sep_y, bw as f32, 1.0, self.palette.tab_border.to_pixel(), 0.7);
    }
}
