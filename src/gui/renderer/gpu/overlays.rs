
#[cfg(not(target_os = "macos"))]
use super::super::shared::overlay_layout;
#[cfg(not(target_os = "macos"))]
use super::super::traits::Renderer;
#[cfg(not(target_os = "macos"))]
use super::super::types::{DragPosition, RoundedRectCmd};
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
