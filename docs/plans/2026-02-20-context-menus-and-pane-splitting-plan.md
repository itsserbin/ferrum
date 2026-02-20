# Context Menus & Pane Splitting Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace auto-paste on right-click with OS-native context menus and add recursive binary pane splitting to Ferrum terminal.

**Architecture:** Binary tree of panes per tab (Ghostty-style). Each leaf node holds its own Terminal + PTY session. Native context menus via `muda` crate with platform-specific display (NSMenu/Win32/GTK). Rendering adapted to iterate pane layout tree instead of single terminal per tab.

**Tech Stack:** Rust, winit 0.30, muda (native menus), wgpu/softbuffer (rendering), portable-pty

**Design doc:** `docs/plans/2026-02-20-context-menus-and-pane-splitting-design.md`

---

## Phase 1: PaneNode Data Structure

Pure data structures and tree operations. No GUI changes yet — fully testable in isolation.

### Task 1: Create PaneNode types and basic tree operations

**Files:**
- Create: `src/gui/pane.rs`
- Modify: `src/gui/mod.rs:1` (add module declaration)

**Step 1: Write the failing tests**

Create `src/gui/pane.rs` with test module first:

```rust
pub(super) type PaneId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SplitDirection {
    Horizontal, // left | right
    Vertical,   // top / bottom
}

// Stub types — will be filled in step 3
pub(super) struct PaneNode;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_leaf_find() {
        let tree = PaneNode::new_leaf(1);
        assert!(tree.find_leaf(1).is_some());
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
    }

    #[test]
    fn split_nonexistent_returns_none() {
        let mut tree = PaneNode::new_leaf(1);
        assert_eq!(tree.split(99, SplitDirection::Horizontal, 2), None);
    }

    #[test]
    fn close_leaf_returns_sibling() {
        let mut tree = PaneNode::new_leaf(1);
        tree.split(1, SplitDirection::Horizontal, 2);
        let closed = tree.close(2);
        assert!(closed);
        assert!(tree.is_leaf());
        assert!(tree.find_leaf(1).is_some());
    }

    #[test]
    fn close_only_leaf_fails() {
        let mut tree = PaneNode::new_leaf(1);
        assert!(!tree.close(1));
    }

    #[test]
    fn nested_split_and_close() {
        // Start: [1]
        // Split 1 right -> [1 | 2]
        // Split 1 down  -> [(1 / 3) | 2]
        // Close 1        -> [3 | 2]
        let mut tree = PaneNode::new_leaf(1);
        tree.split(1, SplitDirection::Horizontal, 2);
        tree.split(1, SplitDirection::Vertical, 3);
        assert_eq!(tree.leaf_count(), 3);

        tree.close(1);
        assert_eq!(tree.leaf_count(), 2);
        assert!(tree.find_leaf(3).is_some());
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test pane::tests -q`
Expected: FAIL — `PaneNode` has no methods yet

**Step 3: Implement PaneNode**

Replace the stub `PaneNode` struct with the full implementation in `src/gui/pane.rs`:

```rust
use std::io::Write;

use crate::core::terminal::Terminal;
use crate::core::{SecurityGuard, Selection};
use crate::gui::state::ScrollbarState;
use crate::pty;

pub(super) type PaneId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SplitDirection {
    Horizontal, // left | right
    Vertical,   // top / bottom
}

pub(super) struct PaneLeaf {
    pub(super) id: PaneId,
    pub(super) terminal: Terminal,
    pub(super) session: pty::Session,
    pub(super) pty_writer: Box<dyn Write + Send>,
    pub(super) selection: Option<Selection>,
    pub(super) scroll_offset: usize,
    pub(super) security: SecurityGuard,
    pub(super) scrollbar: ScrollbarState,
}

pub(super) struct PaneSplit {
    pub(super) direction: SplitDirection,
    pub(super) ratio: f32,
    pub(super) first: Box<PaneNode>,
    pub(super) second: Box<PaneNode>,
}

pub(super) enum PaneNode {
    Leaf(PaneLeaf),
    Split(PaneSplit),
}

impl PaneNode {
    /// Creates a test-only leaf with just an ID (no PTY).
    #[cfg(test)]
    pub fn new_leaf(id: PaneId) -> Self {
        // Tests use a minimal stub; real code uses new_leaf_with_pty.
        PaneNode::Leaf(PaneLeaf {
            id,
            terminal: Terminal::new(24, 80),
            session: unsafe { std::mem::zeroed() }, // test-only
            pty_writer: Box::new(std::io::sink()),
            selection: None,
            scroll_offset: 0,
            security: SecurityGuard::new(),
            scrollbar: ScrollbarState::new(),
        })
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self, PaneNode::Leaf(_))
    }

    pub fn leaf_count(&self) -> usize {
        match self {
            PaneNode::Leaf(_) => 1,
            PaneNode::Split(s) => s.first.leaf_count() + s.second.leaf_count(),
        }
    }

    pub fn leaf_ids(&self) -> Vec<PaneId> {
        match self {
            PaneNode::Leaf(leaf) => vec![leaf.id],
            PaneNode::Split(s) => {
                let mut ids = s.first.leaf_ids();
                ids.extend(s.second.leaf_ids());
                ids
            }
        }
    }

    pub fn find_leaf(&self, id: PaneId) -> Option<&PaneLeaf> {
        match self {
            PaneNode::Leaf(leaf) if leaf.id == id => Some(leaf),
            PaneNode::Leaf(_) => None,
            PaneNode::Split(s) => s.first.find_leaf(id).or_else(|| s.second.find_leaf(id)),
        }
    }

    pub fn find_leaf_mut(&mut self, id: PaneId) -> Option<&mut PaneLeaf> {
        match self {
            PaneNode::Leaf(leaf) if leaf.id == id => Some(leaf),
            PaneNode::Leaf(_) => None,
            PaneNode::Split(s) => s
                .first
                .find_leaf_mut(id)
                .or_else(|| s.second.find_leaf_mut(id)),
        }
    }

    /// Split the leaf with `target_id` into two. Returns the new leaf's ID,
    /// or None if target was not found.
    pub fn split(
        &mut self,
        target_id: PaneId,
        direction: SplitDirection,
        new_id: PaneId,
    ) -> Option<PaneId> {
        self.split_inner(target_id, direction, new_id)
    }

    fn split_inner(
        &mut self,
        target_id: PaneId,
        direction: SplitDirection,
        new_id: PaneId,
    ) -> Option<PaneId> {
        match self {
            PaneNode::Leaf(leaf) if leaf.id == target_id => {
                // Replace self with Split(original, new_leaf)
                let original = std::mem::replace(self, PaneNode::new_leaf(0)); // placeholder
                let new_leaf = PaneNode::new_leaf(new_id);

                let (first, second) = match direction {
                    SplitDirection::Horizontal | SplitDirection::Vertical => {
                        // For Right/Down: original is first, new is second
                        // For Left/Up: new is first, original is second
                        // Simplified: always original first (direction variants
                        // determine visual layout during rendering)
                        (original, new_leaf)
                    }
                };

                *self = PaneNode::Split(PaneSplit {
                    direction,
                    ratio: 0.5,
                    first: Box::new(first),
                    second: Box::new(second),
                });
                Some(new_id)
            }
            PaneNode::Leaf(_) => None,
            PaneNode::Split(s) => s
                .first
                .split_inner(target_id, direction, new_id)
                .or_else(|| s.second.split_inner(target_id, direction, new_id)),
        }
    }

    /// Close a leaf by ID. The sibling replaces the parent Split node.
    /// Returns true if the leaf was found and closed.
    pub fn close(&mut self, target_id: PaneId) -> bool {
        // Can't close the root leaf
        if let PaneNode::Leaf(leaf) = self {
            return leaf.id != target_id && false;
        }
        self.close_inner(target_id)
    }

    fn close_inner(&mut self, target_id: PaneId) -> bool {
        let PaneNode::Split(split) = self else {
            return false;
        };

        // Check if target is a direct child
        if let PaneNode::Leaf(leaf) = split.first.as_ref() {
            if leaf.id == target_id {
                let sibling = std::mem::replace(
                    split.second.as_mut(),
                    PaneNode::new_leaf(0),
                );
                *self = sibling;
                return true;
            }
        }
        if let PaneNode::Leaf(leaf) = split.second.as_ref() {
            if leaf.id == target_id {
                let sibling = std::mem::replace(
                    split.first.as_mut(),
                    PaneNode::new_leaf(0),
                );
                *self = sibling;
                return true;
            }
        }

        // Recurse into children
        split.first.close_inner(target_id) || split.second.close_inner(target_id)
    }
}
```

