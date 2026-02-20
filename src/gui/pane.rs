use std::io::Write;

use crate::core::terminal::Terminal;
use crate::core::{SecurityGuard, Selection};
use crate::gui::state::ScrollbarState;
use crate::pty;

/// Unique identifier for a pane within a tab.
pub(super) type PaneId = u64;

/// Direction in which a pane is split.
#[allow(dead_code)] // Used in later tasks (pane splitting)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SplitDirection {
    /// Left | Right
    Horizontal,
    /// Top / Bottom
    Vertical,
}

/// A rectangle describing a pane's position and size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PaneRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PaneRect {
    #[allow(dead_code)] // Used in later tasks (pane navigation)
    pub fn center_x(&self) -> u32 {
        self.x + self.width / 2
    }
    #[allow(dead_code)] // Used in later tasks (pane navigation)
    pub fn center_y(&self) -> u32 {
        self.y + self.height / 2
    }
}

/// Direction for spatial navigation between panes.
#[allow(dead_code)] // Up/Down used in later tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NavigateDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Result of a divider hit-test, describing which divider was hit.
#[allow(dead_code)] // Fields used in later tasks for resize dragging
pub(super) struct DividerHit {
    pub direction: SplitDirection,
    pub ratio: f32,
    pub position: u32,
    pub available_size: u32,
}

/// Width of the visible divider between panes, in pixels.
pub(super) const DIVIDER_WIDTH: u32 = 1;

/// Width of the hit zone around a divider for mouse interaction, in pixels.
#[allow(dead_code)] // Used in later tasks for resize dragging
pub(super) const DIVIDER_HIT_ZONE: u32 = 6;

/// Splits a rectangle into two sub-rectangles along the given direction.
///
/// The first rectangle gets `ratio` of the available space (total minus divider),
/// and the second rectangle gets the remainder.
pub(super) fn split_rect(
    rect: PaneRect,
    direction: SplitDirection,
    ratio: f32,
    divider_px: u32,
) -> (PaneRect, PaneRect) {
    match direction {
        SplitDirection::Horizontal => {
            let available = rect.width.saturating_sub(divider_px);
            let first_w = (available as f32 * ratio) as u32;
            let second_w = available - first_w;
            (
                PaneRect {
                    x: rect.x,
                    y: rect.y,
                    width: first_w,
                    height: rect.height,
                },
                PaneRect {
                    x: rect.x + first_w + divider_px,
                    y: rect.y,
                    width: second_w,
                    height: rect.height,
                },
            )
        }
        SplitDirection::Vertical => {
            let available = rect.height.saturating_sub(divider_px);
            let first_h = (available as f32 * ratio) as u32;
            let second_h = available - first_h;
            (
                PaneRect {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: first_h,
                },
                PaneRect {
                    x: rect.x,
                    y: rect.y + first_h + divider_px,
                    width: rect.width,
                    height: second_h,
                },
            )
        }
    }
}

/// A leaf node in the pane tree — a single terminal pane.
#[allow(dead_code)] // Fields used in later tasks (Task 4+)
pub(super) struct PaneLeaf {
    pub(super) id: PaneId,
    pub(super) terminal: Terminal,
    pub(super) session: Option<pty::Session>,
    pub(super) pty_writer: Box<dyn Write + Send>,
    pub(super) selection: Option<Selection>,
    pub(super) scroll_offset: usize,
    pub(super) security: SecurityGuard,
    pub(super) scrollbar: ScrollbarState,
}

/// An internal split node holding two children.
pub(super) struct PaneSplit {
    pub(super) direction: SplitDirection,
    pub(super) ratio: f32,
    pub(super) first: Box<PaneNode>,
    pub(super) second: Box<PaneNode>,
}

/// A node in the binary pane tree: either a terminal leaf or a split.
pub(super) enum PaneNode {
    Leaf(PaneLeaf),
    Split(PaneSplit),
}

impl PaneNode {
    /// Create a test-only leaf with stub PTY handles.
    #[cfg(test)]
    fn new_leaf(id: PaneId) -> Self {
        PaneNode::Leaf(PaneLeaf {
            id,
            terminal: Terminal::new(24, 80),
            session: None,
            pty_writer: Box::new(std::io::sink()),
            selection: None,
            scroll_offset: 0,
            security: SecurityGuard::default(),
            scrollbar: ScrollbarState::new(),
        })
    }

