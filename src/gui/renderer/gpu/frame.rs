#![cfg_attr(target_os = "macos", allow(dead_code))]

//! Frame lifecycle: buffer upload, surface management, and presentation.

use wgpu;

use crate::core::Color;

use super::MAX_UI_COMMANDS;
use super::buffers::*;

use super::super::{CLOSE_HOVER_BG_COLOR, TAB_TEXT_ACTIVE, TAB_TEXT_INACTIVE};

impl super::GpuRenderer {
    pub(super) fn mix_rgb(c0: u32, c1: u32, t: f32) -> u32 {
        let t = t.clamp(0.0, 1.0);
        let r0 = ((c0 >> 16) & 0xFF) as f32;
        let g0 = ((c0 >> 8) & 0xFF) as f32;
        let b0 = (c0 & 0xFF) as f32;
        let r1 = ((c1 >> 16) & 0xFF) as f32;
        let g1 = ((c1 >> 8) & 0xFF) as f32;
        let b1 = (c1 & 0xFF) as f32;
        let r = (r0 + (r1 - r0) * t).round().clamp(0.0, 255.0) as u32;
        let g = (g0 + (g1 - g0) * t).round().clamp(0.0, 255.0) as u32;
        let b = (b0 + (b1 - b0) * t).round().clamp(0.0, 255.0) as u32;
        (r << 16) | (g << 8) | b
    }

    pub(super) fn draw_close_button_commands(
        &mut self,
        tab_index: usize,
        tw: u32,
        hover_progress: f32,
    ) {
        let (cx, cy, cw, ch) = self.close_button_rect(tab_index, tw);
        let hover_t = hover_progress.clamp(0.0, 1.0);
        if hover_t > 0.01 {
            let circle_r = cw.min(ch) as f32 / 2.0;
            let circle_cx = cx as f32 + cw as f32 / 2.0;
            let circle_cy = cy as f32 + ch as f32 / 2.0;
            self.push_circle(
                circle_cx,
                circle_cy,
                circle_r,
                CLOSE_HOVER_BG_COLOR,
                0.34 + hover_t * 0.51,
            );
        }

        let center_x = cx as f32 + cw as f32 * 0.5;
        let center_y = cy as f32 + ch as f32 * 0.5;
        let half = (cw.min(ch) as f32 * 0.22).clamp(2.5, 4.5);
        let thickness = (1.25 * self.metrics.ui_scale as f32).clamp(1.15, 2.2);
        let icon_color = Self::mix_rgb(TAB_TEXT_INACTIVE, TAB_TEXT_ACTIVE, hover_t * 0.75);
        self.push_line(
            center_x - half,
            center_y - half,
            center_x + half,
            center_y + half,
            thickness,
            icon_color,
            1.0,
        );
        self.push_line(
            center_x + half,
            center_y - half,
            center_x - half,
            center_y + half,
            thickness,
            icon_color,
            1.0,
        );
    }

    /// Uploads grid cell buffer and glyph info to the GPU.
    fn upload_grid_data(&mut self) {
        if !self.grid_dirty || self.grid_cells.is_empty() {
            return;
        }
        let needed = self.grid_cells.len() * std::mem::size_of::<PackedCell>();
        if needed as u64 > self.grid_cell_buffer.size() {
            self.grid_cell_buffer =
                Self::create_storage_buffer(&self.device, needed, "grid_cells");
        }
        self.queue.write_buffer(
            &self.grid_cell_buffer,
            0,
            bytemuck::cast_slice(&self.grid_cells),
        );
        self.queue.write_buffer(
            &self.grid_uniform_buffer,
            0,
            bytemuck::bytes_of(&self.grid_uniforms),
        );

        let glyph_data = self.atlas.glyph_info_buffer_data();
        let glyph_bytes = bytemuck::cast_slice(&glyph_data);
        if glyph_bytes.len() as u64 > self.glyph_info_buffer.size() {
            self.glyph_info_buffer =
                Self::create_storage_buffer_init(&self.device, glyph_bytes, "glyph_info");
        } else {
            self.queue
                .write_buffer(&self.glyph_info_buffer, 0, glyph_bytes);
        }
    }

    /// Uploads UI commands and composite uniforms to the GPU.
    fn upload_ui_data(&mut self) {
        let command_count = self.commands.len().min(MAX_UI_COMMANDS);
        let ui_uniforms = UiUniforms {
            width: self.width as f32,
            height: self.height as f32,
            atlas_width: self.atlas.atlas_width as f32,
            atlas_height: self.atlas.atlas_height as f32,
            command_count: command_count as u32,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };
        self.queue
            .write_buffer(&self.ui_uniform_buffer, 0, bytemuck::bytes_of(&ui_uniforms));

        if command_count > 0 {
            let cmd_bytes = bytemuck::cast_slice(&self.commands[..command_count]);
            if cmd_bytes.len() as u64 > self.ui_command_buffer.size() {
                self.ui_command_buffer =
                    Self::create_storage_buffer(&self.device, cmd_bytes.len(), "ui_commands");
            }
            self.queue
                .write_buffer(&self.ui_command_buffer, 0, cmd_bytes);
        }

        let grid_pixel_w = self.grid_uniforms.cols * self.grid_uniforms.cell_width;
        let grid_pixel_h = self.grid_uniforms.rows * self.grid_uniforms.cell_height;
        let composite_uniforms = CompositeUniforms {
            tab_bar_height: self.metrics.tab_bar_height_px() as f32,
            window_height: self.height as f32,
            window_width: self.width as f32,
            window_padding: self.metrics.window_padding_px() as f32,
            grid_pixel_width: grid_pixel_w as f32,
            grid_pixel_height: grid_pixel_h as f32,
            bg_color: Color::DEFAULT_BG.to_pixel(),
            _padding: 0,
        };
        self.queue.write_buffer(
            &self.composite_uniform_buffer,
            0,
            bytemuck::bytes_of(&composite_uniforms),
        );
    }

    /// Acquires the surface texture, reconfiguring once on failure.
    fn acquire_surface(
        &mut self,
    ) -> Option<(wgpu::SurfaceTexture, wgpu::TextureView)> {
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => {
                self.surface.configure(&self.device, &self.surface_config);
                match self.surface.get_current_texture() {
                    Ok(t) => t,
                    Err(_) => {
                        self.commands.clear();
                        self.grid_dirty = false;
                        return None;
                    }
                }
            }
        };
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        Some((output, output_view))
    }

    /// Encodes all GPU passes and presents the frame.
    pub fn present_frame(&mut self) {
        self.upload_grid_data();
        self.upload_ui_data();

        let Some((output, output_view)) = self.acquire_surface() else {
            return;
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });

        self.encode_grid_pass(&mut encoder);
        self.encode_ui_pass(&mut encoder);
        self.encode_composite_pass(&mut encoder, &output_view);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.commands.clear();
        self.grid_dirty = false;
    }

    /// Resizes the surface and internal textures.
    pub fn resize(&mut self, width: u32, height: u32) {
        let w = width.max(1);
        let h = height.max(1);
        if w == self.width && h == self.height {
            return;
        }
        self.width = w;
        self.height = h;
        self.surface_config.width = w;
        self.surface_config.height = h;
        self.surface.configure(&self.device, &self.surface_config);
        self.resize_textures();
    }
}