Note: The `#[cfg(test)] new_leaf` uses `std::mem::zeroed()` for Session which is unsafe — this is test-only. Real pane creation uses `new_leaf_with_pty` (Task 3).

**Step 4: Add module declaration**

In `src/gui/mod.rs`, add after line 1 (`mod events;`):

```rust
mod pane;
```

So the top becomes:
```rust
mod events;
mod input;
mod interaction;
mod lifecycle;
mod pane;
mod platform;
mod renderer;
mod state;
mod tabs;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test pane::tests -q`
Expected: All 7 tests PASS

**Step 6: Commit**

```bash
git add src/gui/pane.rs src/gui/mod.rs
git commit -m "feat: add PaneNode binary tree data structure with split/close/find operations"
```

---

### Task 2: Add PaneLayout calculation and spatial navigation

**Files:**
- Modify: `src/gui/pane.rs` (add layout + navigate)

**Step 1: Write the failing tests**

Add to the `tests` module in `src/gui/pane.rs`:

```rust
#[test]
fn layout_single_pane() {
    let tree = PaneNode::new_leaf(1);
    let rect = PaneRect { x: 0, y: 0, width: 800, height: 600 };
    let layout = tree.layout(rect, 1);
    assert_eq!(layout.len(), 1);
    assert_eq!(layout[0].0, 1);
    assert_eq!(layout[0].1, rect);
}

#[test]
fn layout_horizontal_split() {
    let mut tree = PaneNode::new_leaf(1);
    tree.split(1, SplitDirection::Horizontal, 2);
    let rect = PaneRect { x: 0, y: 0, width: 800, height: 600 };
    let layout = tree.layout(rect, 1);
    assert_eq!(layout.len(), 2);
    // First pane: left half (minus divider)
    assert_eq!(layout[0].0, 1);
    assert!(layout[0].1.width < 400);
    // Second pane: right half
    assert_eq!(layout[1].0, 2);
    assert!(layout[1].1.x > 0);
}

#[test]
fn layout_vertical_split() {
    let mut tree = PaneNode::new_leaf(1);
    tree.split(1, SplitDirection::Vertical, 2);
    let rect = PaneRect { x: 0, y: 0, width: 800, height: 600 };
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
    let rect = PaneRect { x: 0, y: 0, width: 800, height: 600 };
    let layout = tree.layout(rect, 1);
    let target = PaneNode::navigate_spatial(&layout, 1, NavigateDirection::Right);
    assert_eq!(target, Some(2));
}

#[test]
fn navigate_horizontal_left() {
    let mut tree = PaneNode::new_leaf(1);
    tree.split(1, SplitDirection::Horizontal, 2);
    let rect = PaneRect { x: 0, y: 0, width: 800, height: 600 };
    let layout = tree.layout(rect, 1);
    let target = PaneNode::navigate_spatial(&layout, 2, NavigateDirection::Left);
    assert_eq!(target, Some(1));
}

#[test]
fn navigate_no_neighbor() {
    let tree = PaneNode::new_leaf(1);
    let rect = PaneRect { x: 0, y: 0, width: 800, height: 600 };
    let layout = tree.layout(rect, 1);
    assert_eq!(PaneNode::navigate_spatial(&layout, 1, NavigateDirection::Right), None);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test pane::tests -q`
Expected: FAIL — `PaneRect`, `NavigateDirection`, `layout`, `navigate_spatial` don't exist

**Step 3: Implement layout and navigation**