    /// Returns `true` if this node is a leaf (not a split).
    pub(super) fn is_leaf(&self) -> bool {
        matches!(self, PaneNode::Leaf(_))
    }

    /// Returns `true` if a leaf with the given id exists in this subtree.
    fn contains_leaf(&self, id: PaneId) -> bool {
        match self {
            PaneNode::Leaf(leaf) => leaf.id == id,
            PaneNode::Split(split) => {
                split.first.contains_leaf(id) || split.second.contains_leaf(id)
            }
        }
    }

    /// Recursively counts the number of leaf nodes in this tree.
    #[allow(dead_code)] // Used in later tasks and tests
    pub(super) fn leaf_count(&self) -> usize {
        match self {
            PaneNode::Leaf(_) => 1,
            PaneNode::Split(split) => split.first.leaf_count() + split.second.leaf_count(),
        }
    }

    /// Recursively collects all leaf IDs in tree order.
    pub(super) fn leaf_ids(&self) -> Vec<PaneId> {
        match self {
            PaneNode::Leaf(leaf) => vec![leaf.id],
            PaneNode::Split(split) => {
                let mut ids = split.first.leaf_ids();
                ids.extend(split.second.leaf_ids());
                ids
            }
        }
    }

    /// Recursively searches for a leaf by ID, returning an immutable reference.
    pub(super) fn find_leaf(&self, id: PaneId) -> Option<&PaneLeaf> {
        match self {
            PaneNode::Leaf(leaf) if leaf.id == id => Some(leaf),
            PaneNode::Leaf(_) => None,
            PaneNode::Split(split) => {
                split.first.find_leaf(id).or_else(|| split.second.find_leaf(id))
            }
        }
    }

    /// Recursively searches for a leaf by ID, returning a mutable reference.
    #[allow(dead_code)] // Used in later tasks (Task 4+)
    pub(super) fn find_leaf_mut(&mut self, id: PaneId) -> Option<&mut PaneLeaf> {
        match self {
            PaneNode::Leaf(leaf) if leaf.id == id => Some(leaf),
            PaneNode::Leaf(_) => None,
            PaneNode::Split(split) => split
                .first
                .find_leaf_mut(id)
                .or_else(|| split.second.find_leaf_mut(id)),
        }
    }

    /// Recursively computes the layout of all leaf panes within the given rectangle.
    ///
    /// Returns a list of `(PaneId, PaneRect)` pairs, one for each leaf, describing
    /// where each pane should be rendered.  `divider_px` is the pixel width of the
    /// divider bar between split panes.
    pub(super) fn layout(&self, rect: PaneRect, divider_px: u32) -> Vec<(PaneId, PaneRect)> {
        match self {
            PaneNode::Leaf(leaf) => vec![(leaf.id, rect)],
            PaneNode::Split(split) => {
                let (first_rect, second_rect) =
                    split_rect(rect, split.direction, split.ratio, divider_px);
                let mut result = split.first.layout(first_rect, divider_px);
                result.extend(split.second.layout(second_rect, divider_px));
                result
            }
        }
    }

    /// Finds the nearest pane in the given direction from `from_id` using spatial
    /// proximity (Manhattan distance between pane centers).
    ///
    /// Only considers panes whose center is strictly in the requested direction
    /// relative to the source pane's center.  Returns `None` if no neighbor
    /// exists in that direction.  When multiple panes are equidistant, returns
    /// the first one encountered in layout order (left-to-right depth-first).
    #[allow(dead_code)] // Used in later tasks (pane navigation shortcuts)
    pub(super) fn navigate_spatial(
        layout: &[(PaneId, PaneRect)],
        from_id: PaneId,
        direction: NavigateDirection,
    ) -> Option<PaneId> {
        let from_rect = layout.iter().find(|(id, _)| *id == from_id)?.1;
        let from_cx = from_rect.center_x() as i64;
        let from_cy = from_rect.center_y() as i64;

        let mut best: Option<(PaneId, i64)> = None;

        for &(id, rect) in layout {
            if id == from_id {
                continue;
            }
            let cx = rect.center_x() as i64;
            let cy = rect.center_y() as i64;

            let in_direction = match direction {
                NavigateDirection::Right => cx > from_cx,
                NavigateDirection::Left => cx < from_cx,
                NavigateDirection::Down => cy > from_cy,
                NavigateDirection::Up => cy < from_cy,
            };

            if !in_direction {
                continue;
            }

            let dist = (cx - from_cx).abs() + (cy - from_cy).abs();
            if best.map_or(true, |(_, d)| dist < d) {
                best = Some((id, dist));
            }
        }

        best.map(|(id, _)| id)
    }

