//! Packs terminal grid cells into GPU buffer format.

use crate::core::{Color, Grid, Selection, UnderlineStyle};
use crate::gui::pane::PaneRect;

use super::GridBatch;
use super::buffers::{GridUniforms, PackedCell};

impl super::GpuRenderer {
    pub(super) fn terminal_texture_extent(&self) -> (u32, u32) {
        let padding = self.metrics.window_padding_px();
        let tab_bar_height = self.metrics.tab_bar_height_px();
        let terminal_width = self.width.saturating_sub(padding.saturating_mul(2));
        let terminal_height = self
            .height
            .saturating_sub(tab_bar_height + padding.saturating_mul(2));
        (terminal_width, terminal_height)
    }

    fn ensure_grid_frame_started(&mut self) {
        if self.grid_dirty {
            return;
        }
        self.grid_dirty = true;
        self.grid_batches.clear();

        // First pass: clear the full terminal texture area to default background.
        let (terminal_width, terminal_height) = self.terminal_texture_extent();
        if terminal_width == 0 || terminal_height == 0 {
            return;
        }

        let bg = self.palette.default_bg.to_pixel();
        self.grid_batches.push(GridBatch {
            cells: vec![PackedCell {
                codepoint: 0,
                fg: bg,
                bg,
                attrs: 0,
            }],
            uniforms: GridUniforms {
                cols: 1,
                rows: 1,
                cell_width: terminal_width,
                cell_height: terminal_height,
                origin_x: 0,
                origin_y: 0,
                bg_color: bg,
                _pad0: 0,
                tex_width: self.width,
                tex_height: self.height,
                _pad1: 0,
                _pad2: 0,
            },
            dispatch_width: terminal_width,
            dispatch_height: terminal_height,
        });
    }

    fn pack_grid_cells(
        &mut self,
        grid: &Grid,
        selection: Option<&Selection>,
        viewport_start: usize,
        fg_dim: f32,
    ) -> Vec<PackedCell> {
        let rows = grid.rows;
        let cols = grid.cols;
        let mut cells = Vec::with_capacity(rows * cols);
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
                if cell.underline_style != UnderlineStyle::None {
                    attrs |= 4;
                }
                if cell.reverse {
                    attrs |= 8;
                }

                let mut fg = if cell.fg == Color::SENTINEL_FG { self.palette.default_fg } else { cell.fg };
                let bg_color = if cell.bg == Color::SENTINEL_BG { self.palette.default_bg } else { cell.bg };
                if cell.bold {
                    fg = fg.bold_bright_with_palette(&self.palette.ansi);
                }
                if fg_dim > 0.0 {
                    fg = fg.dimmed(fg_dim);
                }
                let mut bg = bg_color.to_pixel();
                if selected {
                    bg = super::super::blend_rgb(
                        bg,
                        self.palette.selection_overlay_color.to_pixel(),
                        self.palette.selection_overlay_alpha,
                    );
                }

                cells.push(PackedCell {
                    codepoint,
                    fg: fg.to_pixel(),
                    bg,
                    attrs,
                });
            }
        }
        cells
    }

    /// Queues one grid compute batch for this frame.
    pub(super) fn queue_grid_batch(
        &mut self,
        grid: &Grid,
        selection: Option<&Selection>,
        viewport_start: usize,
        region: PaneRect,
        fg_dim: f32,
    ) {
        self.ensure_grid_frame_started();

        if region.width == 0 || region.height == 0 {
            return;
        }

        let dispatch_width = (grid.cols as u32)
            .saturating_mul(self.metrics.cell_width)
            .min(region.width);
        let dispatch_height = (grid.rows as u32)
            .saturating_mul(self.metrics.cell_height)
            .min(region.height);
        if dispatch_width == 0 || dispatch_height == 0 {
            return;
        }

        let cells = self.pack_grid_cells(grid, selection, viewport_start, fg_dim);
        self.grid_batches.push(GridBatch {
            cells,
            uniforms: GridUniforms {
                cols: grid.cols as u32,
                rows: grid.rows as u32,
                cell_width: self.metrics.cell_width,
                cell_height: self.metrics.cell_height,
                origin_x: region.x,
                origin_y: region.y,
                bg_color: self.palette.default_bg.to_pixel(),
                _pad0: 0,
                tex_width: self.width,
                tex_height: self.height,
                _pad1: 0,
                _pad2: 0,
            },
            dispatch_width,
            dispatch_height,
        });
    }
}
