use std::sync::atomic::{AtomicBool, Ordering};

use objc2::MainThreadMarker;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2_app_kit::{
    NSFloatingWindowLevel, NSNormalWindowLevel,
    NSBezelStyle, NSButton, NSImage, NSLayoutAttribute, NSTitlebarAccessoryViewController, NSView,
    NSWindow, NSWindowTabbingMode,
};
use objc2_foundation::ns_string;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

/// Flag: native "+" button was clicked, need to create a new tab.
static NEW_TAB_REQUESTED: AtomicBool = AtomicBool::new(false);

// SAFETY: These are raw Objective-C runtime C functions from libobjc.dylib.
// They have a stable ABI on macOS and are the foundation of the Objective-C runtime.
// Using raw FFI declarations avoids potential version conflicts with objc2 crate internals.
// All functions are well-documented in Apple's Objective-C Runtime Reference:
// - object_getClass: Returns the class of an object (always valid for valid objects)
// - sel_registerName: Registers a selector name and returns its unique identifier
// - class_addMethod: Adds a method to a class (returns false if method already exists)
// - class_replaceMethod: Replaces or adds a method implementation in a class
// - objc_msgSend: The universal message dispatch function for Objective-C
unsafe extern "C" {
    fn object_getClass(obj: *const core::ffi::c_void) -> *mut core::ffi::c_void;
    fn sel_registerName(name: *const core::ffi::c_char) -> *const core::ffi::c_void;
    fn class_addMethod(
        cls: *mut core::ffi::c_void,
        name: *const core::ffi::c_void,
        imp: unsafe extern "C" fn(),
        types: *const core::ffi::c_char,
    ) -> bool;
    fn class_replaceMethod(
        cls: *mut core::ffi::c_void,
        name: *const core::ffi::c_void,
        imp: unsafe extern "C" fn(),
        types: *const core::ffi::c_char,
    );
    fn objc_msgSend(
        obj: *const core::ffi::c_void,
        sel: *const core::ffi::c_void,
        ...
    ) -> *mut core::ffi::c_void;
}

/// ObjC method implementation for `newWindowForTab:`.
///
/// # Safety
///
/// This function is designed to be called by the Objective-C runtime as a method
/// implementation. It must have the correct calling convention (extern "C") and
/// signature matching the Objective-C method type encoding "v@:@":
/// - `_this`: The receiver object (self in Objective-C)
/// - `_cmd`: The selector being invoked
/// - `_sender`: The sender object passed to the action method
///
/// The function only performs an atomic store operation, which is inherently safe.
/// The parameters are unused but must be present to match the expected signature.
unsafe extern "C" fn handle_new_window_for_tab(
    _this: *mut core::ffi::c_void,
    _cmd: *const core::ffi::c_void,
    _sender: *mut core::ffi::c_void,
) {
    NEW_TAB_REQUESTED.store(true, Ordering::SeqCst);
}

/// Extracts the NSWindow from a winit Window via raw-window-handle.
fn get_ns_window(window: &Window) -> Option<Retained<NSWindow>> {
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return None;
    };
    // SAFETY: The ns_view pointer from RawWindowHandle::AppKit is guaranteed to be a valid
    // NSView pointer while the Window is alive. We hold a reference to the Window, so the
    // view remains valid. The cast from NonNull<c_void> to NSView is sound because AppKit
    // windows always have an NSView as their content view. Retained::retain creates a new
    // strong reference, and ns_view.window() safely returns the owning NSWindow.
    unsafe {
        let ns_view: Retained<NSView> = Retained::retain(appkit.ns_view.as_ptr().cast())?;
        ns_view.window()
    }
}