    /// Split the leaf with `target_id`. The new sibling is `new_node`.
    /// When `reverse` is false, the original stays first and `new_node` goes second.
    /// When `reverse` is true, `new_node` goes first and the original goes second
    /// (used for SplitLeft/SplitUp).
    pub(super) fn split_with_node(
        &mut self,
        target_id: PaneId,
        direction: SplitDirection,
        new_node: PaneNode,
        reverse: bool,
    ) -> bool {
        match self {
            PaneNode::Leaf(leaf) if leaf.id == target_id => {
                let original = std::mem::replace(
                    self,
                    PaneNode::Leaf(PaneLeaf {
                        id: u64::MAX,
                        terminal: Terminal::new(1, 1),
                        session: None,
                        pty_writer: Box::new(std::io::sink()),
                        selection: None,
                        scroll_offset: 0,
                        security: SecurityGuard::default(),
                        scrollbar: ScrollbarState::new(),
                    }),
                );
                let (first, second) = if reverse {
                    (new_node, original)
                } else {
                    (original, new_node)
                };
                *self = PaneNode::Split(PaneSplit {
                    direction,
                    ratio: 0.5,
                    first: Box::new(first),
                    second: Box::new(second),
                });
                true
            }
            PaneNode::Leaf(_) => false,
            PaneNode::Split(split) => {
                if split.first.contains_leaf(target_id) {
                    split
                        .first
                        .split_with_node(target_id, direction, new_node, reverse)
                } else {
                    split
                        .second
                        .split_with_node(target_id, direction, new_node, reverse)
                }
            }
        }
    }

    /// Splits the leaf identified by `target_id` into two panes.
    ///
    /// The original leaf becomes the first child of a new split, and a fresh
    /// leaf with `new_id` becomes the second child.  Returns `Some(new_id)` on
    /// success, `None` if `target_id` was not found.
    #[cfg(test)]
    pub(super) fn split(
        &mut self,
        target_id: PaneId,
        direction: SplitDirection,
        new_id: PaneId,
    ) -> Option<PaneId> {
        match self {
            PaneNode::Leaf(leaf) if leaf.id == target_id => {
                // Swap out the current node with a temporary, build a Split
                // containing the original leaf + a new sibling, then put it back.
                let original = std::mem::replace(self, PaneNode::new_leaf(0));
                let new_sibling = PaneNode::new_leaf(new_id);
                *self = PaneNode::Split(PaneSplit {
                    direction,
                    ratio: 0.5,
                    first: Box::new(original),
                    second: Box::new(new_sibling),
                });
                Some(new_id)
            }
            PaneNode::Leaf(_) => None,
            PaneNode::Split(split) => split
                .first
                .split(target_id, direction, new_id)
                .or_else(|| split.second.split(target_id, direction, new_id)),
        }
    }

