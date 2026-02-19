use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSWindow, NSWindowTabbingMode};
use objc2_foundation::ns_string;
use winit::window::Window;

use super::ffi::*;
use super::get_ns_window;

/// Flag: native "+" button was clicked, need to create a new tab.
static NEW_TAB_REQUESTED: AtomicUsize = AtomicUsize::new(0);

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
    NEW_TAB_REQUESTED.fetch_add(1, Ordering::SeqCst);
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

/// Returns and resets the native "+" click counter.
pub fn take_new_tab_requests() -> usize {
    NEW_TAB_REQUESTED.swap(0, Ordering::SeqCst)
}