/// Configures a window for native macOS tab grouping.
pub fn configure_native_tabs(window: &Window) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };
    // SAFETY: ns_window is a valid, retained NSWindow obtained from get_ns_window().
    // setTabbingMode is a safe Objective-C method on NSWindow that sets the tabbing behavior.
    // The msg_send! macro for setTabbingIdentifier: is safe because:
    // 1. The selector "setTabbingIdentifier:" exists on NSWindow (macOS 10.12+)
    // 2. The ns_string! macro creates a valid NSString
    // 3. The return type () matches the void return of the Objective-C method
    unsafe {
        ns_window.setTabbingMode(NSWindowTabbingMode::Preferred);
        let identifier = ns_string!("com.ferrum.terminal");
        let _: () = msg_send![&ns_window, setTabbingIdentifier: identifier];
    }
}

/// Adds `newWindowForTab:` to every level of the responder chain.
/// Uses class_replaceMethod (handles both "add new" and "replace existing no-op" cases).
/// Targets: window class, window delegate class, NSApp class, NSApp delegate class.
pub fn install_new_tab_handler(window: &Window) {
    static INSTALLED: AtomicBool = AtomicBool::new(false);
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    let Some(ns_window) = get_ns_window(window) else {
        INSTALLED.store(false, Ordering::SeqCst);
        return;
    };

    // SAFETY: This block performs Objective-C runtime manipulation to install
    // a custom method handler for the native "+" tab button. The operations are safe because:
    //
    // 1. sel_registerName: The selector string "newWindowForTab:" is a valid C string
    //    and corresponds to the standard AppKit method for creating new tabs.
    //
    // 2. core::mem::transmute for the function pointer: The handle_new_window_for_tab
    //    function has the correct Objective-C calling convention (extern "C") and
    //    signature (self, _cmd, sender) matching the expected method signature "v@:@"
    //    (void return, object self, selector, object argument).
    //
    // 3. class_addMethod/class_replaceMethod: We only call these on valid class pointers
    //    obtained from object_getClass, with null checks before each injection.
    //
    // 4. object_getClass: Called on valid Objective-C objects (ns_window, delegates).
    //    Returns the class object which is always valid for a valid object.
    //
    // 5. objc_msgSend: Used to call "delegate" and "sharedApplication" selectors which
    //    are standard AppKit methods that return valid objects or nil.
    //
    // 6. objc_getClass: "NSApplication" is a valid AppKit class name.
    //
    // The INSTALLED atomic flag ensures this only runs once, preventing duplicate
    // method installations.
    unsafe {
        let sel = sel_registerName(c"newWindowForTab:".as_ptr());
        let imp: unsafe extern "C" fn() =
            core::mem::transmute(handle_new_window_for_tab as unsafe extern "C" fn(_, _, _));
        let types = c"v@:@".as_ptr();

        // Helper: try add, then replace if already exists.
        let inject = |cls: *mut core::ffi::c_void| {
            if cls.is_null() {
                return;
            }
            if !class_addMethod(cls, sel, imp, types) {
                // Method already exists (e.g. winit no-op stub) — replace it.
                class_replaceMethod(cls, sel, imp, types);
            }
        };

        // 1) Window class (NSWindow subclass from winit).
        let win_cls = object_getClass(Retained::as_ptr(&ns_window).cast());
        inject(win_cls);

        // 2) Window delegate class.
        let sel_delegate = sel_registerName(c"delegate".as_ptr());
        let win_delegate = objc_msgSend(Retained::as_ptr(&ns_window).cast(), sel_delegate);
        if !win_delegate.is_null() {
            inject(object_getClass(win_delegate));
        }

        // 3) NSApplication class.
        let sel_shared = sel_registerName(c"sharedApplication".as_ptr());
        let ns_app_cls_ptr = {
            unsafe extern "C" {
                fn objc_getClass(name: *const core::ffi::c_char) -> *const core::ffi::c_void;
            }
            objc_getClass(c"NSApplication".as_ptr())
        };
        if !ns_app_cls_ptr.is_null() {
            let ns_app = objc_msgSend(ns_app_cls_ptr, sel_shared);
            if !ns_app.is_null() {
                // NSApp instance class.
                inject(object_getClass(ns_app));

                // 4) NSApp delegate class.
                let app_delegate = objc_msgSend(ns_app, sel_delegate);
                if !app_delegate.is_null() {
                    inject(object_getClass(app_delegate));
                }
            }
        }

        // Log for debugging (visible in terminal output).
        eprintln!("[ferrum] newWindowForTab: installed on responder chain");
    }
}

