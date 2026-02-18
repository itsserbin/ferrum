---
title: Research - macOS Native Tab Bar "+" Button Implementation
task_file: User request (ad-hoc)
scratchpad: .specs/scratchpad/0927d97d.md
created: 2026-02-16
status: complete
---

# Research: macOS Native Tab Bar "+" Button Implementation

## Executive Summary

The macOS native tab bar "+" button appears ONLY when `newWindowForTab(_:)` exists in the responder chain starting from the window. Ferrum's current implementation adds this method to NSApplication's metaclass using `class_addMethod`, which is incorrect for two reasons: (1) it adds a class method instead of an instance method, and (2) even if corrected, NSApplication is too high in the responder chain. The solution is to create NSWindowController wrappers using objc2's `define_class!` macro, implement `newWindowForTab:` there, and associate each controller with its window. This matches the standard AppKit pattern described in Christian Tietze's authoritative guides. No other Rust terminal emulator has implemented native macOS tabs successfully.

## Related Existing Research

- **research-macos-native-tabs.md**: Covers basic native tab implementation (tabbingMode, tabbingIdentifier, addTabbedWindow) but does not address the "+" button responder chain issue.

---

## Documentation & References

| Resource | Description | Relevance | Link |
|----------|-------------|-----------|------|
| NSResponder.newWindowForTab | Official Apple documentation for tab creation selector | Core method that enables "+" button | https://developer.apple.com/documentation/appkit/nsresponder/1644675-newwindowfortab |
| NSWindowController | Apple documentation for window controller class | Recommended location for newWindowForTab implementation | https://developer.apple.com/documentation/appkit/nswindowcontroller |
| NSApplicationDelegate | Application delegate protocol documentation | Alternative fallback location | https://developer.apple.com/documentation/appkit/nsapplicationdelegate |
| Christian Tietze - Programmatically Add Tabs | Authoritative guide on NSWindow tabbing without NSDocument | Explains responder chain requirements and windowController necessity | https://christiantietze.de/posts/2019/01/programmatically-add-nswindow-tabs/ |
| Christian Tietze - Multiple NSWindowControllers | Guide for managing multiple window controllers with tabs | Shows proper controller lifecycle management | https://christiantietze.de/posts/2019/07/nswindow-tabbing-multiple-nswindowcontroller/ |
| Christian Tietze - Cocoa Responder Chain | Deep dive into AppKit responder chain architecture | Explains chain order and pseudo-responder behavior | https://christiantietze.de/posts/2023/08/cocoa-appkit-responder-chain/ |
| objc2 define_class! macro | Rust macro for creating Objective-C classes | Implementation tool for NSWindowController subclass | https://docs.rs/objc2/latest/objc2/macro.define_class.html |
| objc2-app-kit NSWindowController | Rust bindings for NSWindowController | API reference for available methods | https://docs.rs/objc2-app-kit/latest/objc2_app_kit/struct.NSWindowController.html |
| objc2 hello_world_app.rs | Example showing delegate implementation pattern | Reference for define_class! syntax | https://github.com/madsmtm/objc2/blob/main/examples/app/hello_world_app.rs |
| winit Issue #4260 - macOS AppDelegates | Discussion of custom delegates in winit 0.30 | Confirms winit doesn't create delegates, safe to add custom | https://github.com/rust-windowing/winit/issues/4260 |
| Electron PR #9725 - Native Tab Button | Electron's implementation of native macOS tabs | Cross-platform reference for similar issue | https://github.com/electron/electron/pull/9725 |

### Key Concepts

- **Responder Chain**: macOS event handling architecture where messages flow from first responder (view/window) up through controller, application, and delegate objects
- **newWindowForTab(_:)**: NSResponder method that creates new window for tab; presence in responder chain enables "+" button
- **NSWindowController**: Coordinator object that manages an NSWindow; automatically inserted in responder chain between window and application
- **define_class!**: objc2 macro for creating Objective-C classes from Rust with type safety and protocol conformance
- **class_addMethod vs instance method**: Objective-C runtime distinction - class methods added to metaclass, instance methods to class itself
- **Pseudo-responder**: Object like NSWindowDelegate that gets action messages but isn't formally in responder chain

---

## The Root Cause

### Three Conditions for "+" Button

The native tab bar "+" button appears when ALL three conditions are met:

1. ✅ **tabbingMode set**: `NSWindowTabbingMode::Preferred` (or Automatic) - Ferrum does this
2. ✅ **Tab bar visible**: `NSWindowTabGroup.setTabBarVisible(true)` - Ferrum does this
3. ❌ **newWindowForTab in responder chain**: Must be reachable from KEY WINDOW - **Ferrum doesn't do this correctly**

### What's Wrong with Current Implementation

Current code in `install_new_tab_responder()`:

```rust
let cls = object_getClass(Retained::as_ptr(&app).cast());
class_addMethod(cls, sel, ...);
```

**Problem 1: Wrong Target**

`object_getClass()` returns the **metaclass** of NSApplication, not its instance class. This adds a **class method** (callable as `NSApplication.newWindowForTab:`), not an **instance method** (callable as `app_instance.newWindowForTab:`). The responder chain looks for instance methods.

**Problem 2: Wrong Level in Responder Chain**

Even if we fixed Problem 1, adding to NSApplication is too high in the chain. The responder chain flow:

```
[User clicks "+"]
  → NSView
  → NSWindow (key window)
  → NSWindowController ← MISSING IN WINIT!
  → NSApplication
  → NSApplicationDelegate
```

The system checks for `newWindowForTab:` starting from the key window. Without NSWindowController, the chain breaks immediately after NSWindow, never reaching NSApplication.

**Problem 3: No Controller Per Window**

Christian Tietze quote: "When you initialize a new window, set `window.windowController = self` to make sure the new tab forwards the responder chain messages."

Without windowController set, each new tab becomes a "dead end" - it can't forward responder messages, so the "+" button only works from the first window (and even that fails due to Problems 1 and 2).

---

## Libraries & Tools

| Name | Purpose | Maturity | Notes |
|------|---------|----------|-------|
| objc2 0.6 | Core Objective-C runtime bindings | Stable | Already in Ferrum; provides define_class! macro |
| objc2-app-kit 0.3 | AppKit framework bindings | Stable | Already in Ferrum; provides NSWindowController APIs |
| winit 0.30 | Cross-platform windowing | Stable | Doesn't create NSWindowController (by design); allows custom delegates |

### Recommended Stack

**Continue with existing dependencies** - no new crates needed. Use `define_class!` macro to create NSWindowController subclass, leveraging existing objc2 infrastructure.

---

## Patterns & Approaches

### Pattern 1: NSWindowController Subclass (Recommended)

**When to use**: Native macOS tabs with proper responder chain

**Trade-offs**:
- **Pros**: Matches Apple's intended architecture; responder chain works naturally; proven pattern from expert guides; can store per-window state
- **Cons**: More complex than single-point solution; requires lifecycle management; must store controllers to prevent deallocation; novel in Rust ecosystem (no examples found)

**Implementation**:
1. Create NSWindowController subclass with `define_class!`
2. Implement `newWindowForTab:` method with `#[method(...)]` attribute
3. Initialize controller with `initWithWindow` for each NSWindow
4. Store `Retained<Controller>` in app state to prevent premature deallocation
5. Let existing window close notifications clean up controllers

**Verification**:
- objc2-app-kit 0.3 provides `initWithWindow`, `window()`, `setWindow()` - all APIs present ✅
- `define_class!` supports NSWindowController as superclass (any NSObject subclass works) ✅
- Pattern used in Swift/Objective-C extensively; translates to Rust ✅

### Pattern 2: Custom NSApplicationDelegate (Partial Solution)

**When to use**: Fallback for when all windows are closed

**Trade-offs**:
- **Pros**: Simpler than controllers; single central handler; good for app-level actions
- **Cons**: Too high in responder chain for per-window actions; uncertain if enables "+" button for multiple tabs; may only work for first window

**Implementation**:
1. Create NSObject subclass implementing NSApplicationDelegate
2. Add `newWindowForTab:` method
3. Set as `NSApplication.sharedApplication.delegate` after EventLoop creation
4. Store delegate instance to prevent deallocation

**Note**: This alone is INSUFFICIENT based on Christian Tietze's analysis, but useful as supplement to Pattern 1.

### Pattern 3: NSWindowDelegate (Not Viable)

**When to use**: Don't use for `newWindowForTab`

**Trade-offs**:
- **Pros**: Simple, winit already uses window delegates
- **Cons**: `newWindowForTab` is NOT an NSWindowDelegate method; it's NSResponder method; delegate acts as "pseudo-responder" but doesn't expose this selector; no examples found