Add to `src/gui/pane.rs` (before the tests module):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PaneRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PaneRect {
    pub fn center_x(&self) -> u32 {
        self.x + self.width / 2
    }

    pub fn center_y(&self) -> u32 {
        self.y + self.height / 2
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NavigateDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Width of the divider between panes in pixels (before DPI scaling).
pub(super) const DIVIDER_WIDTH: u32 = 1;
/// Hit-zone width for mouse interaction with dividers.
pub(super) const DIVIDER_HIT_ZONE: u32 = 6;

impl PaneNode {
    /// Calculate the layout rectangles for all leaf panes within the given area.
    /// `divider_px` is the divider width in physical pixels (already DPI-scaled).
    pub fn layout(&self, rect: PaneRect, divider_px: u32) -> Vec<(PaneId, PaneRect)> {
        let mut result = Vec::new();
        self.layout_inner(rect, divider_px, &mut result);
        result
    }

    fn layout_inner(
        &self,
        rect: PaneRect,
        divider_px: u32,
        out: &mut Vec<(PaneId, PaneRect)>,
    ) {
        match self {
            PaneNode::Leaf(leaf) => out.push((leaf.id, rect)),
            PaneNode::Split(split) => {
                let (first_rect, second_rect) = split_rect(rect, split.direction, split.ratio, divider_px);
                split.first.layout_inner(first_rect, divider_px, out);
                split.second.layout_inner(second_rect, divider_px, out);
            }
        }
    }

    /// Find the nearest pane in a given direction using spatial positions.
    pub fn navigate_spatial(
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

            let valid = match direction {
                NavigateDirection::Right => cx > from_cx,
                NavigateDirection::Left => cx < from_cx,
                NavigateDirection::Down => cy > from_cy,
                NavigateDirection::Up => cy < from_cy,
            };

            if !valid {
                continue;
            }

            let dist = (cx - from_cx).abs() + (cy - from_cy).abs();
            if best.is_none() || dist < best.unwrap().1 {
                best = Some((id, dist));
            }
        }

        best.map(|(id, _)| id)
    }
}

/// Split a rectangle into two based on direction and ratio.
fn split_rect(
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test pane::tests -q`
Expected: All 13 tests PASS

**Step 5: Commit**

```bash
git add src/gui/pane.rs
git commit -m "feat: add pane layout calculation and spatial navigation"
```

---

### Task 3: Add divider hit-testing

**Files:**
- Modify: `src/gui/pane.rs`

**Step 1: Write the failing tests**

Add to tests:

```rust
#[test]
fn divider_hit_horizontal() {
    let mut tree = PaneNode::new_leaf(1);
    tree.split(1, SplitDirection::Horizontal, 2);
    let rect = PaneRect { x: 0, y: 0, width: 800, height: 600 };
    let divider_px = 1;
    let layout = tree.layout(rect, divider_px);
    // Divider is at x = 399 (first pane width)
    let first_w = layout[0].1.width;
    let hit = tree.hit_test_divider(first_w + divider_px / 2, 300, rect, divider_px, 6);
    assert!(hit.is_some());
}

#[test]
fn divider_hit_miss() {
    let mut tree = PaneNode::new_leaf(1);
    tree.split(1, SplitDirection::Horizontal, 2);
    let rect = PaneRect { x: 0, y: 0, width: 800, height: 600 };
    // Click far from divider
    let hit = tree.hit_test_divider(100, 300, rect, 1, 6);
    assert!(hit.is_none());
}
```

**Step 2: Run tests, verify fail**

Run: `cargo test pane::tests -q`
Expected: FAIL — `hit_test_divider` doesn't exist

**Step 3: Implement divider hit-testing**

Add to PaneNode impl:

```rust
/// Information about a divider that was hit by a mouse click.
pub(super) struct DividerHit {
    /// The split node path to reach this divider.
    pub direction: SplitDirection,
    /// Current ratio of the split.
    pub ratio: f32,
    /// The pixel position of the divider (x for horizontal, y for vertical).
    pub position: u32,
    /// The available size for the split (width for horizontal, height for vertical).
    pub available_size: u32,
}

impl PaneNode {
    /// Test if a pixel coordinate hits a divider. Returns DividerHit if so.
    pub fn hit_test_divider(
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

        let (divider_pos, in_hit_zone) = match split.direction {
            SplitDirection::Horizontal => {
                let available = rect.width.saturating_sub(divider_px);
                let first_w = (available as f32 * split.ratio) as u32;
                let div_x = rect.x + first_w;
                let in_zone = px >= div_x.saturating_sub(hit_zone / 2)
                    && px <= div_x + divider_px + hit_zone / 2
                    && py >= rect.y
                    && py < rect.y + rect.height;
                (div_x, in_zone)
            }
            SplitDirection::Vertical => {
                let available = rect.height.saturating_sub(divider_px);
                let first_h = (available as f32 * split.ratio) as u32;
                let div_y = rect.y + first_h;
                let in_zone = py >= div_y.saturating_sub(hit_zone / 2)
                    && py <= div_y + divider_px + hit_zone / 2
                    && px >= rect.x
                    && px < rect.x + rect.width;
                (div_y, in_zone)
            }
        };

        if in_hit_zone {
            let available_size = match split.direction {
                SplitDirection::Horizontal => rect.width.saturating_sub(divider_px),
                SplitDirection::Vertical => rect.height.saturating_sub(divider_px),
            };
            return Some(DividerHit {
                direction: split.direction,
                ratio: split.ratio,
                position: divider_pos,
                available_size,
            });
        }

        // Recurse into children
        let (first_rect, second_rect) = split_rect(rect, split.direction, split.ratio, divider_px);
        split
            .first
            .hit_test_divider(px, py, first_rect, divider_px, hit_zone)
            .or_else(|| {
                split
                    .second
                    .hit_test_divider(px, py, second_rect, divider_px, hit_zone)
            })
    }

    /// Find which leaf pane contains the given pixel coordinate.
    pub fn pane_at_pixel(
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
                    .or_else(|| {
                        split
                            .second
                            .pane_at_pixel(px, py, second_rect, divider_px)
                    })
            }
        }
    }

    /// Update the ratio of the split node whose divider is at the given position.
    /// `new_pixel_pos` is the new position in pixels.
    pub fn resize_divider_at(
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

        let (divider_pos, in_hit_zone) = match split.direction {
            SplitDirection::Horizontal => {
                let available = rect.width.saturating_sub(divider_px);
                let first_w = (available as f32 * split.ratio) as u32;
                let div_x = rect.x + first_w;
                let in_zone = px >= div_x.saturating_sub(hit_zone / 2)
                    && px <= div_x + divider_px + hit_zone / 2;
                (div_x, in_zone)
            }
            SplitDirection::Vertical => {
                let available = rect.height.saturating_sub(divider_px);
                let first_h = (available as f32 * split.ratio) as u32;
                let div_y = rect.y + first_h;
                let in_zone = py >= div_y.saturating_sub(hit_zone / 2)
                    && py <= div_y + divider_px + hit_zone / 2;
                (div_y, in_zone)
            }
        };

        if in_hit_zone {
            let available = match split.direction {
                SplitDirection::Horizontal => rect.width.saturating_sub(divider_px),
                SplitDirection::Vertical => rect.height.saturating_sub(divider_px),
            };
            let origin = match split.direction {
                SplitDirection::Horizontal => rect.x,
                SplitDirection::Vertical => rect.y,
            };
            if available > 0 {
                let clamped = new_pixel_pos.clamp(origin + 20, origin + available - 20);
                split.ratio = (clamped - origin) as f32 / available as f32;
            }
            return true;
        }

        let (first_rect, second_rect) = split_rect(rect, split.direction, split.ratio, divider_px);
        split
            .first
            .resize_divider_at(px, py, first_rect, divider_px, hit_zone, new_pixel_pos)
            || split
                .second
                .resize_divider_at(px, py, second_rect, divider_px, hit_zone, new_pixel_pos)
    }
}
```

**Step 4: Run tests**

Run: `cargo test pane::tests -q`
Expected: All 15 tests PASS

**Step 5: Commit**

```bash
git add src/gui/pane.rs
git commit -m "feat: add divider hit-testing, pane-at-pixel lookup, and resize"
```

---

## Phase 2: Refactor TabState to Use PaneNode

This phase is the critical migration. We replace the flat terminal/session fields in TabState with a PaneNode tree while keeping all existing functionality working.

### Task 4: Refactor TabState struct

**Files:**
- Modify: `src/gui/state.rs:7-10` (PtyEvent)
- Modify: `src/gui/state.rs:41-51` (TabState)
- Modify: `src/gui/mod.rs` (add pane imports)

**Step 1: Update PtyEvent to include pane_id**

In `src/gui/state.rs`, change PtyEvent (lines 7-10):

```rust
pub(super) enum PtyEvent {
    Data { tab_id: u64, pane_id: u64, bytes: Vec<u8> },
    Exited { tab_id: u64, pane_id: u64 },
}
```

**Step 2: Refactor TabState**

Replace TabState (lines 41-51) in `src/gui/state.rs`:

```rust
pub(super) struct TabState {
    pub(super) id: u64,
    pub(super) title: String,
    pub(super) pane_tree: crate::gui::pane::PaneNode,
    pub(super) focused_pane: crate::gui::pane::PaneId,
    pub(super) next_pane_id: crate::gui::pane::PaneId,
}
```

**Step 3: Add convenience methods to TabState**

Add after TabState definition in `src/gui/state.rs`:

```rust
impl TabState {
    /// Returns the focused pane leaf.
    pub(super) fn focused_leaf(&self) -> Option<&crate::gui::pane::PaneLeaf> {
        self.pane_tree.find_leaf(self.focused_pane)
    }

    /// Returns the focused pane leaf mutably.
    pub(super) fn focused_leaf_mut(&mut self) -> Option<&mut crate::gui::pane::PaneLeaf> {
        self.pane_tree.find_leaf_mut(self.focused_pane)
    }

    /// Convenience: access to terminal of focused pane.
    pub(super) fn terminal(&self) -> Option<&Terminal> {
        self.focused_leaf().map(|l| &l.terminal)
    }

    /// Convenience: access to terminal of focused pane mutably.
    pub(super) fn terminal_mut(&mut self) -> Option<&mut Terminal> {
        self.focused_leaf_mut().map(|l| &mut l.terminal)
    }