    /// Tests whether a pixel coordinate hits a divider in this pane tree.
    ///
    /// Returns `Some(DividerHit)` if `(px, py)` is within `hit_zone` pixels of
    /// a divider, `None` otherwise.  For leaf nodes, always returns `None`.
    #[allow(dead_code)] // Used in later tasks for resize dragging
    pub(super) fn hit_test_divider(
        &self,
        px: u32,
        py: u32,
        rect: PaneRect,
        divider_px: u32,
        hit_zone: u32,
    ) -> Option<DividerHit> {
        let PaneNode::Split(split) = self else {
            return None;
        };

        let (first_rect, second_rect) =
            split_rect(rect, split.direction, split.ratio, divider_px);

        // Compute divider position and check hit.
        match split.direction {
            SplitDirection::Horizontal => {
                let divider_x = first_rect.x + first_rect.width;
                let available = rect.width.saturating_sub(divider_px);
                // Check if click is within hit_zone of divider center and within rect bounds.
                let divider_center = divider_x + divider_px / 2;
                let in_zone_x = (px as i64 - divider_center as i64).unsigned_abs() <= hit_zone as u64;
                let in_bounds_y = py >= rect.y && py < rect.y + rect.height;
                if in_zone_x && in_bounds_y {
                    return Some(DividerHit {
                        direction: split.direction,
                        ratio: split.ratio,
                        position: divider_x,
                        available_size: available,
                    });
                }
            }
            SplitDirection::Vertical => {
                let divider_y = first_rect.y + first_rect.height;
                let available = rect.height.saturating_sub(divider_px);
                let divider_center = divider_y + divider_px / 2;
                let in_zone_y = (py as i64 - divider_center as i64).unsigned_abs() <= hit_zone as u64;
                let in_bounds_x = px >= rect.x && px < rect.x + rect.width;
                if in_zone_y && in_bounds_x {
                    return Some(DividerHit {
                        direction: split.direction,
                        ratio: split.ratio,
                        position: divider_y,
                        available_size: available,
                    });
                }
            }
        }

        // Not on this divider — recurse into children.
        split
            .first
            .hit_test_divider(px, py, first_rect, divider_px, hit_zone)
            .or_else(|| {
                split
                    .second
                    .hit_test_divider(px, py, second_rect, divider_px, hit_zone)
            })
    }

    /// Finds which leaf pane contains the given pixel coordinate.
    ///
    /// Returns `Some(PaneId)` if the pixel is inside a leaf's bounding rect,
    /// `None` otherwise.
    #[allow(dead_code)] // Used in later tasks for focus-follows-mouse
    pub(super) fn pane_at_pixel(
        &self,
        px: u32,
        py: u32,
        rect: PaneRect,
        divider_px: u32,
    ) -> Option<PaneId> {
        match self {
            PaneNode::Leaf(leaf) => {
                if px >= rect.x
                    && px < rect.x + rect.width
                    && py >= rect.y
                    && py < rect.y + rect.height
                {
                    Some(leaf.id)
                } else {
                    None
                }
            }
            PaneNode::Split(split) => {
                let (first_rect, second_rect) =
                    split_rect(rect, split.direction, split.ratio, divider_px);
                split
                    .first
                    .pane_at_pixel(px, py, first_rect, divider_px)
                    .or_else(|| split.second.pane_at_pixel(px, py, second_rect, divider_px))
            }
        }
    }

    /// Finds the divider at `(px, py)` and updates its ratio so the divider
    /// moves to `new_pixel_pos`.  Clamps the ratio so that each child pane
    /// is at least 20 px in the split dimension.  Returns `true` if a divider
    /// was found and updated.
    #[allow(dead_code)] // Used in later tasks for resize dragging
    pub(super) fn resize_divider_at(
        &mut self,
        px: u32,
        py: u32,
        rect: PaneRect,
        divider_px: u32,
        hit_zone: u32,
        new_pixel_pos: u32,
    ) -> bool {
        let PaneNode::Split(split) = self else {
            return false;
        };

        let (first_rect, _second_rect) =
            split_rect(rect, split.direction, split.ratio, divider_px);

        // Check if the click is on this node's divider.
        let is_this_divider = match split.direction {
            SplitDirection::Horizontal => {
                let divider_x = first_rect.x + first_rect.width;
                let divider_center = divider_x + divider_px / 2;
                let in_zone_x =
                    (px as i64 - divider_center as i64).unsigned_abs() <= hit_zone as u64;
                let in_bounds_y = py >= rect.y && py < rect.y + rect.height;
                in_zone_x && in_bounds_y
            }
            SplitDirection::Vertical => {
                let divider_y = first_rect.y + first_rect.height;
                let divider_center = divider_y + divider_px / 2;
                let in_zone_y =
                    (py as i64 - divider_center as i64).unsigned_abs() <= hit_zone as u64;
                let in_bounds_x = px >= rect.x && px < rect.x + rect.width;
                in_zone_y && in_bounds_x
            }
        };

        if is_this_divider {
            let min_pane = 20u32;
            match split.direction {
                SplitDirection::Horizontal => {
                    let available = rect.width.saturating_sub(divider_px);
                    if available == 0 {
                        return true;
                    }
                    let first_w =
                        new_pixel_pos.saturating_sub(rect.x).min(available).max(min_pane);
                    let first_w = first_w.min(available.saturating_sub(min_pane));
                    split.ratio = first_w as f32 / available as f32;
                }
                SplitDirection::Vertical => {
                    let available = rect.height.saturating_sub(divider_px);
                    if available == 0 {
                        return true;
                    }
                    let first_h =
                        new_pixel_pos.saturating_sub(rect.y).min(available).max(min_pane);
                    let first_h = first_h.min(available.saturating_sub(min_pane));
                    split.ratio = first_h as f32 / available as f32;
                }
            }
            return true;
        }

        // Recurse into children.
        let PaneNode::Split(split) = self else {
            unreachable!();
        };
        let (first_rect, second_rect) =
            split_rect(rect, split.direction, split.ratio, divider_px);
        split
            .first
            .resize_divider_at(px, py, first_rect, divider_px, hit_zone, new_pixel_pos)
            || split
                .second
                .resize_divider_at(px, py, second_rect, divider_px, hit_zone, new_pixel_pos)
    }

