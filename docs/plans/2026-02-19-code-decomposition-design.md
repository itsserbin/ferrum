# Code Decomposition Design

Split large files and functions into focused, single-responsibility modules. Create new types for better encapsulation.

## Phase 1: Eliminate Tab Code Duplication

### Problem

~165 lines of identical business logic duplicated between CPU and GPU renderers:
- `hit_test_tab_bar` (~50 lines copy-paste)
- `hit_test_tab_hover` (~20 lines)
- `hit_test_tab_security_badge` (~20 lines)
- `window_button_at_position` (~25 lines)
- `tab_hover_tooltip` (~15 lines)
- `point_in_rect` (duplicated in two places)

### Solution

Create `src/gui/renderer/shared/tab_hit_test.rs` — single source of truth for hit-testing logic. Both CPU and GPU become thin delegates.

#### New file: `shared/tab_hit_test.rs` (~130 lines)

Pure functions that take layout metrics and return results:

```rust
pub fn hit_test_tab_bar(
    x: f64, y: f64, tab_count: usize, buf_width: u32,
    metrics: &TabLayoutMetrics, tabs: &[TabState],
) -> TabBarHit

pub fn hit_test_tab_hover(
    x: f64, y: f64, tab_count: usize, buf_width: u32,
    metrics: &TabLayoutMetrics,
) -> Option<usize>

pub fn hit_test_tab_security_badge(
    x: f64, y: f64, tabs: &[TabState], buf_width: u32,
    metrics: &TabLayoutMetrics,
) -> Option<usize>

pub fn window_button_at_position(
    x: f64, y: f64, buf_width: u32, metrics: &TabLayoutMetrics,
) -> Option<WindowButton>

pub fn tab_hover_tooltip<'a>(
    tabs: &'a [TabState], hovered: Option<usize>, buf_width: u32,
    metrics: &TabLayoutMetrics,
) -> Option<&'a str>

pub fn point_in_rect(x: f64, y: f64, rect: (u32, u32, u32, u32)) -> bool
```

#### Changes to existing files

- **`tab_bar/layout.rs`**: Remove duplicated hit-test implementations, replace with one-line delegates to `shared/tab_hit_test`
- **`gpu/hit_test.rs`**: Remove duplicated hit-test implementations, keep only context menu + security popup rendering (GPU-specific)
- **`gpu/tab_layout.rs`**: Remove `point_in_rect`, use shared version
- Move `point_in_rect` from `tab_bar/primitives.rs` to `shared/tab_math.rs`

#### Target structure

```
gui/renderer/
├── shared/
│   ├── tab_math.rs          (layout calculations — unchanged)
│   ├── tab_hit_test.rs      (NEW — shared hit-testing)
│   └── mod.rs               (updated exports)
├── tab_bar/
│   ├── mod.rs               (~400 lines — CPU rendering only)
│   ├── layout.rs            (~150 lines — thin delegates)
│   ├── primitives.rs        (unchanged)
│   └── buttons.rs           (unchanged)
└── gpu/
    ├── tab_layout.rs        (~350 lines — GPU rendering only)
    └── hit_test.rs          (~180 lines — context menu + popup only)
```

---

## Phase 2: Decompose GPU Renderer

### Problem

`gpu/mod.rs` (545 lines) and `gpu/frame.rs` (463 lines) contain 5+ distinct responsibilities each.

### Solution — Split mod.rs

#### New file: `gpu/scrollbar.rs` (~35 lines)

```rust
pub struct ScrollbarRenderer;

impl ScrollbarRenderer {
    pub fn render(
        renderer: &GpuRenderer,
        scroll_offset: usize, total_lines: usize,
        visible_rows: usize, opacity: f32, hovered: bool,
    )

    pub fn thumb_bounds(
        renderer: &GpuRenderer,
        scroll_offset: usize, total_lines: usize,
        visible_rows: usize,
    ) -> Option<(f32, f32, f32, f32)>
}
```

#### New file: `gpu/cursors.rs` (~55 lines)

```rust
pub struct CursorRenderer;

impl CursorRenderer {
    pub fn draw(
        renderer: &GpuRenderer,
        cursor_row: usize, cursor_col: usize,
        style: CursorStyle, grid: &Grid,
    )
}
```

#### New file: `gpu/grid_packing.rs` (~85 lines)

```rust
pub struct GridPacker;

impl GridPacker {
    pub fn pack(
        renderer: &mut GpuRenderer,
        grid: &Grid, selection: &Option<Selection>,
    )
}
```

### Solution — Split frame.rs

#### New file: `gpu/window_buttons.rs` (~100 lines)

Windows-only window chrome rendering (`#[cfg(not(target_os = "macos"))]`):
- `draw_close_button_commands()`
- `draw_window_buttons_commands()`
- `push_minimize_icon()`, `push_maximize_icon()`, `push_close_icon()`

#### New file: `gpu/gpu_passes.rs` (~140 lines)

