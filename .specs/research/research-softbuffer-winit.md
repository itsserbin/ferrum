---
title: Research - softbuffer 0.4 with winit 0.30
task_file: User request for softbuffer integration
scratchpad: /home/user/apps/ferrum/.specs/scratchpad/a7f3d9e2.md
created: 2026-02-15
status: complete
---

# Research: softbuffer 0.4 with winit 0.30

## Executive Summary

softbuffer 0.4 provides CPU-based pixel buffer rendering for windows created with winit 0.30. The integration uses `Context::new()` with the event loop's display handle, `Surface::new()` for per-window surfaces, `buffer_mut()` to get mutable pixel buffers (as `[u32]` slices), and pixel format is 0RGB (24-bit color with unused high byte). The ApplicationHandler pattern requires storing the window in `Option<Arc<Window>>` and surface in `Option<Surface>`.

## Related Existing Research

None found in `.specs/research/`

---

## Documentation & References

| Resource | Description | Relevance | Link |
|----------|-------------|-----------|------|
| softbuffer 0.4 API docs | Official API reference | Primary source for method signatures | https://docs.rs/softbuffer/0.4/softbuffer/ |
| winit 0.30 ApplicationHandler | Trait definition and patterns | Required for event loop integration | https://docs.rs/winit/0.30/winit/application/trait.ApplicationHandler.html |
| softbuffer GitHub examples | Working code examples | Reference implementations | https://github.com/rust-windowing/softbuffer/blob/master/examples/winit.rs |
| softbuffer changelog | API version differences | Clarifies 0.4 vs unreleased API | https://github.com/rust-windowing/softbuffer/blob/master/CHANGELOG.md |

### Key Concepts

- **Context**: Platform-specific state required for managing window rendering surfaces
- **Surface**: Drawing surface tied to a specific window, provides access to pixel buffers
- **Buffer**: Mutable slice of u32 pixels that can be written and presented to window
- **0RGB Format**: 32-bit pixel format where bits 0-7 are blue, 8-15 green, 16-23 red, 24-31 unused
- **ApplicationHandler**: winit 0.30 trait-based event handling pattern replacing closure-based API

---

## Libraries & Tools

| Name | Purpose | Maturity | Notes |
|------|---------|----------|-------|
| softbuffer 0.4 | CPU pixel buffer rendering | Stable | Use 0.4.x for buffer_mut() API |
| winit 0.30 | Cross-platform windowing | Stable | Requires ApplicationHandler pattern |
| raw-window-handle 0.6 | Platform window handle abstraction | Stable | Implicit dependency via traits |

### Recommended Stack

**softbuffer 0.4** with **winit 0.30** provides the simplest CPU-based rendering. The 0.4 API uses `buffer_mut()` and raw `u32` values, which is simpler than the unreleased API that uses `next_buffer()` and `Pixel` structs.

---

## Patterns & Approaches

### Pattern: ApplicationHandler with Option<Arc<Window>>

**When to use**: Required for all winit 0.30 applications due to trait-based event loop API

**Trade-offs**:
- Pros: Type-safe, explicit lifecycle management, supports async initialization
- Cons: More boilerplate than old closure API, requires Option wrapping

**Example structure**:
```rust
struct App {
    context: Option<Context<OwnedDisplayHandle>>,
    window: Option<Arc<Window>>,
    surface: Option<Surface<OwnedDisplayHandle, Arc<Window>>>,
}
```

**Lifecycle**:
1. Initialize App with None values
2. In `resumed()`: Create context, window, and surface
3. In `window_event()`: Handle RedrawRequested and Resized events
4. Access window/surface with `.as_ref().unwrap()` or `.as_mut().unwrap()`

---

## API Details

### Context Creation

**Signature**:
```rust
pub fn new<D: HasDisplayHandle>(display: D) -> Result<Self, SoftBufferError>
```

**Usage**:
```rust
let context = Context::new(event_loop.owned_display_handle()).unwrap();
```

**Type**: `Context<OwnedDisplayHandle>` where `OwnedDisplayHandle` is returned by `event_loop.owned_display_handle()`

---

### Surface Creation

