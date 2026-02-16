#![cfg_attr(target_os = "macos", allow(dead_code))]

//! Frame rendering (GPU passes) and window resize logic.

use wgpu;

use crate::core::Color;

use super::MAX_UI_COMMANDS;
use super::buffers::*;

#[cfg(not(target_os = "macos"))]
use super::super::WindowButton;
#[cfg(not(target_os = "macos"))]
use super::WIN_BTN_WIDTH;
use super::{CLOSE_HOVER_BG_COLOR, TAB_TEXT_INACTIVE};

impl super::GpuRenderer {
    pub(super) fn draw_close_button_commands(
        &mut self,
        tab_index: usize,
        tw: u32,
        mouse_pos: (f64, f64),
    ) {
        let (cx, cy, cw, ch) = self.close_button_rect(tab_index, tw);
        let is_close_hovered = mouse_pos.0 >= cx as f64
            && mouse_pos.0 < (cx + cw) as f64
            && mouse_pos.1 >= cy as f64
            && mouse_pos.1 < (cy + ch) as f64
            && mouse_pos.1 < self.metrics.tab_bar_height_px() as f64;

        if is_close_hovered {
            let circle_r = cw.min(ch) as f32 / 2.0;
            let circle_cx = cx as f32 + cw as f32 / 2.0;
            let circle_cy = cy as f32 + ch as f32 / 2.0;
            self.push_circle(circle_cx, circle_cy, circle_r, CLOSE_HOVER_BG_COLOR, 1.0);
        }

        // X icon.
        let center_x = cx as f32 + cw as f32 * 0.5;
        let center_y = cy as f32 + ch as f32 * 0.5;
        let half = (cw.min(ch) as f32 * 0.22).clamp(2.5, 4.5);
        let thickness = (1.25 * self.metrics.ui_scale as f32).clamp(1.15, 2.2);
        self.push_line(
            center_x - half,
            center_y - half,
            center_x + half,
            center_y + half,
            thickness,
            TAB_TEXT_INACTIVE,
            1.0,
        );
        self.push_line(
            center_x + half,
            center_y - half,
            center_x - half,
            center_y + half,
            thickness,
            TAB_TEXT_INACTIVE,
            1.0,
        );
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_window_buttons_commands(&mut self, buf_width: u32, mouse_pos: (f64, f64)) {
        let bar_h = self.metrics.tab_bar_height_px() as f32;
        let btn_w = self.metrics.scaled_px(WIN_BTN_WIDTH);
        let bw = buf_width;

        let buttons: [(u32, WindowButton); 3] = [
            (bw.saturating_sub(btn_w * 3), WindowButton::Minimize),
            (bw.saturating_sub(btn_w * 2), WindowButton::Maximize),
            (bw.saturating_sub(btn_w), WindowButton::Close),
        ];

        for &(btn_x, ref btn_type) in &buttons {
            let is_hovered = mouse_pos.0 >= btn_x as f64
                && mouse_pos.0 < (btn_x + btn_w) as f64
                && mouse_pos.1 >= 0.0
                && mouse_pos.1 < bar_h as f64;

            if is_hovered {
                let hover_bg = if *btn_type == WindowButton::Close {
                    0xF38BA8
                } else {
                    0x313244
                };
                self.push_rect(btn_x as f32, 0.0, btn_w as f32, bar_h, hover_bg, 1.0);
            }

            let icon_color = if is_hovered && *btn_type == WindowButton::Close {
                0xFFFFFF
            } else {
                0x6C7086
            };

            let center_x = btn_x as f32 + btn_w as f32 / 2.0;
            let center_y = bar_h / 2.0;
            let thickness = (1.25 * self.metrics.ui_scale as f32).clamp(1.15, 2.2);

            match btn_type {
                WindowButton::Minimize => {
                    let half_w = self.metrics.scaled_px(5) as f32;
                    self.push_line(
                        center_x - half_w,
                        center_y,
                        center_x + half_w,
                        center_y,
                        thickness,
                        icon_color,
                        1.0,
                    );
                }
                WindowButton::Maximize => {
                    let half = self.metrics.scaled_px(5) as f32;
                    let x0 = center_x - half;
                    let y0 = center_y - half;
                    let x1 = center_x + half;
                    let y1 = center_y + half;
                    self.push_line(x0, y0, x1, y0, thickness, icon_color, 1.0);
                    self.push_line(x0, y1, x1, y1, thickness, icon_color, 1.0);
                    self.push_line(x0, y0, x0, y1, thickness, icon_color, 1.0);
                    self.push_line(x1, y0, x1, y1, thickness, icon_color, 1.0);
                }
                WindowButton::Close => {
                    let half = self.metrics.scaled_px(5) as f32 * 0.7;
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
            }
        }
    }

    /// Encodes all GPU passes and presents the frame.
    /// Called as the last step in the frame after all draw_* methods.
    pub fn present_frame(&mut self) {
        // Upload grid data if dirty.
        if self.grid_dirty && !self.grid_cells.is_empty() {
            // Ensure cell buffer is large enough.
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

            // Rebuild glyph info buffer in case new glyphs were added.
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

        // Upload UI commands.
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

        // Composite uniforms.
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

        // Get surface texture.
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => {
                // Reconfigure and retry once.
                self.surface.configure(&self.device, &self.surface_config);
                match self.surface.get_current_texture() {
                    Ok(t) => t,
                    Err(_) => {
                        self.commands.clear();
                        self.grid_dirty = false;
                        return;
                    }
                }
            }
        };
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });

        // Pass 1: Grid compute.
        if self.grid_dirty && !self.grid_cells.is_empty() {
            let grid_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("grid_bind_group"),
                layout: &self.grid_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.grid_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.grid_cell_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.glyph_info_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.atlas.texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&self.grid_texture_view),
                    },
                ],
            });

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("grid_compute_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.grid_pipeline);
            compute_pass.set_bind_group(0, &grid_bind_group, &[]);

            // Dispatch enough workgroups to cover the entire texture so the
            // compute shader fills out-of-grid pixels with the background color.
            let wg_x = (self.width + 15) / 16;
            let wg_y = (self.height + 15) / 16;
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // Pass 2: UI render.
        {
            let ui_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ui_bind_group"),
                layout: &self.ui_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.ui_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.ui_command_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.atlas.texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.ui_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            render_pass.set_pipeline(&self.ui_pipeline);
            render_pass.set_bind_group(0, &ui_bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Fullscreen triangle.
        }

        // Pass 3: Composite.
        {
            let composite_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("composite_bind_group"),
                layout: &self.composite_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.grid_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.ui_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: self.composite_uniform_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("composite_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            render_pass.set_pipeline(&self.composite_pipeline);
            render_pass.set_bind_group(0, &composite_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Reset per-frame state.
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