**Verdict**: Research confirms this pattern doesn't work for `newWindowForTab`. NSWindowDelegate has different methods (windowWillClose, windowDidResize, etc.).

---

## Similar Implementations

### Other Rust Terminal Emulators

**Alacritty**:
- Source: https://github.com/alacritty/alacritty/issues/1544
- **Does NOT support native macOS tabs** - by design, prefers tmux/window manager approach
- Has multi-window support via CreateNewWindow action but no tab integration
- Applicability: No reference implementation available

**WezTerm**:
- Source: https://github.com/wezterm/wezterm/issues/4381
- **Native tabs marked "wontfix"** - uses custom cross-platform tab bar
- Applicability: No reference implementation available

**Rio Terminal**:
- Source: https://github.com/raphamorim/rio
- Custom tab implementation with TopTab, BottomTab, Breadcrumb modes
- Applicability: No reference implementation available

**Conclusion**: No Rust terminal emulator has successfully implemented native macOS tabs with the "+" button. Ferrum would be the first.

### Electron's Implementation

- Source: https://github.com/electron/electron/pull/9725
- Electron exposes native macOS tabs via JavaScript bridge when `tabbingIdentifier` set
- Shows cross-platform apps can conditionally use native tabs on macOS
- Applicability: Demonstrates feasibility but different FFI approach (C++ vs Rust)

---

## Potential Issues

| Issue | Impact | Mitigation |
|-------|--------|------------|
| define_class! syntax complexity | **Medium** - NSWindowController subclass more complex than NSObject subclass | Follow objc2 hello_world_app.rs example pattern; use `#[unsafe(super(NSWindowController))]` attribute; test incrementally |
| initWithWindow method signature | **High** - Incorrect signature causes runtime crash | Verify with objc2-app-kit docs; use `msg_send_id!` for init methods; wrap in unsafe block with proper null checking |
| Controller lifecycle management | **High** - Deallocated controller breaks responder chain | Store `Vec<Retained<FermWindowController>>` in app state; use strong references; remove on window close notifications |
| winit delegate compatibility | **Low** - winit might interfere with custom setup | winit 0.30 guarantees no delegate registration (Issue #4260); safe to add custom delegates after EventLoop creation |
| Timing of controller attachment | **Medium** - Setting controller after window shown may not enable button | Attach controller BEFORE calling makeKeyAndOrderFront; test both orders to confirm |
| No Rust precedent | **Medium** - No existing Rust examples to reference | Translate Swift patterns from Christian Tietze's guides; validate against Apple docs; expect some trial-and-error |
| objc2 version compatibility | **Low** - API changes between objc2 versions | Ferrum uses objc2 0.6 (current); define_class! API stable since 0.5; lock dependency versions |

---

## Recommendations

1. **Create NSWindowController subclass with newWindowForTab**: Use objc2's `define_class!` macro to create `FermWindowController` that inherits from NSWindowController. Implement `newWindowForTab:` as instance method with `#[method(newWindowForTab:)]` attribute. Set atomic flag when called, poll in event loop to create new tab. This is the PRIMARY solution.

2. **Initialize controller for each window**: After creating each NSWindow via winit, create FermWindowController with `initWithWindow`. Store `Retained<FermWindowController>` in `Vec` in app state (must keep alive or controller deallocates). Ensure controller created BEFORE calling `makeKeyAndOrderFront` or showing window.

3. **Add NSApplicationDelegate fallback**: Create custom app delegate with `define_class!`, implement `newWindowForTab:` there as well. Set as `NSApplication.sharedApplication.delegate` after EventLoop creation but before window creation. This handles edge case when all windows are closed and user triggers Cmd+T. This is SUPPLEMENTARY, not sufficient alone.

4. **Remove current class_addMethod approach**: Delete `install_new_tab_responder()` function entirely. The metaclass modification is incorrect and doesn't work. Responder chain needs instance methods on proper objects, not class methods on NSApplication.

5. **Test responder chain thoroughly**: Verify "+" button appears in single-tab state; verify clicking "+" triggers handler; verify new tabs ALSO show "+" button (tests controller lifecycle); verify Cmd+T keyboard shortcut works; verify Window menu "New Tab" item enabled. If "+" button still doesn't appear, check if controller needs to override additional NSResponder methods.

---

## Implementation Guidance

### Installation

**No new dependencies required.** Ferrum already has:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.6"
objc2-app-kit = "0.3"
```

Verify `NSWindow` feature is enabled (for `initWithWindow` method):

```toml
objc2-app-kit = { version = "0.3", features = ["NSWindow", "NSWindowController"] }
```

### Configuration

**Step 1: Define NSWindowController Subclass**

Create in `src/gui/platform/macos.rs`:

```rust
use objc2::define_class;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSWindow, NSWindowController};
use objc2_foundation::{NSObject, MainThreadMarker};
use std::sync::atomic::{AtomicBool, Ordering};

