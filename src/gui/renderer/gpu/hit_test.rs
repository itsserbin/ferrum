#![cfg_attr(target_os = "macos", allow(dead_code))]

//! Context menu and security popup drawing for the GPU renderer.

use super::super::SecurityPopup;

impl super::GpuRenderer {
    // ── Security ──────────────────────────────────────────────────────

    /// Draws security popup overlay using a shared layout.
    pub(super) fn draw_security_popup_impl(
        &mut self,
        buf_width: usize,
        buf_height: usize,
        popup: &SecurityPopup,
    ) {
        let layout = popup.layout(
            self.metrics.cell_width,
            self.metrics.cell_height,
            self.metrics.ui_scale,
            buf_width as u32,
            buf_height as u32,
        );

        self.push_rounded_rect_cmd(&layout.bg);
        self.push_rounded_rect_cmd(&layout.border);

        // Title.
        self.push_text(
            layout.title.x,
            layout.title.y,
            &layout.title.text,
            layout.title.color,
            layout.title.opacity,
        );

        // Separator line.
        self.push_rect(
            layout.separator.x,
            layout.separator.y,
            layout.separator.w,
            layout.separator.h,
            layout.separator.color,
            layout.separator.opacity,
        );

        // Content lines.
        for text_cmd in &layout.lines {
            self.push_text(
                text_cmd.x,
                text_cmd.y,
                &text_cmd.text,
                text_cmd.color,
                text_cmd.opacity,
            );
        }
    }
}