**Signature**:
```rust
pub fn new(context: &Context<D>, window: W) -> Result<Self, SoftBufferError>
where
    D: HasDisplayHandle,
    W: HasWindowHandle
```

**Usage**:
```rust
let surface = Surface::new(&context, window.clone()).unwrap();
```

**Type**: `Surface<OwnedDisplayHandle, Arc<Window>>`

**Important**: Window must be `Arc<Window>` because softbuffer needs to clone the handle for internal use.

---

### Getting Mutable Buffer

**Signature**:
```rust
pub fn buffer_mut(&mut self) -> Result<Buffer<'_, D, W>, SoftBufferError>
where
    D: HasDisplayHandle,
    W: HasWindowHandle
```

**Usage**:
```rust
let mut buffer = surface.buffer_mut().unwrap();
```

**Type**: `Buffer<'_, OwnedDisplayHandle, Arc<Window>>` which derefs to `[u32]`

---

### Writing Pixels

**Buffer implements `Deref<Target=[u32]>` and `DerefMut`, so you can**:

1. Direct indexing:
```rust
buffer[index] = pixel_value;
```

2. Iteration:
```rust
for pixel in buffer.iter_mut() {
    *pixel = color;
}
```

3. Fill entire buffer:
```rust
buffer.fill(color);
```

---

### Pixel Format (0RGB)

**Format**: 32-bit little-endian 0RGB

```
Bit layout:
MSB                           LSB
[ Unused ][  Red  ][ Green ][ Blue ]
  24-31     16-23    8-15     0-7
```

**Encoding**:
```rust
let red: u8 = 255;
let green: u8 = 128;
let blue: u8 = 64;
let pixel: u32 = ((red as u32) << 16) | ((green as u32) << 8) | (blue as u32);
```

**Example colors**:
- Red: `0x00FF0000` or `0xFF_00_00`
- Green: `0x0000FF00` or `0x00_FF_00`
- Blue: `0x000000FF` or `0x00_00_FF`
- White: `0x00FFFFFF` or `0xFF_FF_FF`
- Black: `0x00000000` or `0x00_00_00`

**No alpha channel**: The high byte is unused (always 0).

---

### Presenting Buffer

**Signature**:
```rust
pub fn present(self) -> Result<(), SoftBufferError>
```

**Usage**:
```rust
buffer.present().unwrap();
```

**Note**: `present()` consumes the buffer (takes `self`), so you must call it at the end of your rendering.

---

### Handling Resize

**Signature**:
```rust
pub fn resize(
    &mut self,
    width: NonZeroU32,
    height: NonZeroU32
) -> Result<(), SoftBufferError>
```

**Usage**:
```rust
let size = window.inner_size();
if let (Some(width), Some(height)) =
    (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
    surface.resize(width, height).unwrap();
}
```

**When to call**:
- On `WindowEvent::Resized` event
- Before calling `buffer_mut()` if window size might have changed
- Best practice: Call before each frame to ensure buffer matches window

---

## Complete Minimal Example

### Cargo.toml

```toml
[package]
name = "softbuffer-example"
version = "0.1.0"
edition = "2021"

[dependencies]
softbuffer = "0.4"
winit = "0.30"
```

### main.rs - Solid Color Fill

```rust
use std::num::NonZeroU32;
use std::sync::Arc;

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

struct App {
    context: Option<Context<winit::event_loop::OwnedDisplayHandle>>,
    window: Option<Arc<Window>>,
    surface: Option<Surface<winit::event_loop::OwnedDisplayHandle, Arc<Window>>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            context: None,
            window: None,
            surface: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            // Create context from event loop display handle
            let context = Context::new(event_loop.owned_display_handle()).unwrap();

            // Create window
            let window = Arc::new(
                event_loop
                    .create_window(Window::default_attributes())
                    .unwrap(),
            );

            // Create surface from context + window
            let surface = Surface::new(&context, window.clone()).unwrap();

            self.context = Some(context);
            self.window = Some(window.clone());
            self.surface = Some(surface);

            // Request initial redraw
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                let window = self.window.as_ref().unwrap();
                let surface = self.surface.as_mut().unwrap();

                let size = window.inner_size();

                // Resize surface to match window
                if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                {
                    surface.resize(width, height).unwrap();

                    // Get mutable buffer
                    let mut buffer = surface.buffer_mut().unwrap();

                    // Fill with solid red color
                    let red = 255u32;
                    let green = 0u32;
                    let blue = 0u32;
                    let color = (red << 16) | (green << 8) | blue;

                    for pixel in buffer.iter_mut() {
                        *pixel = color;
                    }

                    // Present buffer to window
                    buffer.present().unwrap();
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                // Window was resized, request redraw
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
```

