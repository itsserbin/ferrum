#![cfg_attr(target_os = "macos", allow(dead_code))]

//! UI draw command helpers for pushing primitives into the GPU command buffer.

use super::MAX_UI_COMMANDS;
use super::buffers::*;
use super::super::types::RoundedRectCmd;

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

    /// Pushes a rounded rectangle draw command from a layout struct.
    pub(super) fn push_rounded_rect_cmd(&mut self, cmd: &RoundedRectCmd) {
        if self.commands.len() >= MAX_UI_COMMANDS {
            return;
        }
        self.commands.push(GpuDrawCommand {
            cmd_type: CMD_ROUNDED_RECT,
            param1: cmd.x,
            param2: cmd.y,
            param3: cmd.w,
            param4: cmd.h,
            param5: cmd.radius,
            param6: 0.0,
            color: cmd.color,
            alpha: cmd.opacity,
            _pad: 0.0,
        });
    }

    pub(super) fn push_line(
        &mut self,
        p0: (f32, f32),
        p1: (f32, f32),
        width: f32,
        color: u32,
        alpha: f32,
    ) {
        if self.commands.len() >= MAX_UI_COMMANDS {
            return;
        }
        self.commands.push(GpuDrawCommand {
            cmd_type: CMD_LINE,
            param1: p0.0,
            param2: p0.1,
            param3: p1.0,
            param4: p1.1,
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
        atlas: (f32, f32, f32, f32),
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
            param3: atlas.0,
            param4: atlas.1,
            param5: atlas.2,
            param6: atlas.3,
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
            let info =
                self.atlas
                    .get_or_insert(cp, &self.font, &self.fallback_fonts, self.metrics.font_size, &self.queue);
            if info.w > 0.0 && info.h > 0.0 {
                let gx = x + i as f32 * cw + info.offset_x;
                let gy = y + info.offset_y;
                self.push_glyph(gx, gy, (info.x, info.y, info.w, info.h), color, alpha);
            }
        }
    }
}