static NEW_TAB_REQUESTED: AtomicBool = AtomicBool::new(false);

define_class!(
    #[unsafe(super(NSWindowController))]
    #[thread_kind = MainThreadOnly]
    #[name = "FermWindowController"]
    #[derive(Debug)]
    pub struct FermWindowController;

    unsafe impl FermWindowController {
        #[method(newWindowForTab:)]
        fn new_window_for_tab(&self, _sender: Option<&AnyObject>) {
            // Set flag that event loop polls
            NEW_TAB_REQUESTED.store(true, Ordering::SeqCst);
        }
    }
);

impl FermWindowController {
    /// Creates controller and associates with window
    pub fn new_with_window(window: &NSWindow, mtm: MainThreadMarker) -> Retained<Self> {
        unsafe {
            let controller: Retained<Self> = msg_send_id![
                mtm.alloc::<Self>(),
                initWithWindow: Some(window)
            ];
            controller
        }
    }
}

/// Call from event loop to check if "+" button was clicked
pub fn take_new_tab_request() -> bool {
    NEW_TAB_REQUESTED.swap(false, Ordering::SeqCst)
}
```

**Step 2: Modify configure_native_tabs**

Update existing function to return controller:

```rust
pub fn configure_native_tabs(
    window: &Window,
    mtm: MainThreadMarker
) -> Option<Retained<FermWindowController>> {
    let Some(ns_window) = get_ns_window(window) else {
        return None;
    };

    unsafe {
        ns_window.setTabbingMode(NSWindowTabbingMode::Preferred);
        let identifier = ns_string!("com.ferrum.terminal");
        let _: () = msg_send![&ns_window, setTabbingIdentifier: identifier];

        // CRITICAL: Create controller to enable responder chain
        let controller = FermWindowController::new_with_window(&ns_window, mtm);

        Some(controller)
    }
}
```

**Step 3: Update App State**

Add storage for controllers:

```rust
pub struct App {
    // Existing fields...

    #[cfg(target_os = "macos")]
    window_controllers: Vec<Retained<FermWindowController>>,
}
```

**Step 4: Create Windows with Controllers**

When creating windows:

```rust
#[cfg(target_os = "macos")]
{
    let mtm = MainThreadMarker::new().unwrap();
    if let Some(controller) = configure_native_tabs(&window, mtm) {
        self.window_controllers.push(controller);

        // If we have existing windows, add this as tab
        if self.window_controllers.len() > 1 {
            // Use existing add_as_tab logic
            if let Some(first_window) = self.windows.first() {
                add_as_tab(first_window, &window);
            }
        }
    }
}
```

**Step 5: Poll for Tab Requests**

In event loop:

```rust
#[cfg(target_os = "macos")]
if macos::take_new_tab_request() {
    self.create_new_tab(event_loop);
}
```

**Step 6: Remove Old Responder Installation**

Delete the `install_new_tab_responder()` function and its call - no longer needed.

### Integration Points

**File: src/gui/platform/macos.rs**
- Add `FermWindowController` class definition
- Modify `configure_native_tabs` to create and return controller
- Keep existing `get_ns_window`, `add_as_tab`, `show_tab_bar` functions
- Keep `take_new_tab_request` polling function

**File: src/gui/app.rs or src/gui/state.rs**
- Add `window_controllers: Vec<Retained<FermWindowController>>` field with `#[cfg(target_os = "macos")]`
- Store controller when creating each window
- Remove controller when window closes (match window ID to controller index)

**File: src/gui/events.rs or event loop**
- Poll `take_new_tab_request()` in macOS-specific code path
- Trigger `create_new_tab()` when flag is set

**File: src/main.rs or initialization**
- Remove call to `install_new_tab_responder()` (delete entire function)
- Ensure EventLoop created before any macOS-specific setup (winit requirement)