---

## Exact Imports Needed

```rust
use std::num::NonZeroU32;          // For resize width/height
use std::sync::Arc;                // For Arc<Window>

use softbuffer::{Context, Surface}; // Core softbuffer types
use winit::application::ApplicationHandler; // Event handling trait
use winit::event::WindowEvent;     // Window event enum
use winit::event_loop::{ActiveEventLoop, EventLoop}; // Event loop types
use winit::window::{Window, WindowId}; // Window types
```

---

## Potential Issues

| Issue | Impact | Mitigation |
|-------|--------|------------|
| Buffer size mismatch | Panic or visual artifacts | Always call `resize()` before `buffer_mut()` |
| Zero-sized window | `NonZeroU32::new()` returns None | Check for None before resizing/rendering |
| Missing Arc<Window> | Compilation error | Always wrap Window in Arc before passing to Surface |
| Wrong pixel format | Wrong colors displayed | Use `(R << 16) \| (G << 8) \| B` formula consistently |
| Presenting without writes | Undefined buffer contents | Always write pixels before present() |
| Not requesting redraw | Window never updates | Call `window.request_redraw()` after changes |

---

## Advanced: Pattern Fill Example

Replace the solid color loop with:

```rust
// Get buffer dimensions
let width = buffer.width().get() as usize;
let height = buffer.height().get() as usize;

// Fill with gradient pattern
for y in 0..height {
    for x in 0..width {
        let index = y * width + x;

        let red = (x * 255 / width) as u32;
        let green = (y * 255 / height) as u32;
        let blue = 128u32;

        buffer[index] = (red << 16) | (green << 8) | blue;
    }
}
```

---

## Advanced: Double Buffering Note

softbuffer handles double buffering internally via the platform backend. You don't need to manage multiple buffers yourself. The `buffer_mut()` call gives you whichever buffer is ready to be written, and `present()` swaps it to the display.

---

## Sources

Primary sources consulted:

- [softbuffer 0.4 API Documentation](https://docs.rs/softbuffer/0.4/softbuffer/)
- [softbuffer Context Struct](https://docs.rs/softbuffer/0.4/softbuffer/struct.Context.html)
- [softbuffer Surface Struct](https://docs.rs/softbuffer/0.4/softbuffer/struct.Surface.html)
- [softbuffer Buffer Struct](https://docs.rs/softbuffer/0.4/softbuffer/struct.Buffer.html)
- [winit 0.30 ApplicationHandler](https://docs.rs/winit/0.30/winit/application/trait.ApplicationHandler.html)
- [winit 0.30 EventLoop](https://docs.rs/winit/0.30/winit/event_loop/struct.EventLoop.html)
- [softbuffer GitHub Repository](https://github.com/rust-windowing/softbuffer)
- [softbuffer CHANGELOG](https://github.com/rust-windowing/softbuffer/blob/master/CHANGELOG.md)
- [softbuffer winit.rs Example](https://github.com/rust-windowing/softbuffer/blob/master/examples/winit.rs)
- [winit 0.30 Discussion #3667](https://github.com/rust-windowing/winit/discussions/3667)

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| Source verification | ✅ | All from official docs.rs and GitHub repos |
| Recency check | ✅ | softbuffer 0.4 stable, winit 0.30 stable (latest) |
| Alternatives explored | ✅ | Confirmed 0.4 API vs unreleased API differences |
| Actionability | ✅ | Complete copy-paste example provided |
| Evidence quality | ✅ | Direct API documentation and official examples |

**Limitations/Caveats**:
- API will change in future softbuffer release (`buffer_mut()` → `next_buffer()`, raw u32 → `Pixel` struct)
- Current research is specifically for softbuffer 0.4.x, not unreleased master branch
- Example is minimal; production code should add error handling beyond `.unwrap()`
