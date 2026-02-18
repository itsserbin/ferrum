---
title: Research - winit 0.30 API for ApplicationHandler
task_file: User request - winit 0.30 API research
scratchpad: /home/user/apps/ferrum/.specs/scratchpad/9a5ad0fc.md
created: 2026-02-15
status: complete
---

# Research: winit 0.30 ApplicationHandler API

## Executive Summary

winit 0.30 introduces a breaking change from closure-based event loops to a trait-based `ApplicationHandler` pattern. Windows must now be created inside the running event loop (in the `resumed()` method) using `ActiveEventLoop::create_window()`. The old `EventLoop::run()` is deprecated in favor of `EventLoop::run_app()`. This research provides exact struct names, method signatures, imports, and a complete minimal working example.

## Related Existing Research

None found in `.specs/research/`.

---

## Documentation & References

| Resource | Description | Relevance | Link |
|----------|-------------|-----------|------|
| ApplicationHandler Trait Docs | Official trait definition and method signatures | Primary API reference | [docs.rs](https://docs.rs/winit/latest/winit/application/trait.ApplicationHandler.html) |
| winit 0.30 Changelog | Official migration guide with breaking changes | Critical for understanding API changes | [changelog](https://rust-windowing.github.io/winit/winit/changelog/v0_30/index.html) |
| WindowEvent Enum Docs | Complete enum variants for event handling | Required for event matching | [docs.rs](https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html) |
| KeyEvent Struct Docs | Keyboard input structure and fields | Required for keyboard handling | [docs.rs](https://docs.rs/winit/latest/winit/event/struct.KeyEvent.html) |
| WindowAttributes Docs | Window configuration builder methods | Required for window creation | [docs.rs](https://docs.rs/winit/latest/winit/window/struct.WindowAttributes.html) |
| GitHub Discussion #3667 | Community examples with wgpu integration | Working code verification | [discussion](https://github.com/rust-windowing/winit/discussions/3667) |

### Key Concepts

- **ApplicationHandler**: Trait-based replacement for closure event loops; requires `resumed()` and `window_event()` methods
- **ActiveEventLoop**: Renamed from `EventLoopWindowTarget`; provides `create_window()` and `exit()` methods
- **Deferred Window Creation**: Windows can no longer be created before the event loop runs; must use `resumed()` callback
- **run_app()**: New event loop entry point accepting `&mut impl ApplicationHandler` instead of closure

---

## API Breakdown

### 1. ApplicationHandler Trait

**Complete Trait Signature:**
```rust
pub trait ApplicationHandler<T: 'static = ()> {
    // Required methods
    fn resumed(&mut self, event_loop: &ActiveEventLoop);
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    );

    // Provided methods (optional to override)
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) { ... }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: T) { ... }
    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) { ... }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) { ... }
    fn suspended(&mut self, event_loop: &ActiveEventLoop) { ... }
    fn exiting(&mut self, event_loop: &ActiveEventLoop) { ... }
    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) { ... }
}
```

**Required Methods:**
- `resumed()`: Called when application becomes active; **recommended place to create windows**
- `window_event()`: Receives all OS window events (close, keyboard, redraw, etc.)

---

### 2. Window Creation Pattern

**BREAKING CHANGE:** `Window::new()` has been removed.

**Correct Pattern:**
```rust
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("Ferrum");
            self.window = Some(
                event_loop.create_window(attrs).unwrap()
            );
        }
    }
}
```

**Why:** Windows must be created inside the actively running event loop. This fixes long-standing issues on iOS and macOS.

---

### 3. WindowEvent Handling

**Relevant Event Variants:**

```rust
// CloseRequested - unit variant
WindowEvent::CloseRequested

// KeyboardInput - contains KeyEvent
WindowEvent::KeyboardInput {
    device_id: DeviceId,
    event: KeyEvent,
    is_synthetic: bool,
}

// RedrawRequested - unit variant
WindowEvent::RedrawRequested
```

**KeyEvent Structure:**
```rust
pub struct KeyEvent {
    pub physical_key: PhysicalKey,     // Hardware key position (scancode)
    pub logical_key: Key,              // Interpreted key (affected by layout)
    pub text: Option<SmolStr>,         // Text produced by keypress
    pub location: KeyLocation,         // Left/right/numpad/standard
    pub state: ElementState,           // Pressed or Released
    pub repeat: bool,                  // True if key held down
}
```

**Handling Pattern:**
```rust
fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
    match event {
        WindowEvent::CloseRequested => {
            event_loop.exit();  // Exit the event loop
        }
        WindowEvent::KeyboardInput { event, .. } => {
            println!("{:?}", event);  // Log entire KeyEvent
        }
        WindowEvent::RedrawRequested => {
            // Rendering code
        }
        _ => {}
    }
}
```

---

### 4. Event Loop Execution

**Old (Deprecated):**
```rust
let event_loop = EventLoop::new().unwrap();
event_loop.run(move |event, event_loop| { /* ... */ })
```

**New (Required):**
```rust
let event_loop = EventLoop::new().unwrap();
let mut app = App { window: None };
event_loop.run_app(&mut app).unwrap();
```

**Key Differences:**
- Closure replaced with mutable reference to `ApplicationHandler` implementor
- State moved from closure captures to struct fields
- `run()` replaced with `run_app()`

---

### 5. Required Imports

**Minimal Import Set:**
```rust
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{EventLoop, ActiveEventLoop};
use winit::window::{Window, WindowId};
```

**Additional Imports for Advanced Keyboard Handling:**
```rust
use winit::event::{KeyEvent, ElementState};
use winit::keyboard::{PhysicalKey, KeyCode};
```

---

## Complete Minimal Working Example

This example satisfies all requirements: opens window titled "Ferrum", logs keyboard input, closes on X button.

```rust
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{EventLoop, ActiveEventLoop};
use winit::window::{Window, WindowId};

struct App {
    window: Option<Window>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Ferrum");
            self.window = Some(event_loop.create_window(window_attributes).unwrap());
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Close requested, exiting...");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                println!("Keyboard input: {:?}", event);
            }
            WindowEvent::RedrawRequested => {
                // Rendering would go here
            }
            _ => {}
        }
    }
}

pub fn run() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App { window: None };
    event_loop.run_app(&mut app).unwrap();
}
```

---

## Implementation Guidance

### Installation

Already configured in project:
```toml
[dependencies]
winit = "0.30"
```

### Configuration

None required for basic usage.

### Integration Points

**Current Code Issues (src/gui/mod.rs):**
1. Using deprecated `event_loop.run()` closure pattern
2. Using removed event types directly without proper enum matching
3. Typo: `ReqrawRequested` should be `RedrawRequested`
4. Missing proper imports

**Migration Steps:**
1. Create `App` struct with `Option<Window>` field
2. Implement `ApplicationHandler` trait
3. Move window creation into `resumed()` method
4. Move event matching into `window_event()` method
5. Replace `event_loop.run()` with `event_loop.run_app(&mut app)`

---

## Advanced Keyboard Handling

**Detailed Logging Example:**
```rust
WindowEvent::KeyboardInput { event, .. } => {
    println!("Key: {:?}, State: {:?}, Text: {:?}, Repeat: {}",
        event.logical_key,
        event.state,
        event.text,
        event.repeat
    );
}
```

**Game-Style Key Matching:**
```rust
WindowEvent::KeyboardInput {
    event: KeyEvent {
        physical_key: PhysicalKey::Code(KeyCode::KeyW),
        state: ElementState::Pressed,
        repeat: false,
        ..
    },
    ..
} => {
    println!("W key pressed (not repeated)");
}
```

---

## Potential Issues

| Issue | Impact | Mitigation |
|-------|--------|------------|
| Window not created on some platforms | High - app won't display | Always check `if self.window.is_none()` in `resumed()` |
| Exit not working properly | Medium - app hangs on close | Use `event_loop.exit()` not `std::process::exit()` |
| Keyboard events logged twice | Low - debug noise | Check `is_synthetic` field and filter if needed |
| Unwrap panics on error | Medium - crashes | Replace `.unwrap()` with proper error handling in production |

---

## Recommendations

1. **Use provided minimal example**: Directly addresses all stated requirements with exact API usage
2. **Create window in `resumed()`**: Only correct pattern in winit 0.30; avoid workarounds
3. **Match on WindowEvent exhaustively**: Use `_ => {}` catchall to handle unneeded events
4. **Log full KeyEvent initially**: Provides all keyboard information; can filter later based on needs
5. **Replace unwrap() in production**: Use `Result<(), Box<dyn Error>>` return type and `?` operator

---

## Sources

All sources consulted during research:

- [ApplicationHandler Trait Documentation](https://docs.rs/winit/latest/winit/application/trait.ApplicationHandler.html)
- [winit 0.30 Changelog and Migration Guide](https://rust-windowing.github.io/winit/winit/changelog/v0_30/index.html)
- [WindowEvent Enum Documentation](https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html)
- [KeyEvent Struct Documentation](https://docs.rs/winit/latest/winit/event/struct.KeyEvent.html)
- [WindowAttributes Documentation](https://docs.rs/winit/latest/winit/window/struct.WindowAttributes.html)
- [ActiveEventLoop Documentation](https://docs.rs/winit/latest/winit/event_loop/struct.ActiveEventLoop.html)
- [EventLoop Documentation](https://docs.rs/winit/latest/winit/event_loop/struct.EventLoop.html)
- [GitHub Discussion: winit 0.30 with wgpu](https://github.com/rust-windowing/winit/discussions/3667)
- [Official winit Repository](https://github.com/rust-windowing/winit)

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| Source verification | ✅ | All critical information from official docs.rs and official changelog |
| Recency check | ✅ | All sources for winit 0.30.x (current stable), verified no 0.31 stable exists |
| Alternatives explored | ✅ | Multiple event handling patterns demonstrated (simple logging vs detailed) |
| Actionability | ✅ | Complete copy-pasteable example with exact imports, signatures, and method implementations |
| Evidence quality | ✅ | Primary sources only (official docs + official changelog), supplemented with verified community examples |

Limitations/Caveats:
- Example uses `.unwrap()` for clarity; production code should use proper error handling
- Minimal example doesn't demonstrate graphics rendering (out of scope)
- Advanced features (custom event types, multi-window) not covered (not requested)