    /// Whether this tab has multiple panes.
    pub(super) fn has_multiple_panes(&self) -> bool {
        !self.pane_tree.is_leaf()
    }
}
```

**Step 4: Fix all compilation errors**

This is the largest step. Every place that accesses `tab.terminal`, `tab.session`, `tab.pty_writer`, `tab.selection`, `tab.scroll_offset`, `tab.security`, `tab.scrollbar` needs to go through the pane tree instead.

The general pattern is:
- `tab.terminal` → `tab.focused_leaf().unwrap().terminal` (or match on `tab.focused_leaf()`)
- `tab.selection` → `tab.focused_leaf().unwrap().selection`
- etc.

**Key files that need updating** (follow compiler errors):

1. `src/gui/events/pty.rs` — PTY data routing now uses `pane_id` to find correct leaf
2. `src/gui/events/render_shared.rs` — FrameParams needs full tab, render iterates panes
3. `src/gui/events/mouse/input.rs` — selection, scrollbar, paste
4. `src/gui/events/mouse/cursor.rs` — hover states
5. `src/gui/events/mouse/terminal_click.rs` — click-to-cursor
6. `src/gui/events/keyboard/` — all keyboard handlers
7. `src/gui/interaction/clipboard.rs` — copy/paste
8. `src/gui/tabs/create.rs` — tab creation builds single-pane tree
9. `src/gui/events/redraw/` — animation scheduling

The approach: fix compile errors one file at a time, starting from `tabs/create.rs` (which builds TabState), then work outward through event handlers.

**Step 5: Update tab creation**

In `src/gui/tabs/create.rs`, update `build_tab_state` to create a single-pane tree:

```rust
fn build_tab_state(
    rows: usize,
    cols: usize,
    title: Option<String>,
    next_tab_id: &mut u64,
    tx: &mpsc::Sender<PtyEvent>,
) -> anyhow::Result<TabState> {
    let tab_id = *next_tab_id;
    *next_tab_id += 1;
    let pane_id: u64 = 0; // First pane in this tab

    let shell = pty::default_shell();
    let session = pty::Session::spawn(&shell, rows as u16, cols as u16)
        .context("failed to spawn PTY session")?;
    let pty_writer = session.writer().context("failed to acquire PTY writer")?;

    // Spawn dedicated PTY reader thread
    let tx_clone = tx.clone();
    let mut reader = session.reader().context("failed to clone PTY reader")?;
    std::thread::Builder::new()
        .name(format!("pty-reader-{}-{}", tab_id, pane_id))
        .spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = tx_clone.send(PtyEvent::Exited {
                            tab_id,
                            pane_id,
                        });
                        break;
                    }
                    Ok(n) => {
                        let _ = tx_clone.send(PtyEvent::Data {
                            tab_id,
                            pane_id,
                            bytes: buf[..n].to_vec(),
                        });
                    }
                    Err(_) => {
                        let _ = tx_clone.send(PtyEvent::Exited {
                            tab_id,
                            pane_id,
                        });
                        break;
                    }
                }
            }
        })
        .context("failed to spawn PTY reader thread")?;

    let mut terminal = Terminal::new(rows, cols);
    {
        let msg = last_login_message();
        terminal.process(msg.as_bytes());
    }

    let leaf = PaneLeaf {
        id: pane_id,
        terminal,
        session,
        pty_writer,
        selection: None,
        scroll_offset: 0,
        security: SecurityGuard::new(),
        scrollbar: ScrollbarState::new(),
    };

    Ok(TabState {
        id: tab_id,
        title: title.unwrap_or_else(|| format!("bash #{}", tab_id + 1)),
        pane_tree: PaneNode::Leaf(leaf),
        focused_pane: pane_id,
        next_pane_id: 1,
    })
}
```

**Step 6: Update PTY event handling**

In `src/gui/events/pty.rs`, update to route by pane_id:

```rust
pub(in crate::gui) fn on_pty_event(&mut self, event: &PtyEvent) {
    match event {
        PtyEvent::Data { tab_id, pane_id, bytes } => {
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == *tab_id) {
                if let Some(leaf) = tab.pane_tree.find_leaf_mut(*pane_id) {
                    leaf.terminal.process(bytes);
                    leaf.scroll_offset = 0;

                    let popped = leaf.terminal.drain_scrollback_popped();
                    if popped > 0 {
                        leaf.selection = leaf
                            .selection
                            .and_then(|sel| sel.adjust_for_scrollback_pop(popped));
                    }

                    for event in leaf.terminal.drain_security_events() {
                        leaf.security.record(event);
                    }

                    let responses = leaf.terminal.drain_responses();
                    if !responses.is_empty() {
                        let _ = leaf.pty_writer.write_all(&responses);
                        let _ = leaf.pty_writer.flush();
                    }
                }
            }
        }
        PtyEvent::Exited { tab_id, pane_id } => {
            // For now, if any pane exits, close it (or close the tab if it's the only one)
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == *tab_id) {
                if tab.pane_tree.is_leaf() {
                    // Single pane — close the entire tab (existing behavior)
                    if let Some(idx) = self.tabs.iter().position(|t| t.id == *tab_id) {
                        let len_before = self.tabs.len();
                        self.adjust_rename_after_tab_remove(idx);
                        self.adjust_security_popup_after_tab_remove(idx);
                        self.tabs.remove(idx);
                        self.refresh_tab_bar_visibility();
                        if self.tabs.is_empty() {
                            self.pending_requests.push(WindowRequest::CloseWindow);
                            return;
                        }
                        self.active_tab = normalized_active_index_after_remove(
                            self.active_tab, len_before, idx,
                        )
                        .unwrap_or(0);
                    }
                } else {
                    // Multiple panes — close just this pane
                    tab.pane_tree.close(*pane_id);
                    // If focused pane was closed, move focus to first available
                    if tab.focused_pane == *pane_id {
                        if let Some(first_id) = tab.pane_tree.leaf_ids().first().copied() {
                            tab.focused_pane = first_id;
                        }
                    }
                }
            }
        }
    }
}
```

**Step 7: Fix remaining compilation errors throughout the codebase**

Work through compiler errors file by file. The pattern for each access is:

```rust
// BEFORE: tab.terminal.foo
// AFTER:  tab.focused_leaf().map(|l| &l.terminal.foo)
//   or:   if let Some(leaf) = tab.focused_leaf() { leaf.terminal.foo }
```

For rendering in `render_shared.rs`, update `FrameParams` and `draw_frame_content` to render the focused pane's terminal (preserving single-pane behavior for now; multi-pane rendering is Phase 4).

**Step 8: Run the full test suite**

Run: `cargo test -q`
Expected: All existing tests PASS (behavior unchanged, just data structure changed)

**Step 9: Run the application manually**

Run: `cargo run`
Expected: Application works identically to before — single terminal per tab, all shortcuts work

**Step 10: Commit**

```bash
git add -A
git commit -m "refactor: migrate TabState to PaneNode tree structure

Each tab now holds a pane tree instead of a flat terminal/session.
Single-pane behavior is preserved. Multi-pane support follows."
```

---

## Phase 3: Native Context Menus

### Task 5: Add muda dependency and create menu module

**Files:**
- Modify: `Cargo.toml` (add muda)
- Create: `src/gui/menus.rs`
- Modify: `src/gui/mod.rs` (add module)

**Step 1: Add muda to Cargo.toml**

Add to `[dependencies]` section (after the `arboard` line):

```toml
muda = "0.15"
```

On Linux, muda requires GTK. Add platform-specific dependency:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
gtk = "0.18"
```

**Step 2: Create `src/gui/menus.rs` with menu builders**