    /// Closes the leaf identified by `target_id`, replacing the parent split
    /// with the surviving sibling.
    ///
    /// Returns `false` if:
    /// - `target_id` is the only remaining leaf (root leaf), or
    /// - `target_id` was not found.
    pub(super) fn close(&mut self, target_id: PaneId) -> bool {
        // Cannot close the root leaf.
        if let PaneNode::Leaf(_) = self {
            return false;
        }

        // We need to check if either direct child is the target leaf and if so
        // replace `self` (the Split) with the surviving sibling.
        // We use a take-and-reassemble pattern to satisfy the borrow checker.
        self.close_inner(target_id)
    }

    /// Internal recursive close implementation.
    fn close_inner(&mut self, target_id: PaneId) -> bool {
        // Only splits can contain a target to close.
        let PaneNode::Split(_) = self else {
            return false;
        };

        // Take ownership of self temporarily.  We replace *self with a
        // zero-cost Leaf placeholder that will be overwritten immediately.
        // Since we own `self` as `&mut`, we must put something valid back.
        // We reconstruct below in all paths.
        let node = std::mem::replace(
            self,
            PaneNode::Leaf(PaneLeaf {
                id: u64::MAX,
                terminal: Terminal::new(1, 1),
                session: None,
                pty_writer: Box::new(std::io::sink()),
                selection: None,
                scroll_offset: 0,
                security: SecurityGuard::default(),
                scrollbar: ScrollbarState::new(),
            }),
        );

        let PaneNode::Split(split) = node else {
            unreachable!("close_inner: node was verified as Split before mem::replace");
        };

        let PaneSplit {
            first,
            second,
            direction,
            ratio,
        } = split;

        // Check if target is the first child.
        if matches!(first.as_ref(), PaneNode::Leaf(l) if l.id == target_id) {
            *self = *second;
            return true;
        }

        // Check if target is the second child.
        if matches!(second.as_ref(), PaneNode::Leaf(l) if l.id == target_id) {
            *self = *first;
            return true;
        }

        // Target is not a direct child — reconstruct the split and recurse.
        let mut reconstructed = PaneNode::Split(PaneSplit {
            direction,
            ratio,
            first,
            second,
        });

        let found = match &mut reconstructed {
            PaneNode::Split(s) => s.first.close_inner(target_id) || s.second.close_inner(target_id),
            _ => false,
        };

        *self = reconstructed;
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_leaf_find() {
        let tree = PaneNode::new_leaf(1);
        assert!(tree.find_leaf(1).is_some());
        assert_eq!(tree.find_leaf(1).unwrap().id, 1);
        assert!(tree.find_leaf(99).is_none());
    }

    #[test]
    fn single_leaf_is_leaf() {
        let tree = PaneNode::new_leaf(1);
        assert!(tree.is_leaf());
        assert_eq!(tree.leaf_count(), 1);
    }

    #[test]
    fn split_creates_two_leaves() {
        let mut tree = PaneNode::new_leaf(1);
        let new_id = tree.split(1, SplitDirection::Horizontal, 2);
        assert_eq!(new_id, Some(2));

        assert!(!tree.is_leaf());
        assert_eq!(tree.leaf_count(), 2);

        assert!(tree.find_leaf(1).is_some());
        assert!(tree.find_leaf(2).is_some());

        if let PaneNode::Split(split) = &tree {
            assert_eq!(split.direction, SplitDirection::Horizontal);
            assert!((split.ratio - 0.5).abs() < f32::EPSILON);
        } else {
            panic!("Expected Split node after split");
        }
    }

    #[test]
    fn split_nonexistent_returns_none() {
        let mut tree = PaneNode::new_leaf(1);
        assert_eq!(tree.split(99, SplitDirection::Vertical, 2), None);

        // Tree should be unchanged.
        assert!(tree.is_leaf());
        assert_eq!(tree.leaf_count(), 1);
    }

    #[test]
    fn close_leaf_returns_sibling() {
        let mut tree = PaneNode::new_leaf(1);
        assert!(tree.split(1, SplitDirection::Horizontal, 2).is_some());
        assert_eq!(tree.leaf_count(), 2);

        // Close leaf 2, should leave leaf 1.
        assert!(tree.close(2));
        assert!(tree.is_leaf());
        assert_eq!(tree.find_leaf(1).unwrap().id, 1);
    }

    #[test]
    fn close_only_leaf_fails() {
        let mut tree = PaneNode::new_leaf(1);
        assert!(!tree.close(1));
        // Tree should be unchanged.
        assert!(tree.is_leaf());
        assert_eq!(tree.leaf_count(), 1);
    }

    #[test]
    fn nested_split_and_close() {
        // Build: split 1 -> (1, 2), then split 2 -> (2, 3)
        let mut tree = PaneNode::new_leaf(1);
        assert!(tree.split(1, SplitDirection::Horizontal, 2).is_some());
        assert!(tree.split(2, SplitDirection::Vertical, 3).is_some());
        assert_eq!(tree.leaf_count(), 3);

        // Close leaf 2 — its parent split should collapse, leaving 3 in its place.
        assert!(tree.close(2));
        assert_eq!(tree.leaf_count(), 2);

        // Leaves 1 and 3 should still be accessible.
        assert!(tree.find_leaf(1).is_some());
        assert!(tree.find_leaf(3).is_some());
        assert!(tree.find_leaf(2).is_none());
    }

    #[test]
    fn close_first_child_returns_second() {
        let mut tree = PaneNode::new_leaf(1);
        assert!(tree.split(1, SplitDirection::Horizontal, 2).is_some());
        // Close leaf 1 (first child), should leave leaf 2.
        assert!(tree.close(1));
        assert!(tree.is_leaf());
        assert_eq!(tree.find_leaf(2).unwrap().id, 2);
        assert!(tree.find_leaf(1).is_none());
    }

    #[test]
    fn close_nonexistent_from_split() {
        let mut tree = PaneNode::new_leaf(1);
        assert!(tree.split(1, SplitDirection::Horizontal, 2).is_some());
        // Try to close nonexistent ID — should fail, tree unchanged.
        assert!(!tree.close(999));
        assert_eq!(tree.leaf_count(), 2);
        assert!(tree.find_leaf(1).is_some());
        assert!(tree.find_leaf(2).is_some());
    }

    #[test]
    fn find_leaf_mut_modifies_terminal() {
        let mut tree = PaneNode::new_leaf(1);
        // Modify terminal via find_leaf_mut.
        let leaf = tree.find_leaf_mut(1).unwrap();
        leaf.scroll_offset = 42;
        // Verify the change persisted.
        assert_eq!(tree.find_leaf(1).unwrap().scroll_offset, 42);
    }

    #[test]
    fn deep_nesting_split_and_close() {
        // Build: [1] -> [1|2] -> [1|(2/3)] -> [1|((2/4)/3)]
        let mut tree = PaneNode::new_leaf(1);
        assert!(tree.split(1, SplitDirection::Horizontal, 2).is_some());
        assert!(tree.split(2, SplitDirection::Vertical, 3).is_some());
        assert!(tree.split(2, SplitDirection::Vertical, 4).is_some());
        assert_eq!(tree.leaf_count(), 4);

        // Close deepest leaf (4).
        assert!(tree.close(4));
        assert_eq!(tree.leaf_count(), 3);
        assert!(tree.find_leaf(4).is_none());
        assert!(tree.find_leaf(2).is_some());
    }

    #[test]
    fn leaf_ids_returns_all() {
        let mut tree = PaneNode::new_leaf(1);
        tree.split(1, SplitDirection::Horizontal, 2);
        tree.split(2, SplitDirection::Vertical, 3);

        let mut ids = tree.leaf_ids();
        ids.sort();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    // --- Layout and navigation tests ---

    #[test]
    fn layout_single_pane() {
        let tree = PaneNode::new_leaf(1);
        let rect = PaneRect {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        };
        let layout = tree.layout(rect, 1);
        assert_eq!(layout.len(), 1);
        assert_eq!(layout[0].0, 1);
        assert_eq!(layout[0].1, rect);
    }

    #[test]
    fn layout_horizontal_split() {
        let mut tree = PaneNode::new_leaf(1);
        tree.split(1, SplitDirection::Horizontal, 2);
        let rect = PaneRect {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        };
        let layout = tree.layout(rect, 1);
        assert_eq!(layout.len(), 2);
        assert_eq!(layout[0].0, 1);
        assert!(layout[0].1.width < 400);
        assert_eq!(layout[1].0, 2);
        assert!(layout[1].1.x > 0);
    }

    #[test]
    fn layout_vertical_split() {
        let mut tree = PaneNode::new_leaf(1);
        tree.split(1, SplitDirection::Vertical, 2);
        let rect = PaneRect {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        };
        let layout = tree.layout(rect, 1);
        assert_eq!(layout.len(), 2);
        assert_eq!(layout[0].0, 1);
        assert!(layout[0].1.height < 300);
        assert_eq!(layout[1].0, 2);
        assert!(layout[1].1.y > 0);
    }

    #[test]
    fn navigate_horizontal_right() {
        let mut tree = PaneNode::new_leaf(1);
        tree.split(1, SplitDirection::Horizontal, 2);
        let rect = PaneRect {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        };
        let layout = tree.layout(rect, 1);
        let target = PaneNode::navigate_spatial(&layout, 1, NavigateDirection::Right);
        assert_eq!(target, Some(2));
    }

    #[test]
    fn navigate_horizontal_left() {
        let mut tree = PaneNode::new_leaf(1);
        tree.split(1, SplitDirection::Horizontal, 2);
        let rect = PaneRect {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        };
        let layout = tree.layout(rect, 1);
        let target = PaneNode::navigate_spatial(&layout, 2, NavigateDirection::Left);
        assert_eq!(target, Some(1));
    }

    #[test]
    fn navigate_no_neighbor() {
        let tree = PaneNode::new_leaf(1);
        let rect = PaneRect {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        };
        let layout = tree.layout(rect, 1);
        assert_eq!(
            PaneNode::navigate_spatial(&layout, 1, NavigateDirection::Right),
            None
        );
    }

    // --- Divider hit-testing tests ---

    #[test]
    fn divider_hit_horizontal() {
        let mut tree = PaneNode::new_leaf(1);
        tree.split(1, SplitDirection::Horizontal, 2);
        let rect = PaneRect {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        };
        let divider_px = 1;
        let layout = tree.layout(rect, divider_px);
        // Divider is right after the first pane's width
        let first_w = layout[0].1.width;
        let hit = tree.hit_test_divider(first_w, 300, rect, divider_px, 6);
        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert_eq!(hit.direction, SplitDirection::Horizontal);
    }

    #[test]
    fn divider_hit_miss() {
        let mut tree = PaneNode::new_leaf(1);
        tree.split(1, SplitDirection::Horizontal, 2);
        let rect = PaneRect {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        };
        // Click far from divider
        let hit = tree.hit_test_divider(100, 300, rect, 1, 6);
        assert!(hit.is_none());
    }
}