/// Adds `new_window` as a native macOS tab to the `existing` window's tab group.
pub fn add_as_tab(existing: &Window, new_window: &Window) {
    let Some(existing_ns) = get_ns_window(existing) else {
        return;
    };
    let Some(new_ns) = get_ns_window(new_window) else {
        return;
    };
    // SAFETY: Both existing_ns and new_ns are valid, retained NSWindow objects obtained
    // from get_ns_window(). The msg_send! for "addTabbedWindow:ordered:" is safe because:
    // 1. The selector exists on NSWindow (macOS 10.12+)
    // 2. new_ns is dereferenced to pass the NSWindow object (not the Retained wrapper)
    // 3. The ordered: parameter is NSWindowOrderingMode::Above (1) which is a valid enum value
    // 4. The return type () matches the void return of the Objective-C method
    // makeKeyAndOrderFront is a standard NSWindow method that brings the window to front.
    unsafe {
        // NSWindowOrderingMode::Above = 1
        let _: () = msg_send![&existing_ns, addTabbedWindow: &*new_ns, ordered: 1i64];
        new_ns.makeKeyAndOrderFront(None);
    }
}

/// Selects a tab by index in the native macOS tab group.
/// If `index >= tab count`, selects the last tab.
pub fn select_tab(window: &Window, index: usize) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };
    // SAFETY: ns_window is a valid, retained NSWindow from get_ns_window().
    // - "tabbedWindows" is a valid NSWindow selector (macOS 10.12+) returning NSArray or nil
    // - "count" is a valid NSArray selector returning the number of elements
    // - "objectAtIndex:" is a valid NSArray selector; we ensure idx < count before calling
    // - makeKeyAndOrderFront is a standard NSWindow method
    // All return types match their Objective-C counterparts, and we handle nil (None) cases.
    unsafe {
        let tabbed: Option<Retained<AnyObject>> = msg_send![&ns_window, tabbedWindows];
        let Some(windows) = tabbed else { return };
        let count: usize = msg_send![&windows, count];
        if count == 0 {
            return;
        }
        let idx = index.min(count - 1);
        let target: Retained<NSWindow> = msg_send![&windows, objectAtIndex: idx];
        target.makeKeyAndOrderFront(None);
    }
}

/// Selects the next tab in the native macOS tab group.
pub fn select_next_tab(window: &Window) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };
    // SAFETY: ns_window is a valid, retained NSWindow from get_ns_window().
    // "selectNextTab:" is a valid NSWindow selector (macOS 10.12+) that accepts an optional
    // sender parameter (nil/null is valid). The return type () matches the void return.
    unsafe {
        let _: () = msg_send![&ns_window, selectNextTab: std::ptr::null::<AnyObject>()];
    }
}

/// Selects the previous tab in the native macOS tab group.
pub fn select_previous_tab(window: &Window) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };
    // SAFETY: ns_window is a valid, retained NSWindow from get_ns_window().
    // "selectPreviousTab:" is a valid NSWindow selector (macOS 10.12+) that accepts an
    // optional sender parameter (nil/null is valid). The return type () matches the void return.
    unsafe {
        let _: () = msg_send![&ns_window, selectPreviousTab: std::ptr::null::<AnyObject>()];
    }
}