```rust
use muda::{ContextMenu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu, accelerator::{Accelerator, Code, Modifiers}};

/// Identifiers for context menu actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuAction {
    // Terminal context menu
    Copy,
    Paste,
    SelectAll,
    ClearSelection,
    SplitRight,
    SplitDown,
    SplitLeft,
    SplitUp,
    ClosePane,
    ClearTerminal,
    ResetTerminal,
    // Tab context menu
    RenameTab,
    DuplicateTab,
    CloseTab,
}

/// Builds the terminal area context menu.
/// `has_selection`: whether text is currently selected
/// `has_multiple_panes`: whether this tab has >1 pane
pub(super) fn build_terminal_context_menu(
    has_selection: bool,
    has_multiple_panes: bool,
) -> (ContextMenu, Vec<(muda::MenuId, MenuAction)>) {
    let menu = ContextMenu::new();
    let mut action_map = Vec::new();

    let copy_item = MenuItem::new("Copy", has_selection, None::<Accelerator>);
    action_map.push((copy_item.id().clone(), MenuAction::Copy));
    let paste_item = MenuItem::new("Paste", true, None::<Accelerator>);
    action_map.push((paste_item.id().clone(), MenuAction::Paste));
    let select_all = MenuItem::new("Select All", true, None::<Accelerator>);
    action_map.push((select_all.id().clone(), MenuAction::SelectAll));
    let clear_sel = MenuItem::new("Clear Selection", has_selection, None::<Accelerator>);
    action_map.push((clear_sel.id().clone(), MenuAction::ClearSelection));

    let _ = menu.append_items(&[
        &copy_item,
        &paste_item,
        &select_all,
        &clear_sel,
        &PredefinedMenuItem::separator(),
    ]);

    let split_right = MenuItem::new("Split Right", true, None::<Accelerator>);
    action_map.push((split_right.id().clone(), MenuAction::SplitRight));
    let split_down = MenuItem::new("Split Down", true, None::<Accelerator>);
    action_map.push((split_down.id().clone(), MenuAction::SplitDown));
    let split_left = MenuItem::new("Split Left", true, None::<Accelerator>);
    action_map.push((split_left.id().clone(), MenuAction::SplitLeft));
    let split_up = MenuItem::new("Split Up", true, None::<Accelerator>);
    action_map.push((split_up.id().clone(), MenuAction::SplitUp));

    let _ = menu.append_items(&[
        &split_right,
        &split_down,
        &split_left,
        &split_up,
        &PredefinedMenuItem::separator(),
    ]);

    if has_multiple_panes {
        let close_pane = MenuItem::new("Close Pane", true, None::<Accelerator>);
        action_map.push((close_pane.id().clone(), MenuAction::ClosePane));
        let _ = menu.append_items(&[&close_pane, &PredefinedMenuItem::separator()]);
    }

    let clear_term = MenuItem::new("Clear Terminal", true, None::<Accelerator>);
    action_map.push((clear_term.id().clone(), MenuAction::ClearTerminal));
    let reset_term = MenuItem::new("Reset Terminal", true, None::<Accelerator>);
    action_map.push((reset_term.id().clone(), MenuAction::ResetTerminal));

    let _ = menu.append_items(&[&clear_term, &reset_term]);

    (menu, action_map)
}

/// Builds the tab bar context menu.
pub(super) fn build_tab_context_menu() -> (ContextMenu, Vec<(muda::MenuId, MenuAction)>) {
    let menu = ContextMenu::new();
    let mut action_map = Vec::new();

    let rename = MenuItem::new("Rename", true, None::<Accelerator>);
    action_map.push((rename.id().clone(), MenuAction::RenameTab));
    let duplicate = MenuItem::new("Duplicate", true, None::<Accelerator>);
    action_map.push((duplicate.id().clone(), MenuAction::DuplicateTab));
    let close = MenuItem::new("Close", true, None::<Accelerator>);
    action_map.push((close.id().clone(), MenuAction::CloseTab));

    let _ = menu.append_items(&[
        &rename,
        &PredefinedMenuItem::separator(),
        &duplicate,
        &PredefinedMenuItem::separator(),
        &close,
    ]);

    (menu, action_map)
}

/// Shows a context menu natively for the given window.
pub(super) fn show_context_menu(
    window: &winit::window::Window,
    menu: &ContextMenu,
    position: Option<muda::dpi::Position>,
) {
    use winit::raw_window_handle::*;

    #[cfg(target_os = "windows")]
    {
        if let RawWindowHandle::Win32(handle) = window.window_handle().unwrap().as_raw() {
            unsafe { menu.show_context_menu_for_hwnd(handle.hwnd.get(), position) };
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let RawWindowHandle::AppKit(handle) = window.window_handle().unwrap().as_raw() {
            unsafe {
                menu.show_context_menu_for_nsview(handle.ns_view.as_ptr() as _, position)
            };
        }
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, muda needs GTK. This requires gtk::init() called at startup.
        // For now, Linux context menus are a TODO — will be addressed when
        // Linux GTK integration is added.
        let _ = (window, menu, position);
        eprintln!("Native context menus not yet supported on Linux");
    }
}
```

**Step 3: Add module declaration**

In `src/gui/mod.rs`, add `mod menus;` in the module list.

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add Cargo.toml src/gui/menus.rs src/gui/mod.rs
git commit -m "feat: add muda dependency and native context menu builders"
```

---

### Task 6: Integrate native context menus with right-click

**Files:**
- Modify: `src/gui/events/mouse/input.rs:36-84` (replace right-click handler)
- Modify: `src/gui/lifecycle/mod.rs:135-197` (add menu event polling)
- Modify: `src/gui/state.rs` (add menu state to FerrumWindow)

**Step 1: Add menu action state to FerrumWindow**

In `src/gui/state.rs`, add to FerrumWindow struct:

```rust
pub(super) pending_menu_action: Option<(crate::gui::menus::MenuAction, Option<usize>)>,
```

The `Option<usize>` is the tab index for tab context menu actions.

**Step 2: Replace on_right_mouse_input**

In `src/gui/events/mouse/input.rs`, replace the `on_right_mouse_input` method (lines 36-84) to show native menus instead of auto-paste:

```rust
fn on_right_mouse_input(&mut self, state: ElementState) {
    match state {
        ElementState::Pressed => {
            self.commit_rename();
            self.security_popup = None;
            let (mx, my) = self.mouse_pos;
            let tab_bar_height = self.backend.tab_bar_height_px();

            if my < tab_bar_height as f64 {
                // Right-click on tab bar
                if let TabBarHit::Tab(idx) | TabBarHit::CloseTab(idx) =
                    self.tab_bar_hit(mx, my)
                {
                    let (menu, action_map) = crate::gui::menus::build_tab_context_menu();
                    // Store action map for later lookup
                    self.pending_menu_context = Some(MenuContext::Tab {
                        tab_index: idx,
                        action_map,
                    });
                    let pos = muda::dpi::Position::Physical(
                        muda::dpi::PhysicalPosition::new(mx as i32, my as i32),
                    );
                    crate::gui::menus::show_context_menu(&self.window, &menu, Some(pos));
                }
                return;
            }

            if self.is_mouse_reporting() {
                let (row, col) = self.pixel_to_grid(mx, my);
                self.send_mouse_event(2, col, row, true);
                return;
            }

            // Right-click on terminal area — show terminal context menu
            let has_selection = self
                .active_tab_ref()
                .and_then(|t| t.focused_leaf())
                .and_then(|l| l.selection)
                .is_some();
            let has_multiple_panes = self
                .active_tab_ref()
                .is_some_and(|t| t.has_multiple_panes());

            let (menu, action_map) = crate::gui::menus::build_terminal_context_menu(
                has_selection,
                has_multiple_panes,
            );
            self.pending_menu_context = Some(MenuContext::Terminal { action_map });
            let pos = muda::dpi::Position::Physical(
                muda::dpi::PhysicalPosition::new(mx as i32, my as i32),
            );
            crate::gui::menus::show_context_menu(&self.window, &menu, Some(pos));
        }
        ElementState::Released => {
            if self.is_mouse_reporting() {
                let (row, col) = self.pixel_to_grid(self.mouse_pos.0, self.mouse_pos.1);
                self.send_mouse_event(2, col, row, false);
            }
        }
    }
}
```

**Step 3: Add MenuContext enum and field to state**

In `src/gui/state.rs`, add:

```rust
pub(super) enum MenuContext {
    Tab {
        tab_index: usize,
        action_map: Vec<(muda::MenuId, crate::gui::menus::MenuAction)>,
    },
    Terminal {
        action_map: Vec<(muda::MenuId, crate::gui::menus::MenuAction)>,
    },
}
```

Add to FerrumWindow: `pub(super) pending_menu_context: Option<MenuContext>,`

**Step 4: Add menu event polling in about_to_wait**

In `src/gui/lifecycle/mod.rs`, add after the `self.drain_pty_events(event_loop);` line (line 136):

```rust
self.drain_menu_events();
```

Add the method to App impl:

```rust
fn drain_menu_events(&mut self) {
    while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
        // Find the window with a pending menu context
        for win in self.windows.values_mut() {
            if let Some(ctx) = win.pending_menu_context.take() {
                match ctx {
                    MenuContext::Tab { tab_index, action_map } => {
                        if let Some((_, action)) = action_map.iter().find(|(id, _)| *id == event.id) {
                            win.handle_menu_action(*action, Some(tab_index),
                                &mut self.next_tab_id, &self.tx);
                        }
                    }
                    MenuContext::Terminal { action_map } => {
                        if let Some((_, action)) = action_map.iter().find(|(id, _)| *id == event.id) {
                            win.handle_menu_action(*action, None,
                                &mut self.next_tab_id, &self.tx);
                        }
                    }
                }
                win.window.request_redraw();
                break;
            }
        }
    }
}
```

**Step 5: Implement handle_menu_action**

Add to FerrumWindow (in a new file `src/gui/events/menu.rs` or inline):

```rust
pub(super) fn handle_menu_action(
    &mut self,
    action: crate::gui::menus::MenuAction,
    tab_index: Option<usize>,
    next_tab_id: &mut u64,
    tx: &mpsc::Sender<PtyEvent>,
) {
    use crate::gui::menus::MenuAction;
    match action {
        MenuAction::Copy => self.copy_selection(),
        MenuAction::Paste => self.paste_clipboard(),
        MenuAction::SelectAll => { /* TODO: implement select all */ }
        MenuAction::ClearSelection => {
            if let Some(tab) = self.active_tab_mut() {
                if let Some(leaf) = tab.focused_leaf_mut() {
                    leaf.selection = None;
                }
            }
        }
        MenuAction::SplitRight => self.split_pane(SplitDirection::Horizontal, next_tab_id, tx),
        MenuAction::SplitDown => self.split_pane(SplitDirection::Vertical, next_tab_id, tx),
        MenuAction::SplitLeft => self.split_pane(SplitDirection::Horizontal, next_tab_id, tx), // TODO: left variant
        MenuAction::SplitUp => self.split_pane(SplitDirection::Vertical, next_tab_id, tx), // TODO: up variant
        MenuAction::ClosePane => self.close_focused_pane(),
        MenuAction::ClearTerminal => {
            if let Some(tab) = self.active_tab_mut() {
                if let Some(leaf) = tab.focused_leaf_mut() {
                    leaf.terminal.clear_screen();
                }
            }
        }
        MenuAction::ResetTerminal => {
            if let Some(tab) = self.active_tab_mut() {
                if let Some(leaf) = tab.focused_leaf_mut() {
                    leaf.terminal.reset();
                }
            }
        }
        MenuAction::RenameTab => {
            if let Some(idx) = tab_index {
                self.start_rename(idx);
            }
        }
        MenuAction::DuplicateTab => {
            if let Some(idx) = tab_index {
                self.duplicate_tab(idx, next_tab_id, tx);
            }
        }
        MenuAction::CloseTab => {
            if let Some(idx) = tab_index {
                self.close_tab(idx);
            }
        }
    }
}
```

**Step 6: Verify compilation and test**

Run: `cargo build && cargo test -q`
Expected: Compiles and tests pass

**Step 7: Commit**

```bash
git add -A
git commit -m "feat: integrate native context menus on right-click via muda"
```

---

### Task 7: Remove old custom context menu code

**Files:**
- Modify: `src/gui/renderer/types.rs` (remove ContextMenu, ContextAction, ContextMenuTarget, related types)
- Modify: `src/gui/renderer/context_menu.rs` (remove or gut)
- Modify: `src/gui/renderer/traits.rs` (remove draw_context_menu, hit_test_context_menu)
- Modify: `src/gui/events/mouse/context_menu.rs` (remove)
- Modify: `src/gui/events/mouse/cursor.rs` (remove context_menu hover logic)
- Modify: `src/gui/events/redraw/animation.rs` (remove context_menu animation)
- Modify: `src/gui/events/render_shared.rs` (remove context_menu rendering)
- Modify: `src/gui/state.rs` (remove context_menu field from FerrumWindow)
- Modify: `src/gui/renderer/backend.rs` (remove hit_test_context_menu)
- Modify: `src/gui/renderer/gpu/hit_test.rs` (remove draw_context_menu_impl)

**Step 1: Remove all ContextMenu references systematically**

Follow the compiler after removing each piece:

1. Delete `context_menu: Option<ContextMenu>` from FerrumWindow (state.rs:130)
2. Remove ContextMenu, ContextAction, ContextMenuTarget, ContextMenuLayout from types.rs
3. Remove `draw_context_menu` from Renderer trait (traits.rs:237-249)
4. Remove `hit_test_context_menu` from backend.rs
5. Remove `handle_context_menu_left_click` from context_menu.rs
6. Remove context menu hover code from cursor.rs (lines 77-87)
7. Remove context menu animation from animation.rs
8. Remove context menu rendering from render_shared.rs (lines 329-331)
9. Remove `draw_context_menu` implementations from CPU and GPU renderers
10. Remove the `ContextMenu` import from `src/gui/mod.rs`

**Step 2: Fix all compilation errors**

Remove all references to `self.context_menu` throughout the codebase. These include:
- `self.context_menu = None;` in various places (remove these lines)
- `if self.context_menu.is_some()` checks (remove or adjust)
- FrameParams `context_menu` field (remove)

**Step 3: Verify compilation and tests**

Run: `cargo build && cargo test -q`
Expected: Everything compiles and passes

**Step 4: Manual test**

Run: `cargo run`
Expected: Right-click shows native OS menu. Old custom menus gone.

**Step 5: Commit**

```bash
git add -A
git commit -m "refactor: remove old custom context menu rendering code