Three-pass GPU encoding:
- `encode_grid_pass()` — compute shader, terminal grid to intermediate texture
- `encode_ui_pass()` — fragment shader, UI draw commands
- `encode_composite_pass()` — blend grid + UI into swapchain surface

#### Move `mix_rgb()` to `shared/color.rs`

Pure color interpolation utility, usable by both CPU and GPU.

#### Target structure

```
gui/renderer/gpu/
├── mod.rs              (~150 lines — struct + trait impl delegates)
├── setup.rs            (unchanged)
├── frame.rs            (~200 lines — upload, surface, resize, present_frame)
├── grid_packing.rs     (NEW)
├── cursors.rs          (NEW)
├── scrollbar.rs        (NEW)
├── gpu_passes.rs       (NEW)
├── window_buttons.rs   (NEW, cfg-gated)
├── tab_layout.rs       (~350 lines — GPU tab drawing)
├── hit_test.rs         (~180 lines — context menu + popup)
└── ui_commands.rs      (unchanged)
```

---

## Phase 3: Decompose Grid Operations

### Problem

`core/terminal/grid_ops.rs` (642 lines) contains 5 distinct responsibilities: reflow, resize, scrolling, alt-screen, display construction.

### Solution

#### New file: `core/terminal/reflow.rs` (~200 lines)

```rust
/// Logical line = one or more soft-wrapped rows merged together.
pub(super) struct LogicalLine {
    pub cells: Vec<Cell>,
    pub hard_wrap: bool,
    pub min_len: usize,
}

/// Pure reflow engine. All methods take data in, return data out.
pub(super) struct ReflowEngine;

impl ReflowEngine {
    /// Re-wrap logical lines to fit new column width. Pure function.
    pub fn rewrap_lines(lines: &[LogicalLine], new_cols: usize) -> Vec<Row>

    /// Merge soft-wrapped physical rows into logical lines.
    pub fn collect_logical_lines(
        scrollback: &VecDeque<Row>, grid: &Grid, cursor_row: usize,
    ) -> (Vec<LogicalLine>, Option<(usize, usize)>)

    /// Count meaningful content rows in grid.
    pub fn compute_content_rows(
        grid: &Grid, cursor_row: usize, scrollback_len: usize,
    ) -> usize
}
```

#### New file: `core/terminal/resize.rs` (~120 lines)

```rust
impl Terminal {
    /// Public entry point. Delegates to simple or reflow strategy.
    pub fn resize(&mut self, rows: usize, cols: usize)

    /// No-reflow resize for alt-screen or height-only changes.
    fn simple_resize(&mut self, rows: usize, cols: usize)

    /// Reflow resize: collect logical lines, rewrap, fill grid.
    fn reflow_resize(&mut self, new_rows: usize, new_cols: usize)

    /// Populate grid and scrollback from rewrapped rows.
    fn fill_grid_from_rewrapped(&mut self, rewrapped: &[Row], new_rows: usize, new_cols: usize)
}
```

#### New file: `core/terminal/alt_screen.rs` (~80 lines)

```rust
/// Encapsulates main/alt screen switching for full-screen apps.
pub(super) struct ScreenBuffer {
    saved_grid: Option<Grid>,
    saved_cursor: (usize, usize),
    saved_scroll_top: usize,
    saved_scroll_bottom: usize,
}

impl ScreenBuffer {
    pub fn new() -> Self
    pub fn enter(&mut self, terminal: &mut TerminalState)
    pub fn leave(&mut self, terminal: &mut TerminalState)
    pub fn is_active(&self) -> bool
}
```

#### Remaining in `grid_ops.rs` (~80 lines)

Thin delegation + scrolling:

```rust
impl Terminal {
    pub fn build_display(&self, scroll_offset: usize) -> Grid
    pub fn scroll_up_region(&mut self, top: usize, bottom: usize)
    pub fn scroll_down_region(&mut self, top: usize, bottom: usize)
    pub fn is_alt_screen(&self) -> bool
}
```

#### Target structure

```
core/terminal/
├── mod.rs           (unchanged)
├── grid_ops.rs      (~80 lines — display + scrolling)
├── reflow.rs        (NEW — ~200 lines)
├── resize.rs        (NEW — ~120 lines)
├── alt_screen.rs    (NEW — ~80 lines)
└── handlers/        (unchanged)
```

---

## Execution Order

1. **Phase 1** first — eliminates duplication, reduces maintenance risk
2. **Phase 2** second — GPU decomposition, larger scope but isolated
3. **Phase 3** last — core terminal logic, needs careful testing

Each phase = separate commit. Run `cargo test` after each phase.

## Risk Assessment

- **Phase 1** (Low risk): Moving pure functions, no logic changes
- **Phase 2** (Medium risk): GPU code harder to test, but changes are structural
- **Phase 3** (Medium risk): Core terminal logic; existing tests provide safety net

## Success Criteria

- All existing tests pass after each phase
- No file exceeds ~400 lines
- Zero duplicated business logic between CPU and GPU
- Each module has a single clear responsibility
- `cargo build` succeeds for both GPU and CPU-only builds