/// Sync native macOS tab bar visibility:
/// hide when only one native tab remains, show for 2+ tabs.
pub fn sync_native_tab_bar_visibility(window: &Window) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };
    let tab_group = ns_window.tabGroup();
    let tab_count = tab_group
        .as_ref()
        .map(|group| group.windows().len())
        .or_else(|| ns_window.tabbedWindows().map(|windows| windows.len()))
        .unwrap_or(1);
    let should_be_visible = tab_count > 1;
    let is_visible = tab_group
        .as_ref()
        .is_some_and(|group| group.isTabBarVisible());

    // Match Ghostty-style behavior: rely on native toggle based on current state.
    if should_be_visible != is_visible {
        ns_window.toggleTabBar(None);
    }
}

/// Returns true and resets the flag if the native "+" button was clicked.
pub fn take_new_tab_request() -> bool {
    NEW_TAB_REQUESTED.swap(false, Ordering::SeqCst)
}

// =============================================================================
// Pin Button (Titlebar Accessory) Support
// =============================================================================

use std::collections::HashMap;
use std::sync::Mutex;

/// Flag: pin button was clicked, need to toggle pin state.
static PIN_BUTTON_CLICKED: AtomicBool = AtomicBool::new(false);

/// Map from NSWindow pointer to its toolbar item (for updating icon state).
/// We store raw addresses as usize to keep the static map `Send + Sync`.
static TOOLBAR_ITEMS: Mutex<Option<HashMap<usize, usize>>> = Mutex::new(None);

/// ObjC method implementation for pin button action.
///
/// # Safety
///
/// This function is designed to be called by the Objective-C runtime as a method
/// implementation. It must have the correct calling convention (extern "C") and
/// signature matching the Objective-C method type encoding "v@:@":
/// - `_this`: The receiver object (self in Objective-C)
/// - `_cmd`: The selector being invoked
/// - `_sender`: The sender object passed to the action method
///
/// The function only performs an atomic store operation, which is inherently safe.
unsafe extern "C" fn handle_pin_button_click(
    _this: *mut core::ffi::c_void,
    _cmd: *const core::ffi::c_void,
    _sender: *mut core::ffi::c_void,
) {
    PIN_BUTTON_CLICKED.store(true, Ordering::SeqCst);
}

/// Returns true and resets the flag if the pin button was clicked.
pub fn take_pin_button_request() -> bool {
    PIN_BUTTON_CLICKED.swap(false, Ordering::SeqCst)
}

fn window_and_group_ptrs(ns_window: &Retained<NSWindow>) -> std::collections::HashSet<usize> {
    let mut out = std::collections::HashSet::new();
    out.insert(Retained::as_ptr(ns_window) as usize);

    if let Some(group) = ns_window.tabGroup() {
        let windows = group.windows();
        for win in windows.iter() {
            out.insert(Retained::as_ptr(&win) as usize);
        }
    } else if let Some(tabbed) = ns_window.tabbedWindows() {
        for win in tabbed.iter() {
            out.insert(Retained::as_ptr(&win) as usize);
        }
    }

    out
}

fn set_pin_button_state_for_window_ptr(window_ptr: usize, pinned: bool) {
    let button_ptr = {
        let map = TOOLBAR_ITEMS.lock().unwrap();
        map.as_ref().and_then(|m| m.get(&window_ptr).copied())
    };
    let Some(button_ptr) = button_ptr else {
        return;
    };

    // SAFETY: button_ptr is captured from a live NSButton in setup_toolbar().
    unsafe {
        let button = button_ptr as *mut core::ffi::c_void;

        let symbol_name = if pinned {
            ns_string!("pin.fill")
        } else {
            ns_string!("pin")
        };
        let accessibility_desc = if pinned {
            ns_string!("Unpin Window")
        } else {
            ns_string!("Pin Window")
        };

        let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            symbol_name,
            Some(accessibility_desc),
        );

        if let Some(image) = image {
            let _: () = msg_send![button as *const AnyObject, setImage: &*image];
        }

        let tooltip = if pinned {
            ns_string!("Unpin window")
        } else {
            ns_string!("Pin window on top")
        };
        let _: () = msg_send![button as *const AnyObject, setToolTip: tooltip];
    }
}