Replaced by OS-native context menus via muda."
```

---

## Phase 4: Pane Rendering

### Task 8: Multi-pane rendering for CPU backend

**Files:**
- Modify: `src/gui/events/render_shared.rs` (update draw_frame_content for multi-pane)
- Modify: `src/gui/renderer/traits.rs` (add render_pane method or adjust render signature)

**Step 1: Update FrameParams to carry pane layout**

In `render_shared.rs`, update FrameParams:

```rust
pub(in crate::gui::events) struct FrameParams<'a> {
    pub tab: Option<&'a TabState>,
    pub cursor_blink_start: std::time::Instant,
    pub hovered_tab: Option<usize>,
    pub mouse_pos: (f64, f64),
    pub pinned: bool,
    pub security_popup: Option<&'a SecurityPopup>,
}
```

**Step 2: Update draw_frame_content to iterate panes**

Replace the terminal rendering section (lines 224-266) with a loop over pane layout:

```rust
// 1) Calculate pane layout and render each pane
if let Some(tab) = params.tab {
    let tab_bar_h = renderer.tab_bar_height_px();
    let padding = renderer.window_padding_px();
    let terminal_rect = PaneRect {
        x: padding,
        y: tab_bar_h + padding,
        width: bw as u32 - padding * 2,
        height: bh as u32 - tab_bar_h - padding * 2,
    };
    let divider_px = (DIVIDER_WIDTH as f64 * renderer.scale_factor()).ceil() as u32;
    let pane_layout = tab.pane_tree.layout(terminal_rect, divider_px);

    for &(pane_id, rect) in &pane_layout {
        if let Some(leaf) = tab.pane_tree.find_leaf(pane_id) {
            let viewport_start = leaf
                .terminal
                .scrollback
                .len()
                .saturating_sub(leaf.scroll_offset);

            // Render terminal grid into pane area
            renderer.render_in_rect(
                buffer, bw, bh,
                &leaf.terminal.grid,
                leaf.selection.as_ref(),
                viewport_start,
                rect,
            );

            // Draw cursor if this is the focused pane
            if pane_id == tab.focused_pane
                && leaf.scroll_offset == 0
                && leaf.terminal.cursor_visible
                && should_show_cursor(params.cursor_blink_start, leaf.terminal.cursor_style)
            {
                renderer.draw_cursor_in_rect(
                    buffer, bw, bh,
                    leaf.terminal.cursor_row,
                    leaf.terminal.cursor_col,
                    &leaf.terminal.grid,
                    leaf.terminal.cursor_style,
                    rect,
                );
            }

            // Draw scrollbar for this pane
            let scrollback_len = leaf.terminal.scrollback.len();
            if scrollback_len > 0 {
                let opacity = scrollbar_opacity(
                    leaf.scrollbar.hover,
                    leaf.scrollbar.dragging,
                    leaf.scrollbar.last_activity,
                );
                if opacity > 0.0 {
                    renderer.render_scrollbar_in_rect(
                        buffer, bw, bh,
                        leaf.scroll_offset,
                        scrollback_len,
                        leaf.terminal.grid.rows,
                        opacity,
                        leaf.scrollbar.hover || leaf.scrollbar.dragging,
                        rect,
                    );
                }
            }

            // Dim inactive panes
            if pane_id != tab.focused_pane {
                renderer.draw_dim_overlay(buffer, bw, bh, rect, 0.3);
            }
        }
    }

    // Draw dividers between panes
    let dividers = collect_dividers(&tab.pane_tree, terminal_rect, divider_px);
    for divider in &dividers {
        renderer.draw_divider(buffer, bw, bh, divider);
    }
}
```

**Step 3: Add new renderer trait methods**

In `src/gui/renderer/traits.rs`, add:

```rust
fn render_in_rect(
    &mut self, buffer: &mut [u32], buf_width: usize, buf_height: usize,
    grid: &Grid, selection: Option<&Selection>, viewport_start: usize,
    rect: PaneRect,
);

