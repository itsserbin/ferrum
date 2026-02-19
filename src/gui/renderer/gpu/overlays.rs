#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::shared::overlay_layout;
use super::super::{ACTIVE_TAB_BG, INSERTION_COLOR, TAB_BORDER, TAB_TEXT_ACTIVE, TabInfo};

impl super::GpuRenderer {
    pub(super) fn draw_tab_drag_overlay_impl(
        &mut self,
        buf_width: usize,
        tabs: &[TabInfo],
        source_index: usize,
        current_x: f64,
        indicator_x: f32,
    ) {
        let m = self.tab_layout_metrics();
        let layout = match overlay_layout::compute_drag_overlay_layout(
            &m,
            tabs.len(),
            source_index,
            tabs[source_index].title,
            current_x,
            indicator_x,
            buf_width as u32,
        ) {
            Some(l) => l,
            None => return,
        };

        // Shadow.
        self.push_rounded_rect(
            layout.shadow_x as f32,
            layout.shadow_y as f32,
            layout.rect_w as f32,
            layout.rect_h as f32,
            layout.radius as f32,
            0x000000,
            0.24,
        );
        // Body.
        self.push_rounded_rect(
            layout.body_x as f32,
            layout.body_y as f32,
            layout.rect_w as f32,
            layout.rect_h as f32,
            layout.radius as f32,
            ACTIVE_TAB_BG,
            0.86,
        );
        // Border.
        self.push_rounded_rect(
            layout.body_x as f32,
            layout.body_y as f32,
            layout.rect_w as f32,
            layout.rect_h as f32,
            layout.radius as f32,
            TAB_BORDER,
            0.39,
        );

        // Ghost title.
        self.push_text(
            layout.title_x as f32,
            layout.title_y as f32,
            &layout.title_text,
            TAB_TEXT_ACTIVE,
            1.0,
        );

        // Smooth insertion indicator at lerped position.
        self.push_rect(
            layout.indicator_x as f32,
            layout.indicator_y as f32,
            layout.indicator_w as f32,
            layout.indicator_h as f32,
            INSERTION_COLOR,
            1.0,
        );
    }

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
        self.push_rounded_rect(x, y, w, h, r, ACTIVE_TAB_BG, 0.96);
        self.push_rounded_rect(x, y, w, h, r, TAB_BORDER, 0.31);

        self.push_text(
            layout.text_x as f32,
            layout.text_y as f32,
            &layout.display_text,
            TAB_TEXT_ACTIVE,
            1.0,
        );
    }

}
