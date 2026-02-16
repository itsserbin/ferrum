//! UI draw command helpers for pushing primitives into the GPU command buffer.

use super::buffers::*;
use super::MAX_UI_COMMANDS;

impl super::GpuRenderer {
    pub(super) fn push_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: u32, alpha: f32) {
        if self.commands.len() >= MAX_UI_COMMANDS {
            return;
        }
        self.commands.push(GpuDrawCommand {
            cmd_type: CMD_RECT,
            param1: x,
            param2: y,
            param3: w,
            param4: h,
            param5: 0.0,
            param6: 0.0,
            color,
            alpha,
            _pad: 0.0,
        });
    }

    pub(super) fn push_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        color: u32,
        alpha: f32,
    ) {
        if self.commands.len() >= MAX_UI_COMMANDS {
            return;
        }
        self.commands.push(GpuDrawCommand {
            cmd_type: CMD_ROUNDED_RECT,
            param1: x,
            param2: y,
            param3: w,
            param4: h,
            param5: r,
            param6: 0.0,
            color,
            alpha,
            _pad: 0.0,
        });
    }

    pub(super) fn push_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        width: f32,
        color: u32,
        alpha: f32,
    ) {
        if self.commands.len() >= MAX_UI_COMMANDS {
            return;
        }
        self.commands.push(GpuDrawCommand {
            cmd_type: CMD_LINE,
            param1: x1,
            param2: y1,
            param3: x2,
            param4: y2,
            param5: width,
            param6: 0.0,
            color,
            alpha,
            _pad: 0.0,
        });
    }

    pub(super) fn push_circle(&mut self, cx: f32, cy: f32, r: f32, color: u32, alpha: f32) {
        if self.commands.len() >= MAX_UI_COMMANDS {
            return;
        }
        self.commands.push(GpuDrawCommand {
            cmd_type: CMD_CIRCLE,
            param1: cx,
            param2: cy,
            param3: r,
            param4: 0.0,
            param5: 0.0,
            param6: 0.0,
            color,
            alpha,
            _pad: 0.0,
        });
    }

    pub(super) fn push_glyph(
        &mut self,
        x: f32,
        y: f32,
        atlas_x: f32,
        atlas_y: f32,
        atlas_w: f32,
        atlas_h: f32,
        color: u32,
        alpha: f32,
    ) {
        if self.commands.len() >= MAX_UI_COMMANDS {
            return;
        }
        self.commands.push(GpuDrawCommand {
            cmd_type: CMD_GLYPH,
            param1: x,
            param2: y,
            param3: atlas_x,
            param4: atlas_y,
            param5: atlas_w,
            param6: atlas_h,
            color,
            alpha,
            _pad: 0.0,
        });
    }

    /// Pushes glyph draw commands for a string at the given position.
    pub(super) fn push_text(&mut self, x: f32, y: f32, text: &str, color: u32, alpha: f32) {
        let cw = self.metrics.cell_width as f32;
        for (i, ch) in text.chars().enumerate() {
            let cp = ch as u32;
            let info = self.atlas.get_or_insert(cp, &self.font, self.metrics.font_size, &self.queue);
            if info.w > 0.0 && info.h > 0.0 {
                let gx = x + i as f32 * cw + info.offset_x;
                let gy = y + info.offset_y;
                self.push_glyph(gx, gy, info.x, info.y, info.w, info.h, color, alpha);
            }
        }
    }
}