fn draw_cursor_in_rect(
    &mut self, buffer: &mut [u32], buf_width: usize, buf_height: usize,
    row: usize, col: usize, grid: &Grid, style: CursorStyle,
    rect: PaneRect,
);

fn render_scrollbar_in_rect(
    &mut self, buffer: &mut [u32], buf_width: usize, buf_height: usize,
    scroll_offset: usize, scrollback_len: usize, visible_rows: usize,
    opacity: f32, hover: bool,
    rect: PaneRect,
);

fn draw_dim_overlay(
    &mut self, buffer: &mut [u32], buf_width: usize, buf_height: usize,
    rect: PaneRect, alpha: f32,
);

fn draw_divider(
    &mut self, buffer: &mut [u32], buf_width: usize, buf_height: usize,
    divider: &DividerInfo,
);
```

**Step 4: Implement for CPU renderer**

The CPU renderer implementations clip their output to the PaneRect bounds. The key change is offsetting all pixel coordinates by `(rect.x, rect.y)` and clipping to `rect.width × rect.height`.

For `render_in_rect`: wrap existing `render()` with viewport clipping.
For `draw_dim_overlay`: fill rect with semi-transparent black.
For `draw_divider`: fill 1px line at divider position with divider color.

**Step 5: Implement for GPU renderer**

Similar approach but using viewport/scissor rect in the GPU pipeline.

**Step 6: Verify**

Run: `cargo build && cargo run`
Expected: Single-pane tabs render identically. No visual changes yet (until splitting is wired up).

**Step 7: Commit**

```bash
git add -A
git commit -m "feat: multi-pane rendering with per-pane clipping, dividers, and dimming"
```

---

## Phase 5: Pane Interaction

### Task 9: Implement split_pane and close_focused_pane

**Files:**
- Create: `src/gui/pane_ops.rs` (or add to existing pane.rs)
- Modify: `src/gui/mod.rs`

**Step 1: Implement split_pane on FerrumWindow**

```rust
impl FerrumWindow {
    pub(super) fn split_pane(
        &mut self,
        direction: SplitDirection,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) {
        let Some(tab) = self.active_tab_mut() else { return };
        let new_pane_id = tab.next_pane_id;
        tab.next_pane_id += 1;

        // Calculate size for the new pane (half of focused pane)
        let size = self.window.inner_size();
        let (rows, cols) = self.calc_grid_size(size.width, size.height);
        // After split, each pane gets roughly half the space
        let (new_rows, new_cols) = match direction {
            SplitDirection::Horizontal => (rows, cols / 2),
            SplitDirection::Vertical => (rows / 2, cols),
        };

        // Spawn new PTY session for the new pane
        let shell = crate::pty::default_shell();
        let session = match crate::pty::Session::spawn(&shell, new_rows as u16, new_cols as u16) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to spawn PTY for new pane: {e}");
                return;
            }
        };
        let pty_writer = match session.writer() {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to get PTY writer: {e}");
                return;
            }
        };

        // Spawn PTY reader thread
        let tab_id = tab.id;
        let pane_id = new_pane_id;
        let tx_clone = tx.clone();
        let mut reader = match session.reader() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to get PTY reader: {e}");
                return;
            }
        };
        std::thread::Builder::new()
            .name(format!("pty-reader-{}-{}", tab_id, pane_id))
            .spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => {
                            let _ = tx_clone.send(PtyEvent::Exited { tab_id, pane_id });
                            break;
                        }
                        Ok(n) => {
                            let _ = tx_clone.send(PtyEvent::Data {
                                tab_id,
                                pane_id,
                                bytes: buf[..n].to_vec(),
                            });
                        }
                        Err(_) => {
                            let _ = tx_clone.send(PtyEvent::Exited { tab_id, pane_id });
                            break;
                        }
                    }
                }
            })
            .ok();

        let mut terminal = Terminal::new(new_rows, new_cols);
        let leaf = PaneLeaf {
            id: new_pane_id,
            terminal,
            session,
            pty_writer,
            selection: None,
            scroll_offset: 0,
            security: SecurityGuard::new(),
            scrollbar: ScrollbarState::new(),
        };

        // Insert the new leaf into the tree by splitting the focused pane
        let focused = tab.focused_pane;
        // We need to handle this carefully since split() replaces in-place
        // but needs a real PaneLeaf, not a test stub
        tab.pane_tree.split_with_leaf(focused, direction, PaneNode::Leaf(leaf));
        tab.focused_pane = new_pane_id;

        // Resize all panes to fit the new layout
        self.resize_all_panes();
    }

    pub(super) fn close_focused_pane(&mut self) {
        let Some(tab) = self.active_tab_mut() else { return };
        if tab.pane_tree.is_leaf() {
            return; // Can't close the only pane
        }

        let closing_id = tab.focused_pane;
        tab.pane_tree.close(closing_id);

        // Move focus to first remaining pane
        if let Some(first_id) = tab.pane_tree.leaf_ids().first().copied() {
            tab.focused_pane = first_id;
        }

        self.resize_all_panes();
    }

    /// Recalculate and apply PTY resize for all panes based on current layout.
    fn resize_all_panes(&mut self) {
        let size = self.window.inner_size();
        let tab_bar_h = self.backend.tab_bar_height_px();
        let padding = self.backend.window_padding_px();
        let cw = self.backend.cell_width();
        let ch = self.backend.cell_height();

        let terminal_rect = PaneRect {
            x: padding,
            y: tab_bar_h + padding,
            width: size.width.saturating_sub(padding * 2),
            height: size.height.saturating_sub(tab_bar_h + padding * 2),
        };

        let divider_px = 1; // TODO: DPI scale
        let Some(tab) = self.active_tab_mut() else { return };
        let layout = tab.pane_tree.layout(terminal_rect, divider_px);

        for (pane_id, rect) in layout {
            if let Some(leaf) = tab.pane_tree.find_leaf_mut(pane_id) {
                let cols = (rect.width / cw).max(1) as usize;
                let rows = (rect.height / ch).max(1) as usize;
                leaf.terminal.resize(rows, cols);
                leaf.session.resize(rows as u16, cols as u16);
            }
        }
    }
}
```

Note: `split_with_leaf` is a new method on PaneNode that takes an already-constructed PaneNode (not just an ID). Add this to `src/gui/pane.rs`.

**Step 2: Verify**

Run: `cargo build && cargo run`
Test: Right-click → Split Right should create a new pane

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement pane splitting and closing operations"
```

---

### Task 10: Add keyboard shortcuts for pane operations

**Files:**
- Modify: `src/gui/events/keyboard/shortcuts.rs:55-134` (add to handle_ctrl_shift_shortcuts)

**Step 1: Add pane shortcuts to handle_ctrl_shift_shortcuts**

In `src/gui/events/keyboard/shortcuts.rs`, add before the existing `match key` block in `handle_ctrl_shift_shortcuts` (around line 108):

