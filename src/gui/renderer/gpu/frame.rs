
//! Frame lifecycle: buffer upload, surface management, and presentation.

use wgpu;

use super::MAX_UI_COMMANDS;
use super::buffers::*;

#[cfg(not(target_os = "macos"))]
use super::super::shared::ui_layout;

impl super::GpuRenderer {
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_close_button_commands(
        &mut self,
        tab_index: usize,
        tw: u32,
        hover_progress: f32,
    ) {
        let rect = self.close_button_rect(tab_index, tw);
        let layout = ui_layout::compute_close_button_layout(
            rect,
            hover_progress,
            self.metrics.ui_scale,
            self.palette.close_hover_bg.to_pixel(),
            self.palette.tab_text_inactive.to_pixel(),
            self.palette.tab_text_active.to_pixel(),
        );

        if layout.show_hover_circle {
            self.push_circle(
                layout.circle_cx,
                layout.circle_cy,
                layout.circle_radius,
                layout.circle_bg_color,
                layout.circle_alpha,
            );
        }

        self.push_line(
            (layout.line_a.0, layout.line_a.1),
            (layout.line_a.2, layout.line_a.3),
            layout.icon_thickness,
            layout.icon_color,
            1.0,
        );
        self.push_line(
            (layout.line_b.0, layout.line_b.1),
            (layout.line_b.2, layout.line_b.3),
            layout.icon_thickness,
            layout.icon_color,
            1.0,
        );
    }

    /// Uploads glyph metadata buffer used by the grid and UI passes.
    fn upload_glyph_data(&mut self) {
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

    /// Executes queued grid batches (clear + pane batches) before compositing.
    fn run_grid_batches(&mut self) {
        if !self.grid_dirty || self.grid_batches.is_empty() {
            return;
        }

        for batch in &self.grid_batches {
            if batch.cells.is_empty() || batch.dispatch_width == 0 || batch.dispatch_height == 0 {
                continue;
            }

            let needed = batch.cells.len() * std::mem::size_of::<PackedCell>();
            if needed as u64 > self.grid_cell_buffer.size() {
                self.grid_cell_buffer =
                    Self::create_storage_buffer(&self.device, needed, "grid_cells");
            }
            self.queue.write_buffer(
                &self.grid_cell_buffer,
                0,
                bytemuck::cast_slice(&batch.cells),
            );
            self.queue.write_buffer(
                &self.grid_uniform_buffer,
                0,
                bytemuck::bytes_of(&batch.uniforms),
            );

            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("grid_batch_encoder"),
                });
            self.encode_grid_batch_pass(&mut encoder, batch.dispatch_width, batch.dispatch_height);
            self.queue.submit(std::iter::once(encoder.finish()));
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

        let (grid_pixel_w, grid_pixel_h) = self.terminal_texture_extent();
        let composite_uniforms = CompositeUniforms {
            tab_bar_height: self.metrics.tab_bar_height_px() as f32,
            window_height: self.height as f32,
            window_width: self.width as f32,
            window_padding: self.metrics.window_padding_px() as f32,
            grid_pixel_width: grid_pixel_w as f32,
            grid_pixel_height: grid_pixel_h as f32,
            bg_color: self.palette.default_bg.to_pixel(),
            _padding: 0,
        };
        self.queue.write_buffer(
            &self.composite_uniform_buffer,
            0,
            bytemuck::bytes_of(&composite_uniforms),
        );
    }

    /// Acquires the surface texture, reconfiguring once on failure.
    fn acquire_surface(&mut self) -> Option<(wgpu::SurfaceTexture, wgpu::TextureView)> {
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => {
                self.surface.configure(&self.device, &self.surface_config);
                match self.surface.get_current_texture() {
                    Ok(t) => t,
                    Err(_) => {
                        self.commands.clear();
                        self.grid_dirty = false;
                        self.grid_batches.clear();
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
        self.upload_glyph_data();
        self.run_grid_batches();
        self.upload_ui_data();

        let Some((output, output_view)) = self.acquire_surface() else {
            return;
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });

        self.encode_ui_pass(&mut encoder);
        self.encode_composite_pass(&mut encoder, &output_view);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.commands.clear();
        self.grid_dirty = false;
        self.grid_batches.clear();
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
