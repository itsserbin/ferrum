use crate::gui::*;

impl FerrumWindow {
    /// Resolves resize direction from pointer position near window borders.
    pub(in crate::gui) fn edge_direction(&self, x: f64, y: f64) -> Option<ResizeDirection> {
        let size = self.window.inner_size();
        let w = size.width as f64;
        let h = size.height as f64;
        let resize_border = self.renderer.resize_border_px();

        let left = x < resize_border;
        let right = x > w - resize_border;
        let top = y < resize_border;
        let bottom = y > h - resize_border;

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
