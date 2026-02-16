use crate::gui::renderer::RESIZE_BORDER;
use crate::gui::*;

impl FerrumWindow {
    /// Resolves resize direction from pointer position near window borders.
    pub(in crate::gui) fn edge_direction(&self, x: f64, y: f64) -> Option<ResizeDirection> {
        let size = self.window.inner_size();
        let w = size.width as f64;
        let h = size.height as f64;

        let left = x < RESIZE_BORDER;
        let right = x > w - RESIZE_BORDER;
        let top = y < RESIZE_BORDER;
        let bottom = y > h - RESIZE_BORDER;

        match (left, right, top, bottom) {
            (true, _, true, _) => Some(ResizeDirection::NorthWest),
            (_, true, true, _) => Some(ResizeDirection::NorthEast),
            (true, _, _, true) => Some(ResizeDirection::SouthWest),
            (_, true, _, true) => Some(ResizeDirection::SouthEast),
            (true, _, _, _) => Some(ResizeDirection::West),
            (_, true, _, _) => Some(ResizeDirection::East),
            (_, _, true, _) => Some(ResizeDirection::North),
            (_, _, _, true) => Some(ResizeDirection::South),
            _ => None,
        }
    }
}