pub fn is_window_pinned(window: &Window) -> bool {
    let Some(ns_window) = get_ns_window(window) else {
        return false;
    };
    ns_window.level() != NSNormalWindowLevel
}

pub fn set_native_tab_group_pin_state(window: &Window, pinned: bool) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };

    let level = if pinned {
        NSFloatingWindowLevel
    } else {
        NSNormalWindowLevel
    };

    let ptrs = window_and_group_ptrs(&ns_window);

    if let Some(group) = ns_window.tabGroup() {
        let windows = group.windows();
        for win in windows.iter() {
            win.setLevel(level);
        }
    } else if let Some(tabbed) = ns_window.tabbedWindows() {
        for win in tabbed.iter() {
            win.setLevel(level);
        }
        ns_window.setLevel(level);
    } else {
        ns_window.setLevel(level);
    }

    for ptr in ptrs {
        set_pin_button_state_for_window_ptr(ptr, pinned);
    }
}

/// Sets up a titlebar accessory pin button for the given window.
///
/// The button appears near the traffic lights and uses SF Symbols:
/// - "pin" for unpinned
/// - "pin.fill" for pinned
pub fn setup_toolbar(window: &Window) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };

    let Some(mtm_button) = MainThreadMarker::new() else {
        eprintln!("[ferrum] setup_toolbar() must run on main thread");
        return;
    };
    let Some(mtm_accessory) = MainThreadMarker::new() else {
        eprintln!("[ferrum] setup_toolbar() must run on main thread");
        return;
    };

    // SAFETY: We install a method on the NSWindow class and wire a button action to it.
    // All selectors and APIs used are standard AppKit interfaces.
    unsafe {
        // Install click handler on the window class.
        let sel_pin_action_ptr = sel_registerName(c"ferrumPinButtonClicked:".as_ptr());
        let sel_pin_action = Sel::register(c"ferrumPinButtonClicked:");
        let imp: unsafe extern "C" fn() =
            core::mem::transmute(handle_pin_button_click as unsafe extern "C" fn(_, _, _));
        let types = c"v@:@".as_ptr();

        let win_cls = object_getClass(Retained::as_ptr(&ns_window).cast());
        if !win_cls.is_null() {
            if !class_addMethod(win_cls, sel_pin_action_ptr, imp, types) {
                class_replaceMethod(win_cls, sel_pin_action_ptr, imp, types);
            }
        }

        let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            ns_string!("pin"),
            Some(ns_string!("Pin Window")),
        ) else {
            eprintln!("[ferrum] Failed to create pin icon");
            return;
        };

        let button = NSButton::buttonWithImage_target_action(&image, None, None, mtm_button);
        button.setBordered(false);
        button.setBezelStyle(NSBezelStyle::Toolbar);
        button.setToolTip(Some(ns_string!("Pin window on top")));

        let _: () = msg_send![&button, setTarget: &*ns_window];
        let _: () = msg_send![&button, setAction: sel_pin_action];

        let accessory_vc = NSTitlebarAccessoryViewController::new(mtm_accessory);
        accessory_vc.setView(&button);
        accessory_vc.setLayoutAttribute(NSLayoutAttribute::Leading);
        ns_window.addTitlebarAccessoryViewController(&accessory_vc);

        // Save the button pointer for icon updates.
        let window_ptr = Retained::as_ptr(&ns_window) as usize;
        {
            let mut map = TOOLBAR_ITEMS.lock().unwrap();
            let items = map.get_or_insert_with(HashMap::new);
            items.insert(window_ptr, Retained::as_ptr(&button) as usize);
        }

        eprintln!("[ferrum] Pin button accessory installed");
    }
}

/// Updates the pin button icon state (outline = unpinned, filled = pinned).
pub fn set_pin_button_state(window: &Window, pinned: bool) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };
    let window_ptr = Retained::as_ptr(&ns_window) as usize;
    set_pin_button_state_for_window_ptr(window_ptr, pinned);
}