```rust
// Pane splitting
if Self::physical_key_is(physical, KeyCode::KeyR) {
    self.split_pane(SplitDirection::Horizontal, next_tab_id, tx);
    return true;
}
if Self::physical_key_is(physical, KeyCode::KeyD) {
    self.split_pane(SplitDirection::Vertical, next_tab_id, tx);
    return true;
}
if Self::physical_key_is(physical, KeyCode::KeyL) {
    // Split left = horizontal split, but new pane goes first
    self.split_pane_left(next_tab_id, tx);
    return true;
}
if Self::physical_key_is(physical, KeyCode::KeyU) {
    // Split up = vertical split, but new pane goes first
    self.split_pane_up(next_tab_id, tx);
    return true;
}
if Self::physical_key_is(physical, KeyCode::KeyW) {
    self.close_focused_pane();
    return true;
}

// Pane navigation (Ctrl+Shift+Arrow)
match key {
    Key::Named(NamedKey::ArrowUp) if !self.modifiers.super_key() => {
        self.navigate_pane(NavigateDirection::Up);
        return true;
    }
    Key::Named(NamedKey::ArrowDown) if !self.modifiers.super_key() => {
        self.navigate_pane(NavigateDirection::Down);
        return true;
    }
    Key::Named(NamedKey::ArrowLeft) if !self.modifiers.super_key() => {
        self.navigate_pane(NavigateDirection::Left);
        return true;
    }
    Key::Named(NamedKey::ArrowRight) if !self.modifiers.super_key() => {
        self.navigate_pane(NavigateDirection::Right);
        return true;
    }
    _ => {}
}
```

Note: The existing `Ctrl+Shift+ArrowLeft/Right` on macOS maps to Home/End. This needs to be handled — pane navigation takes precedence when there are multiple panes, otherwise fall through to Home/End.

**Step 2: Implement navigate_pane**

```rust
fn navigate_pane(&mut self, direction: NavigateDirection) {
    let size = self.window.inner_size();
    let tab_bar_h = self.backend.tab_bar_height_px();
    let padding = self.backend.window_padding_px();
    let terminal_rect = PaneRect {
        x: padding,
        y: tab_bar_h + padding,
        width: size.width.saturating_sub(padding * 2),
        height: size.height.saturating_sub(tab_bar_h + padding * 2),
    };

    let Some(tab) = self.active_tab_mut() else { return };
    let layout = tab.pane_tree.layout(terminal_rect, 1);
    if let Some(target) = PaneNode::navigate_spatial(&layout, tab.focused_pane, direction) {
        tab.focused_pane = target;
    }
}
```

**Step 3: Verify**

Run: `cargo build && cargo run`
Test: Ctrl+Shift+R splits right, Ctrl+Shift+D splits down, Ctrl+Shift+W closes pane, Ctrl+Shift+Arrows navigate

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: add keyboard shortcuts for pane split, close, and navigation"
```

---

### Task 11: Mouse-to-pane routing and focus switching

**Files:**
- Modify: `src/gui/events/mouse/input.rs` (route clicks to correct pane)
- Modify: `src/gui/events/mouse/cursor.rs` (divider hover cursor)

**Step 1: Update left-click to route to pane and set focus**

In the left-click handler, after tab bar and scrollbar handling, determine which pane was clicked and set it as focused:

```rust
// In terminal area click handling:
let Some(tab) = self.active_tab_mut() else { return };
let terminal_rect = /* calculate terminal rect */;
if let Some(pane_id) = tab.pane_tree.pane_at_pixel(
    mx as u32, my as u32, terminal_rect, 1,
) {
    if pane_id != tab.focused_pane {
        tab.focused_pane = pane_id;
        // Redraw to update dim overlay
    }
}
```

**Step 2: Add divider hover cursor**

In `cursor.rs` `on_cursor_moved`, add divider detection:

```rust
// Check if hovering over a divider
let terminal_rect = /* calculate */;
if let Some(tab) = self.active_tab_ref() {
    if let Some(hit) = tab.pane_tree.hit_test_divider(
        mx as u32, my as u32, terminal_rect, 1, DIVIDER_HIT_ZONE,
    ) {
        let cursor = match hit.direction {
            SplitDirection::Horizontal => CursorIcon::ColResize,
            SplitDirection::Vertical => CursorIcon::RowResize,
        };
        self.window.set_cursor(cursor);
        return;
    }
}
```

**Step 3: Implement divider drag resize**

Add state for tracking divider drag:

```rust
// In FerrumWindow:
pub(super) divider_drag: Option<DividerDragState>,

struct DividerDragState {
    start_pos: (u32, u32),  // initial mouse position on the divider
}
```

On mouse press on divider: start drag.
On cursor move during drag: call `resize_divider_at()` on the pane tree.
On mouse release: end drag, resize all PTYs.

**Step 4: Verify**

Run: `cargo run`
Test: Click to switch focus between panes. Hover over divider shows resize cursor. Drag divider resizes panes.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: mouse-to-pane routing, focus switching, and divider drag resize"
```

---

### Task 12: Final integration and cleanup

**Files:**
- Various files for edge case fixes

**Step 1: Handle window resize**

When the window is resized, recalculate layout for all panes in the active tab and send PTY resize:

```rust
// In on_resized handler, after existing logic:
self.resize_all_panes();
```

**Step 2: Handle tab switching with panes**

When switching tabs, each tab remembers its own pane_tree and focused_pane. No extra work needed since TabState already stores this.

**Step 3: Handle keyboard input routing**

Key presses should go to the focused pane's PTY writer, not the tab's (which no longer exists):

```rust
// In forward_key_to_pty:
fn write_pty_bytes(&mut self, bytes: &[u8]) {
    if let Some(tab) = self.active_tab_mut() {
        if let Some(leaf) = tab.focused_leaf_mut() {
            let _ = leaf.pty_writer.write_all(bytes);
            let _ = leaf.pty_writer.flush();
        }
    }
}
```

**Step 4: Run full test suite**

Run: `cargo test -q`
Expected: All tests pass

**Step 5: Manual integration test**

Run: `cargo run`
Test checklist:
- [ ] Right-click shows native context menu
- [ ] Context menu Copy/Paste/Select All work
- [ ] Split Right creates a new pane to the right
- [ ] Split Down creates a new pane below
- [ ] Split Left creates a new pane to the left
- [ ] Split Up creates a new pane above
- [ ] Close Pane closes the focused pane
- [ ] Ctrl+Shift+R/D/L/U shortcuts work
- [ ] Ctrl+Shift+W closes focused pane
- [ ] Ctrl+Shift+Arrows navigate between panes
- [ ] Click on a pane focuses it
- [ ] Inactive panes are dimmed
- [ ] Dividers visible between panes
- [ ] Drag divider resizes panes
- [ ] Typing goes to focused pane only
- [ ] PTY output appears in correct pane
- [ ] Tab switching preserves pane layout
- [ ] Window resize adjusts all panes
- [ ] Closing last pane = closing tab
- [ ] Multiple tabs with different pane layouts work independently

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: complete pane splitting and native context menu integration

- OS-native context menus via muda (macOS/Windows)
- Binary pane tree per tab (Ghostty-style)
- Split right/down/left/up with keyboard shortcuts
- Mouse pane focus, divider drag resize
- Inactive pane dimming overlay
- PTY routing per pane"
```

---

## Notes for Implementation

### Shortcut conflicts
- `Ctrl+Shift+W` currently doesn't exist (Ctrl+W closes tab). The new shortcut closes the focused pane; if it's the only pane, it should close the tab (existing behavior).
- `Ctrl+Shift+ArrowLeft/Right` on macOS currently maps to Home/End. With multiple panes, pane navigation should take priority; with single pane, fall through to Home/End.

### Linux context menus
Linux support via muda requires GTK initialization. This is deferred — on Linux, right-click will either show no menu (with a log message) or fall back to the old behavior temporarily.

### Test-only PaneNode construction
The `#[cfg(test)] new_leaf()` method uses `std::mem::zeroed()` for Session which is unsafe. For real tests that need PTY sessions, use integration tests or mock the Session type.

### Performance
Layout calculation is O(n) where n = number of panes. With typical usage (< 10 panes), this is negligible. Layout is recalculated on every frame but is very cheap.

### Rendering approach
The initial implementation uses "render then clip" — each pane is rendered into the full buffer but clipped to its rect. A more efficient approach (render directly into subrect) can be optimized later if needed.
