use std::io::Write;

use crate::core::terminal::Terminal;
use crate::core::{SecurityGuard, Selection};
use crate::gui::state::ScrollbarState;
use crate::pty;

/// Unique identifier for a pane within a tab.
pub(super) type PaneId = u64;

/// Direction in which a pane is split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SplitDirection {
    /// Left | Right
    Horizontal,
    /// Top / Bottom
    Vertical,
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

    /// Recursively counts the number of leaf nodes in this tree.
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
}
