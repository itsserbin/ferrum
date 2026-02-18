---
title: Research - Winit 0.30 API Verification
task_file: User verification request (2026-02-15)
scratchpad: /home/user/apps/ferrum/.specs/scratchpad/a7d3f1c2.md
created: 2026-02-15
status: complete
---

# Research: Winit 0.30 API Verification

## Executive Summary

Winit 0.30.12 is the latest version as of February 2026. It provides stable multi-window support via `ActiveEventLoop::create_window()`, comprehensive keyboard event handling with `KeyEvent` and the generic `Key<SmolStr>` enum, complete mouse event support, and a trait-based `ApplicationHandler` design. The 0.29 to 0.30 migration requires replacing closure-based event loops with trait implementations.

---

## 1. Latest Version

| Package | Latest Version | Date Verified | Source |
|---------|----------------|---------------|--------|
| winit | 0.30.12 | 2026-02-15 | [docs.rs](https://docs.rs/winit/0.30.12/winit/) |

**Status**: 0.30.x is the current stable series. No 0.31 version found.

---

## 2. Multi-Window Support

### Creating Multiple Windows

**YES**, winit 0.30 fully supports multiple windows.

```rust
impl ActiveEventLoop {
    pub fn create_window(
        &self,
        window_attributes: WindowAttributes
    ) -> Result<Window, OsError>
}
```

Windows are created from `ActiveEventLoop` (available in `ApplicationHandler` methods) using `create_window()`. You can call this method multiple times to create multiple windows.

### Event Routing via WindowId

```rust
pub trait ApplicationHandler<T: 'static = ()> {
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent
    );
}
```

Each `window_event()` call receives a `WindowId` that identifies which window generated the event. `WindowId` is obtained via `window.id()` and implements `Copy`, `Clone`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, and `Hash`.

### Multi-Window Pattern

```rust
use std::collections::HashMap;

struct App {
    windows: HashMap<WindowId, Window>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop.create_window(Window::default_attributes()).unwrap();
        self.windows.insert(window.id(), window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if let Some(window) = self.windows.get(&window_id) {
            // Handle event for specific window
        }
    }
}
```

### Example

The `child_window.rs` example in the winit repository demonstrates creating child windows dynamically using the parent window's raw handle via `with_parent_window()`.

---

## 3. Keyboard API

### KeyEvent Struct

```rust
pub struct KeyEvent {
    pub physical_key: PhysicalKey,
    pub logical_key: Key,
    pub text: Option<SmolStr>,
    pub location: KeyLocation,
    pub state: ElementState,
    pub repeat: bool,
}
```

**Fields**:
- `physical_key: PhysicalKey` - Physical key code (hardware scancode)
- `logical_key: Key` - Logical key (accounts for layout and locale)
- `text: Option<SmolStr>` - Text produced by this key press
- `location: KeyLocation` - Physical location on keyboard (left/right/numpad)
- `state: ElementState` - Pressed or Released
- `repeat: bool` - Whether this is a key repeat event

### Key Enum

```rust
pub enum Key<Str = SmolStr> {
    Character(Str),
    Named(NamedKey),
    Dead(Option<char>),
    Unidentified(NativeKey),
}
```

**Key::Character Type**: The `Character` variant contains a generic type `Str` that defaults to `SmolStr`. So by default, it is `Key::Character(SmolStr)`.

To match against character keys more easily, use `key.as_ref()` to convert to `Key::Character(&str)`:

```rust
match event.logical_key.as_ref() {
    Key::Character("a") => { /* ... */ },
    Key::Named(NamedKey::Enter) => { /* ... */ },
    _ => {}
}
```

### NamedKey Variants

Common named keys include:

```rust
pub enum NamedKey {
    Enter,
    Backspace,
    Tab,
    Escape,
    Delete,
    Insert,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    // ... 306+ total variants
}
```

The enum is marked `#[non_exhaustive]` to allow future expansion.

### Checking Modifiers

Modifiers are provided via the `ModifiersChanged` event and the `Modifiers` struct:

```rust
pub struct Modifiers { /* private fields */ }

impl Modifiers {
    pub fn state(&self) -> ModifiersState { /* ... */ }
    pub fn lshift_state(&self) -> ModifiersKeyState { /* ... */ }
    pub fn rshift_state(&self) -> ModifiersKeyState { /* ... */ }
    pub fn lcontrol_state(&self) -> ModifiersKeyState { /* ... */ }
    pub fn rcontrol_state(&self) -> ModifiersKeyState { /* ... */ }
    pub fn lalt_state(&self) -> ModifiersKeyState { /* ... */ }
    pub fn ralt_state(&self) -> ModifiersKeyState { /* ... */ }
    pub fn lsuper_state(&self) -> ModifiersKeyState { /* ... */ }
    pub fn rsuper_state(&self) -> ModifiersKeyState { /* ... */ }
}
```

**ModifiersState** has convenience methods:

```rust
impl ModifiersState {
    pub fn shift_key(&self) -> bool { /* ... */ }
    pub fn control_key(&self) -> bool { /* ... */ }
    pub fn alt_key(&self) -> bool { /* ... */ }
    pub fn super_key(&self) -> bool { /* ... */ }
}
```

**Usage**:

```rust
impl ApplicationHandler for App {
    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::ModifiersChanged(modifiers) => {
                if modifiers.state().control_key() {
                    // Ctrl is pressed
                }
                if modifiers.state().alt_key() {
                    // Alt is pressed
                }
                if modifiers.state().shift_key() {
                    // Shift is pressed
                }
            }
            _ => {}
        }
    }
}
```

Store the `Modifiers` state in your application struct to reference it during `KeyboardInput` events.

---

## 4. Mouse Events

### CursorMoved

```rust
WindowEvent::CursorMoved {
    device_id: DeviceId,
    position: PhysicalPosition<f64>
}
```

**Fields**:
- `device_id: DeviceId` - The device that generated the event
- `position: PhysicalPosition<f64>` - Cursor position in physical pixels

### MouseInput

```rust
WindowEvent::MouseInput {
    device_id: DeviceId,
    state: ElementState,
    button: MouseButton
}
```

**Fields**:
- `device_id: DeviceId` - The device that generated the event
- `state: ElementState` - Pressed or Released
- `button: MouseButton` - Left, Right, Middle, Back, Forward, or Other(u16)

### CursorLeft

```rust
WindowEvent::CursorLeft {
    device_id: DeviceId
}
```

**YES**, the `CursorLeft` event exists and indicates the cursor has left the window's client area.

**Related Events**:
- `CursorEntered { device_id: DeviceId }` - Cursor entered the window

---

## 5. ApplicationHandler Trait

```rust
pub trait ApplicationHandler<T: 'static = ()> {
    // Required methods
    fn resumed(&mut self, event_loop: &ActiveEventLoop);
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent
    );

    // Provided methods (all have default empty implementations)
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) { }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: T) { }
    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent
    ) { }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) { }
    fn suspended(&mut self, event_loop: &ActiveEventLoop) { }
    fn exiting(&mut self, event_loop: &ActiveEventLoop) { }
    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) { }
}
```

### about_to_wait() Method

**YES**, `about_to_wait()` exists as a provided method.

**Signature**:
```rust
fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) { }
```

**Description**: Emitted when the event loop is about to block and wait for new events. This is a good place to trigger window redraws.

**Usage**:
```rust
impl ApplicationHandler for App {
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        for window in self.windows.values() {
            window.request_redraw();
        }
    }
}
```

---

## 6. Breaking Changes (0.29 → 0.30)

| Change | 0.29 API | 0.30 API |
|--------|----------|----------|
| Event loop | `EventLoop::run` with closure | `EventLoop::run_app` with `ApplicationHandler` trait |
| Window creation | `Window::new()` before event loop | `ActiveEventLoop::create_window()` inside event loop |
| Window builder | `WindowBuilder` | `WindowAttributes` |
| Event loop target | `EventLoopWindowTarget` | `ActiveEventLoop` |
| Cursor icon | `set_cursor_icon` | `set_cursor` |
| MSRV | 1.65 | 1.70 |

### Key Migration Points

1. **Window Creation**: Windows can only be created within the active event loop (typically in `resumed()` or `new_events(cause: StartCause::Init)`).

2. **Trait-Based Handler**: Replace closure pattern-matching with trait method implementations:
   ```rust
   // 0.29
   event_loop.run(move |event, elwt| {
       match event { /* ... */ }
   });

   // 0.30
   struct App { /* ... */ }
   impl ApplicationHandler for App { /* ... */ }
   event_loop.run_app(&mut app);
   ```

3. **Generic Parameter Removed**: `ActiveEventLoop` no longer has the user event generic parameter directly (it's on the trait instead).

---

## Sources

- [winit 0.30.12 Documentation](https://docs.rs/winit/0.30.12/winit/)
- [WindowEvent Enum](https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html)
- [KeyEvent Struct](https://docs.rs/winit/latest/winit/event/struct.KeyEvent.html)
- [Key Enum](https://docs.rs/winit/latest/winit/keyboard/enum.Key.html)
- [NamedKey Enum](https://docs.rs/winit/latest/winit/keyboard/enum.NamedKey.html)
- [Modifiers Struct](https://docs.rs/winit/latest/winit/event/struct.Modifiers.html)
- [ModifiersState](https://docs.rs/winit/latest/winit/keyboard/struct.ModifiersState.html)
- [ApplicationHandler Trait](https://docs.rs/winit/latest/winit/application/trait.ApplicationHandler.html)
- [ActiveEventLoop](https://docs.rs/winit/latest/winit/event_loop/struct.ActiveEventLoop.html)
- [WindowId](https://docs.rs/winit/latest/winit/window/struct.WindowId.html)
- [Winit Changelog](https://docs.rs/winit/latest/winit/changelog/index.html)
- [Winit 0.30 Changelog](https://rust-windowing.github.io/winit/winit/changelog/v0_30/index.html)
- [Winit GitHub Repository](https://github.com/rust-windowing/winit)

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| Source verification | ✅ | All information from official docs.rs documentation |
| Recency check | ✅ | Version 0.30.12 confirmed as of 2026-02-15 |
| Alternatives explored | N/A | API verification task, not comparison |
| Actionability | ✅ | Exact Rust type signatures provided for all queries |
| Evidence quality | ✅ | All facts from official documentation |

Limitations/Caveats: None - all requested API details verified with exact type signatures.
