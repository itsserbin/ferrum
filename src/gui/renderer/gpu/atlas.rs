//! Glyph atlas — rasterizes glyphs on CPU and packs them into a GPU texture.
//!
//! Pre-populates ASCII 32..127 on creation; lazily adds Unicode glyphs on demand.

use std::collections::HashMap;
use wgpu;

/// Per-glyph metadata stored in a GPU storage buffer. Must match the WGSL `GlyphInfo` layout.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlyphInfo {
    /// Atlas pixel X origin.
    pub x: f32,
    /// Atlas pixel Y origin.
    pub y: f32,
    /// Glyph width in atlas pixels.
    pub w: f32,
    /// Glyph height in atlas pixels.
    pub h: f32,
    /// X offset from cell origin when rendering.
    pub offset_x: f32,
    /// Y offset from cell top when rendering.
    pub offset_y: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

/// Atlas texture with row-packing of rasterized glyphs.
pub struct GlyphAtlas {
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    pub atlas_width: u32,
    pub atlas_height: u32,
    glyphs: HashMap<u32, GlyphInfo>,
    next_x: u32,
    next_y: u32,
    row_height: u32,
    ascent: i32,
}

impl GlyphAtlas {
    /// Creates a new atlas and pre-populates printable ASCII (32..127).
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        font: &fontdue::Font,
        font_size: f32,
        ascent: i32,
    ) -> Self {
        let atlas_width = 1024u32;
        let atlas_height = 1024u32;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph_atlas"),
            size: wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut atlas = GlyphAtlas {
            texture,
            texture_view,
            atlas_width,
            atlas_height,
            glyphs: HashMap::new(),
            next_x: 0,
            next_y: 0,
            row_height: 0,
            ascent,
        };

        // Pre-populate ASCII 32..127.
        for cp in 32u32..127 {
            if let Some(ch) = char::from_u32(cp) {
                atlas.insert_glyph(queue, font, font_size, cp, ch);
            }
        }

        atlas
    }

    /// Returns glyph info for the given codepoint, inserting it into the atlas if missing.
    pub fn get_or_insert(
        &mut self,
        codepoint: u32,
        font: &fontdue::Font,
        font_size: f32,
        queue: &wgpu::Queue,
    ) -> GlyphInfo {
        if let Some(info) = self.glyphs.get(&codepoint) {
            return *info;
        }
        if let Some(ch) = char::from_u32(codepoint) {
            self.insert_glyph(queue, font, font_size, codepoint, ch);
        }
        self.glyphs.get(&codepoint).copied().unwrap_or(GlyphInfo {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        })
    }

    /// Rasterizes one glyph and uploads it to the atlas texture.
    fn insert_glyph(
        &mut self,
        queue: &wgpu::Queue,
        font: &fontdue::Font,
        font_size: f32,
        codepoint: u32,
        ch: char,
    ) {
        let (metrics, bitmap) = font.rasterize(ch, font_size);
        let gw = metrics.width as u32;
        let gh = metrics.height as u32;

        if gw == 0 || gh == 0 {
            // Space or zero-width — store an empty entry.
            self.glyphs.insert(codepoint, GlyphInfo {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                offset_x: 0.0,
                offset_y: 0.0,
                _pad1: 0.0,
                _pad2: 0.0,
            });
            return;
        }

        // Row packing: advance to next row if this glyph doesn't fit.
        if self.next_x + gw > self.atlas_width {
            self.next_y += self.row_height;
            self.next_x = 0;
            self.row_height = 0;
        }

        // Guard against atlas overflow (silently skip).
        if self.next_y + gh > self.atlas_height {
            self.glyphs.insert(codepoint, GlyphInfo {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                offset_x: 0.0,
                offset_y: 0.0,
                _pad1: 0.0,
                _pad2: 0.0,
            });
            return;
        }

        // Upload bitmap data to the atlas texture.
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: self.next_x,
                    y: self.next_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &bitmap,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(gw),
                rows_per_image: Some(gh),
            },
            wgpu::Extent3d {
                width: gw,
                height: gh,
                depth_or_array_layers: 1,
            },
        );

        // Compute offsets for positioning inside a cell.
        let offset_x = metrics.xmin as f32;
        let top = metrics.height as i32 + metrics.ymin;
        let offset_y = (self.ascent - top) as f32;

        let info = GlyphInfo {
            x: self.next_x as f32,
            y: self.next_y as f32,
            w: gw as f32,
            h: gh as f32,
            offset_x,
            offset_y,
            _pad1: 0.0,
            _pad2: 0.0,
        };

        self.glyphs.insert(codepoint, info);
        self.next_x += gw + 1; // 1px padding between glyphs.
        self.row_height = self.row_height.max(gh + 1);
    }

    /// Builds a flat array of GlyphInfo entries indexed by codepoint for the GPU storage buffer.
    /// The array size is `max_codepoint + 1`, zero-filled for missing entries.
    pub fn glyph_info_buffer_data(&self) -> Vec<GlyphInfo> {
        let max_cp = self.glyphs.keys().copied().max().unwrap_or(0) as usize;
        let len = (max_cp + 1).max(128); // At least 128 entries.
        let mut data = vec![
            GlyphInfo {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                offset_x: 0.0,
                offset_y: 0.0,
                _pad1: 0.0,
                _pad2: 0.0,
            };
            len
        ];
        for (&cp, &info) in &self.glyphs {
            if (cp as usize) < len {
                data[cp as usize] = info;
            }
        }
        data
    }

    /// Creates a sampler suitable for the atlas texture.
    pub fn create_sampler(device: &wgpu::Device) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        })
    }
}
