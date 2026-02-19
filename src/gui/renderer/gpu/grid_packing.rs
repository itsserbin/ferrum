//! Packs terminal grid cells into GPU buffer format.

use crate::core::{Color, Grid, Selection};

use super::buffers::PackedCell;

impl super::GpuRenderer {
    /// Packs grid cells into the GPU buffer format with cell attributes,
    /// glyph atlas lookup, and color resolution.
    pub(super) fn pack_grid(
        &mut self,
        grid: &Grid,
        selection: Option<&Selection>,
        viewport_start: usize,
    ) {
        let rows = grid.rows;
        let cols = grid.cols;
        self.grid_cells.clear();
        self.grid_cells.reserve(rows * cols);

        for row in 0..rows {
            let abs_row = viewport_start + row;
            for col in 0..cols {
                let cell = grid.get_unchecked(row, col);
                let selected = selection.is_some_and(|s| s.contains(abs_row, col));
                let codepoint = cell.character as u32;

                // Ensure non-ASCII terminal glyphs exist in the atlas.
                if codepoint >= 128 {
                    let _ = self.atlas.get_or_insert(
                        codepoint,
                        &self.font,
                        self.metrics.font_size,
                        &self.queue,
                    );
                }

                let mut attrs = 0u32;
                if cell.bold {
                    attrs |= 1;
                }
                if cell.underline {
                    attrs |= 4;
                }
                if cell.reverse {
                    attrs |= 8;
                }
                if selected {
                    attrs |= 16;
                }

                let mut fg = cell.fg;
                // Bold: bright variant for base ANSI colors.
                if cell.bold {
                    fg = fg.bold_bright();
                }

                self.grid_cells.push(PackedCell {
                    codepoint,
                    fg: fg.to_pixel(),
                    bg: cell.bg.to_pixel(),
                    attrs,
                });
            }
        }

        self.grid_uniforms = super::buffers::GridUniforms {
            cols: cols as u32,
            rows: rows as u32,
            cell_width: self.metrics.cell_width,
            cell_height: self.metrics.cell_height,
            atlas_width: self.atlas.atlas_width,
            atlas_height: self.atlas.atlas_height,
            baseline: self.metrics.ascent as u32,
            bg_color: Color::DEFAULT_BG.to_pixel(),
            tex_width: self.width,
            tex_height: self.height,
            _pad1: 0,
            _pad2: 0,
        };
        self.grid_dirty = true;
    }
}
