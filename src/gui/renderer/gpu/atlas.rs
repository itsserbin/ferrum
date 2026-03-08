//! Glyph atlas — rasterizes glyphs via [`GlyphRasterizer`] and packs them into a GPU texture.
//!
//! Pre-populates ASCII 32..127 on creation; adds other glyphs lazily.
//! Texture format depends on raster mode:
//!   Grayscale   → R8Unorm   (1 byte per texel: alpha coverage)
//!   LcdSubpixel → Rgba8Unorm (3 bytes per texel: R_cov, G_cov, B_cov; A unused)

use std::collections::HashMap;
use wgpu;

use crate::gui::renderer::rasterizer::{GlyphCoverage, GlyphRasterizer, RasterMode};

/// Per-glyph metadata stored in a GPU storage buffer. Must match the WGSL `GlyphInfo` layout.
#[repr(C)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
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
    pub texture:      wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    pub atlas_width:  u32,
    pub atlas_height: u32,
    glyphs:           HashMap<u32, GlyphInfo>,
    next_x:           u32,
    next_y:           u32,
    row_height:       u32,
}

impl GlyphAtlas {
    /// Creates a new atlas and pre-populates printable ASCII (32..127).
    pub fn new(
        device:     &wgpu::Device,
        queue:      &wgpu::Queue,
        rasterizer: &mut GlyphRasterizer,
    ) -> Self {
        let atlas_width  = 1024u32;
        let atlas_height = 1024u32;
        let mode = rasterizer.mode;

        let format = match mode {
            RasterMode::Grayscale   => wgpu::TextureFormat::R8Unorm,
            RasterMode::LcdSubpixel => wgpu::TextureFormat::Rgba8Unorm,
        };

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
            format,
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
        };

        for cp in 32u32..127 {
            if let Some(ch) = char::from_u32(cp) {
                atlas.insert_glyph(queue, rasterizer, cp, ch);
            }
        }

        atlas
    }

    /// Returns glyph info for `codepoint`, inserting it lazily if missing.
    pub fn get_or_insert(
        &mut self,
        codepoint:  u32,
        rasterizer: &mut GlyphRasterizer,
        queue:      &wgpu::Queue,
    ) -> GlyphInfo {
        if let Some(&info) = self.glyphs.get(&codepoint) {
            return info;
        }
        if let Some(ch) = char::from_u32(codepoint) {
            self.insert_glyph(queue, rasterizer, codepoint, ch);
        }
        self.glyphs.get(&codepoint).copied().unwrap_or_default()
    }

    /// Rasterizes one glyph and uploads it to the atlas texture.
    fn insert_glyph(
        &mut self,
        queue:      &wgpu::Queue,
        rasterizer: &mut GlyphRasterizer,
        codepoint:  u32,
        ch:         char,
    ) {
        let Some(glyph) = rasterizer.rasterize(ch) else {
            self.glyphs.insert(codepoint, GlyphInfo::default());
            return;
        };

        let gw = glyph.width;
        let gh = glyph.height;

        // Row packing: advance to next row if this glyph doesn't fit.
        if self.next_x + gw > self.atlas_width {
            self.next_y += self.row_height;
            self.next_x = 0;
            self.row_height = 0;
        }

        // Guard against atlas overflow (silently skip).
        if self.next_y + gh > self.atlas_height {
            self.glyphs.insert(codepoint, GlyphInfo::default());
            return;
        }

        let (bytes_per_row, upload_data): (u32, Vec<u8>) = match &glyph.coverage {
            GlyphCoverage::Grayscale(data) => (gw, data.clone()),
            GlyphCoverage::Lcd(data) => {
                // Pack [R, G, B] into RGBA: A=0 (unused).
                let rgba: Vec<u8> = data.iter()
                    .flat_map(|&[r, g, b]| [r, g, b, 0u8])
                    .collect();
                (gw * 4, rgba)
            }
        };

        // Upload coverage data to the atlas texture.
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture:   &self.texture,
                mip_level: 0,
                origin:    wgpu::Origin3d { x: self.next_x, y: self.next_y, z: 0 },
                aspect:    wgpu::TextureAspect::All,
            },
            &upload_data,
            wgpu::TexelCopyBufferLayout {
                offset:         0,
                bytes_per_row:  Some(bytes_per_row),
                rows_per_image: Some(gh),
            },
            wgpu::Extent3d { width: gw, height: gh, depth_or_array_layers: 1 },
        );

        let metrics = rasterizer.metrics();
        let offset_x = glyph.left as f32;
        let offset_y = (metrics.ascent - glyph.top) as f32;

        self.glyphs.insert(codepoint, GlyphInfo {
            x:        self.next_x as f32,
            y:        self.next_y as f32,
            w:        gw as f32,
            h:        gh as f32,
            offset_x,
            offset_y,
            _pad1: 0.0,
            _pad2: 0.0,
        });

        self.next_x    += gw + 1; // 1px padding between glyphs.
        self.row_height = self.row_height.max(gh + 1);
    }

    /// Builds the flat glyph info array for the GPU storage buffer.
    ///
    /// The array size is `max_codepoint + 1`, zero-filled for missing entries.
    pub fn glyph_info_buffer_data(&self) -> Vec<GlyphInfo> {
        let max_cp = self.glyphs.keys().copied().max().unwrap_or(0) as usize;
        let len = (max_cp + 1).max(128); // At least 128 entries.
        let mut data = vec![GlyphInfo::default(); len];
        for (&cp, &info) in &self.glyphs {
            if (cp as usize) < len {
                data[cp as usize] = info;
            }
        }
        data
    }
}