---

## Code Examples

### Complete NSWindowController Subclass

```rust
use objc2::define_class;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSWindow, NSWindowController};
use objc2_foundation::{NSObject, MainThreadMarker};
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag set when native "+" button clicked
static NEW_TAB_REQUESTED: AtomicBool = AtomicBool::new(false);

define_class!(
    /// Custom window controller that enables native tab bar "+" button
    #[unsafe(super(NSWindowController))]
    #[thread_kind = MainThreadOnly]
    #[name = "FermWindowController"]
    #[derive(Debug)]
    pub struct FermWindowController;

    // No instance variables needed for basic implementation

    unsafe impl FermWindowController {
        /// Handles native macOS "+" button and Cmd+T shortcut
        ///
        /// This method MUST be present in the responder chain for the
        /// "+" button to appear in the tab bar. Simply implementing it
        /// enables the button.
        #[method(newWindowForTab:)]
        fn new_window_for_tab(&self, _sender: Option<&AnyObject>) {
            // Set atomic flag that event loop will poll
            NEW_TAB_REQUESTED.store(true, Ordering::SeqCst);

            // Alternative: could use channel/callback here instead of atomic flag
        }
    }
);

impl FermWindowController {
    /// Creates a new controller associated with the given window
    ///
    /// MUST be called BEFORE showing the window to enable "+" button
    pub fn new_with_window(window: &NSWindow, mtm: MainThreadMarker) -> Retained<Self> {
        unsafe {
            // Use msg_send_id! for init methods to get proper retain semantics
            let controller: Retained<Self> = msg_send_id![
                mtm.alloc::<Self>(),
                initWithWindow: Some(window)
            ];
            controller
        }
    }
}

/// Returns true if "+" button was clicked since last call (consumes flag)
pub fn take_new_tab_request() -> bool {
    NEW_TAB_REQUESTED.swap(false, Ordering::SeqCst)
}
```

### Integration Example

```rust
// In window creation code:

#[cfg(target_os = "macos")]
fn create_window_macos(
    &mut self,
    event_loop: &ActiveEventLoop,
    mtm: MainThreadMarker
) -> WindowId {
    // Create winit window as usual
    let window = event_loop.create_window(attributes).unwrap();
    let window_id = window.id();

    // Configure native tabs and get controller
    if let Some(controller) = macos::configure_native_tabs(&window, mtm) {
        // Store controller to prevent deallocation
        self.window_controllers.push(controller);

        // Show tab bar explicitly (makes "+" visible even with one tab)
        macos::show_tab_bar(&window);

        // If we have existing windows, add this as a tab
        if let Some(first_window) = self.windows.first() {
            macos::add_as_tab(first_window, &window);
        }
    }

    window_id
}

// In event loop:

fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    // ... existing code ...

    #[cfg(target_os = "macos")]
    {
        let mtm = MainThreadMarker::new().unwrap();

        // Check if "+" button was clicked
        if macos::take_new_tab_request() {
            self.create_window_macos(event_loop, mtm);
        }
    }
}
```

### Alternative: NSApplicationDelegate Fallback

```rust
// For when all windows are closed and user presses Cmd+T:

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FermAppDelegate"]
    struct AppDelegate;

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[method(newWindowForTab:)]
        fn new_window_for_tab(&self, _sender: Option<&AnyObject>) {
            NEW_TAB_REQUESTED.store(true, Ordering::SeqCst);
        }
    }
);

// Set delegate after EventLoop creation:
unsafe {
    let app = NSApplication::sharedApplication(mtm);
    let delegate = AppDelegate::new(mtm);
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    // Must store delegate to prevent deallocation!
}
```

---

## Sources

All sources consulted with URLs for verification:

### Apple Official Documentation
- [NSResponder.newWindowForTab(_:)](https://developer.apple.com/documentation/appkit/nsresponder/1644675-newwindowfortab)
- [NSWindowController](https://developer.apple.com/documentation/appkit/nswindowcontroller)
- [NSApplicationDelegate](https://developer.apple.com/documentation/appkit/nsapplicationdelegate)
- [NSWindowDelegate](https://developer.apple.com/documentation/appkit/nswindowdelegate)
- [NSWindowTabGroup](https://developer.apple.com/documentation/appkit/nswindowtabgroup)
- [Event Architecture - Responder Chain](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/EventOverview/EventArchitecture/EventArchitecture.html)

### Christian Tietze's Expert Guides
- [Programmatically Add Tabs to NSWindows without NSDocument](https://christiantietze.de/posts/2019/01/programmatically-add-nswindow-tabs/)
- [Implement NSWindow Tabbing with Multiple NSWindowControllers](https://christiantietze.de/posts/2019/07/nswindow-tabbing-multiple-nswindowcontroller/)
- [The World's Most Comprehensive Guide to NSWindow Tabbing Single NSWindowController](https://christiantietze.de/posts/2019/07/nswindow-tabbing-single-nswindowcontroller/)
- [Cocoa AppKit Responder Chain](https://christiantietze.de/posts/2023/08/cocoa-appkit-responder-chain/)

### Rust Bindings Documentation
- [objc2 define_class! macro](https://docs.rs/objc2/latest/objc2/macro.define_class.html)
- [objc2-app-kit NSWindowController](https://docs.rs/objc2-app-kit/latest/objc2_app_kit/struct.NSWindowController.html)
- [objc2-app-kit NSWindow](https://docs.rs/objc2-app-kit/latest/objc2_app_kit/struct.NSWindow.html)
- [objc2 hello_world_app.rs example](https://github.com/madsmtm/objc2/blob/main/examples/app/hello_world_app.rs)
- [objc2 GitHub repository](https://github.com/madsmtm/objc2)

### winit Issues and Documentation
- [winit Issue #4260 - macOS AppDelegates](https://github.com/rust-windowing/winit/issues/4260)
- [winit Issue #4015 - Custom NSApplicationDelegate panicking](https://github.com/rust-windowing/winit/issues/4015)
- [winit platform::macos documentation](https://smithay.github.io/smithay/winit/platform/macos/index.html)
- [winit v0.30 changelog](https://rust-windowing.github.io/winit/winit/changelog/v0_30/index.html)

### Terminal Emulator Projects
- [Alacritty Issue #1544 - Investigate macOS tabs](https://github.com/alacritty/alacritty/issues/1544)
- [WezTerm Issue #4381 - Native macOS tabs](https://github.com/wezterm/wezterm/issues/4381)
- [Rio Terminal GitHub](https://github.com/raphamorim/rio)
- [Electron PR #9725 - Native Tab Button](https://github.com/electron/electron/pull/9725)

### Community Resources
- [ResponderChain - CocoaDev](https://cocoadev.github.io/ResponderChain/)
- [Apple Developer Forums - Manually creating new tabs](https://developer.apple.com/forums/thread/61416)
- [Apple Developer Forums - Capture NSWindow tab events](https://developer.apple.com/forums/thread/126293)

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| Source verification | ✅ | All claims verified against Apple documentation; Christian Tietze's guides (2019-2023) remain canonical references for non-NSDocument tabs; objc2 APIs verified via docs.rs |
| Recency check | ✅ | Apple APIs unchanged since macOS Sierra (2016); winit 0.30 issues from 2024 (current); objc2 0.6 current stable; NSWindow tabbing APIs stable across macOS versions |
| Alternatives explored | ✅ | 3 alternatives evaluated: (1) NSWindowController [RECOMMENDED], (2) NSApplicationDelegate [PARTIAL], (3) NSWindowDelegate [NOT VIABLE]; 4th option (NSWindow subclass) mentioned but dismissed as too invasive |
| Actionability | ⚠️ | Complete code examples provided with define_class! macro; integration steps detailed; HOWEVER: code not compiled/tested (synthesized from documentation); initWithWindow signature may need adjustment; no Rust precedent to validate against |
| Evidence quality | ✅ | Root cause identified with clear evidence (metaclass vs instance class, responder chain order); Christian Tietze's Swift examples directly applicable; distinguish facts (Apple docs) from inferences (Rust translation) |

**Limitations/Caveats**:
- **No compiled proof**: Code examples synthesized from documentation, not tested in actual Ferrum build
- **define_class! syntax**: NSWindowController superclass may need different syntax than NSObject examples
- **No Rust precedent**: No other Rust terminal has implemented this - pioneer territory, expect iteration
- **Controller lifecycle**: Window close notification handling not fully detailed
- **initWithWindow binding**: Assumes objc2-app-kit exposes this method correctly (docs confirm it exists but haven't tested)
