# Context Menus & Pane Splitting Design

## Summary

Two changes to Ferrum:
1. Replace auto-paste on right-click with OS-native context menus (`muda` crate)
2. Add terminal pane splitting with a recursive binary tree (Ghostty-style)

## 1. Native Context Menus

### Dependency

Crate `muda` — cross-platform native menus from the Tauri team. Uses NSMenu on macOS, Win32 TrackPopupMenu on Windows, GTK on Linux.

### Menu Types

**Tab context menu** (right-click on tab bar):
- Rename Tab
- Duplicate Tab
- ---
- Close Tab

**Terminal context menu** (right-click on terminal area):
- Copy (Ctrl+Shift+C / Cmd+C)
- Paste (Ctrl+Shift+V / Cmd+V)
- Select All
- Clear Selection
- ---
- Split Right (Ctrl+Shift+R)
- Split Down (Ctrl+Shift+D)
- Split Left (Ctrl+Shift+L)
- Split Up (Ctrl+Shift+U)
- ---
- Close Pane (Ctrl+Shift+W) — only when >1 pane
- ---
- Clear Terminal
- Reset Terminal

### Context-Sensitive Items

- "Copy" enabled only when there's a selection
- "Close Pane" visible only when there are multiple panes
- "Clear Selection" visible only when there's a selection

### Code Changes

1. **Remove**: auto-paste in `on_right_mouse_input()` for non-selection area
2. **Remove**: custom `ContextMenu` struct, custom rendering (`render_context_menu`), hover animations
3. **Add**: `muda::ContextMenu` creation at window init, shown via `menu.show_context_menu_for_window()` on right-click
4. **Add**: Menu event handling in the event loop

## 2. Pane Tree Architecture

### Data Structures

```rust
type PaneId = u64;

enum PaneNode {
    Leaf(PaneLeaf),
    Split(PaneSplit),
}

struct PaneLeaf {
    id: PaneId,
    terminal: Terminal,
    session: pty::Session,
    pty_writer: Box<dyn Write + Send>,
    selection: Option<Selection>,
    scroll_offset: usize,
    security: SecurityGuard,
    scrollbar: ScrollbarState,
}

struct PaneSplit {
    direction: SplitDirection,
    ratio: f32,                 // 0.0..1.0, initially 0.5
    first: Box<PaneNode>,
    second: Box<PaneNode>,
}

enum SplitDirection {
    Horizontal,  // splits left-right
    Vertical,    // splits top-bottom
}
```

### TabState Changes

```rust
// BEFORE:
struct TabState {
    id: u64,
    terminal: Terminal,
    session: pty::Session,
    pty_writer: ...,
    title: String,
    scroll_offset: usize,
    selection: Option<Selection>,
    security: SecurityGuard,
    scrollbar: ScrollbarState,
}

// AFTER:
struct TabState {
    id: u64,
    title: String,
    pane_tree: PaneNode,
    focused_pane: PaneId,
    next_pane_id: PaneId,
}
```

### Tree Operations

- **`split(pane_id, direction)`** — find leaf by ID, replace with Split node containing original + new leaf
- **`close(pane_id)`** — find leaf, replace parent Split with sibling
- **`find_pane(pane_id)`** — recursive lookup returning &PaneLeaf
- **`find_pane_mut(pane_id)`** — mutable recursive lookup
- **`navigate(from, direction)`** — spatial navigation by physical position
- **`resize(divider, delta)`** — adjust ratio in nearest parent split

### PTY Routing

Each pane leaf has its own PTY reader thread (existing pattern). `PtyEvent::Data` gains a `pane_id` field to route data to the correct terminal.

## 3. Rendering

### Layout Calculation

Recursive traversal produces `Vec<(PaneId, PaneRect)>`:

```rust
struct PaneRect { x: u32, y: u32, width: u32, height: u32 }

fn layout_pane(node: &PaneNode, rect: PaneRect) -> Vec<(PaneId, PaneRect)>
```

### Per-Pane Rendering

Both CPU and GPU renderers iterate layout results, rendering each pane's terminal grid into its assigned rectangle area (clipped to bounds).

### Dividers

- Width: 1px (DPI-scaled)
- Color: `#585B70` (Catppuccin Mocha Surface2)
- Hover/drag color: `#89B4FA` (Catppuccin Blue)
- Hit-area: 4-6px for comfortable mouse targeting

### Inactive Pane Dimming

- Overlay: `rgba(0, 0, 0, 0.3)` on inactive panes
- Active pane: no overlay
- Transition: 150ms ease animation

### Terminal Resize Per Pane

On window resize or ratio change:
1. Recalculate layout
2. For each pane: `(cols, rows) = (rect.width / cell_width, rect.height / cell_height)`
3. Send PTY resize: `session.resize(rows, cols)`
4. Update terminal grid: `terminal.resize(rows, cols)`

## 4. Event Handling

### Keyboard Shortcuts

| Action | Linux/Windows | macOS |
|--------|--------------|-------|
| Split Right | Ctrl+Shift+R | Cmd+Shift+R |
| Split Down | Ctrl+Shift+D | Cmd+Shift+D |
| Split Left | Ctrl+Shift+L | Cmd+Shift+L |
| Split Up | Ctrl+Shift+U | Cmd+Shift+U |
| Close Pane | Ctrl+Shift+W | Cmd+Shift+W |
| Navigate Up | Ctrl+Shift+↑ | Cmd+Shift+↑ |
| Navigate Down | Ctrl+Shift+↓ | Cmd+Shift+↓ |
| Navigate Left | Ctrl+Shift+← | Cmd+Shift+← |
| Navigate Right | Ctrl+Shift+→ | Cmd+Shift+→ |

### Mouse Event Routing

1. `pixel_to_pane(x, y)` — find pane under cursor using layout data
2. If click on divider hit-area → start drag resize
3. If click on pane → set `focused_pane`, forward event to that pane

### Mouse Divider Resize

1. Cursor on divider → change cursor to `ResizeColumn`/`ResizeRow`
2. Mouse press on divider → record which split is being dragged
3. Cursor move during drag → update `ratio` in the `PaneSplit` node
4. Mouse release → finish drag, recalculate layout, resize all affected PTYs

### Right-Click Flow

1. Right-click → determine which pane was clicked
2. Set that pane as focused
3. Show native context menu via `muda`
4. Handle selected action

## 5. Scope (v1)

### Included
- Native context menus (replace custom + remove auto-paste)
- Binary pane tree with split/close/navigate
- Mouse resize of dividers
- Keyboard shortcuts for all pane operations
- Thin dividers + inactive pane dimming

### Excluded (future)
- Pane zoom (fullscreen single pane)
- Pane transfer between tabs
- Undo/redo of splits
- Equalize pane sizes
- Predefined layouts
