use std::sync::atomic::{AtomicBool, Ordering};

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::msg_send;
use objc2_app_kit::{NSView, NSWindow, NSWindowTabbingMode};
use objc2_foundation::ns_string;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

/// Flag: native "+" button was clicked, need to create a new tab.
static NEW_TAB_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Extracts the NSWindow from a winit Window via raw-window-handle.
fn get_ns_window(window: &Window) -> Option<Retained<NSWindow>> {
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return None;
    };
    unsafe {
        let ns_view: Retained<NSView> =
            Retained::retain(appkit.ns_view.as_ptr().cast())?;
        ns_view.window()
    }
}

/// Configures a window for native macOS tab grouping.
pub fn configure_native_tabs(window: &Window) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };
    unsafe {
        ns_window.setTabbingMode(NSWindowTabbingMode::Preferred);
        let identifier = ns_string!("com.ferrum.terminal");
        let _: () = msg_send![&ns_window, setTabbingIdentifier: identifier];
    }
}

/// Shows the native tab bar even with a single tab.
pub fn show_tab_bar(window: &Window) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };
    unsafe {
        // Try NSWindowTabGroup API (macOS 12+).
        let tab_group: Option<Retained<AnyObject>> = msg_send![&ns_window, tabGroup];
        if let Some(group) = tab_group {
            let visible: bool = msg_send![&group, isTabBarVisible];
            if !visible {
                let _: () = msg_send![&group, setTabBarVisible: true];
            }
            return;
        }
        // Fallback: toggleTabBar (macOS 10.12+).
        let _: () = msg_send![&ns_window, toggleTabBar: std::ptr::null::<AnyObject>()];
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
    unsafe {
        let tabbed: Option<Retained<AnyObject>> =
            msg_send![&ns_window, tabbedWindows];
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
    unsafe {
        let _: () = msg_send![&ns_window, selectNextTab: std::ptr::null::<AnyObject>()];
    }
}

/// Selects the previous tab in the native macOS tab group.
pub fn select_previous_tab(window: &Window) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };
    unsafe {
        let _: () = msg_send![&ns_window, selectPreviousTab: std::ptr::null::<AnyObject>()];
    }
}

/// Returns true and resets the flag if the native "+" button was clicked.
pub fn take_new_tab_request() -> bool {
    NEW_TAB_REQUESTED.swap(false, Ordering::SeqCst)
}

/// Registers the `FermWindowController` ObjC class (inherits NSWindowController).
/// This class responds to `newWindowForTab:` — required by macOS to show the "+" button.
fn ensure_controller_class() {
    static REGISTERED: AtomicBool = AtomicBool::new(false);
    if REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }

    // Raw ObjC runtime C functions — stable ABI.
    unsafe extern "C" {
        fn objc_allocateClassPair(
            superclass: *const core::ffi::c_void,
            name: *const core::ffi::c_char,
            extra_bytes: usize,
        ) -> *mut core::ffi::c_void;
        fn objc_registerClassPair(cls: *mut core::ffi::c_void);
        fn sel_registerName(name: *const core::ffi::c_char) -> *const core::ffi::c_void;
        fn class_addMethod(
            cls: *mut core::ffi::c_void,
            name: *const core::ffi::c_void,
            imp: unsafe extern "C" fn(),
            types: *const core::ffi::c_char,
        ) -> bool;
    }

    unsafe extern "C" fn handle_new_window_for_tab(
        _this: *mut core::ffi::c_void,
        _cmd: *const core::ffi::c_void,
        _sender: *mut core::ffi::c_void,
    ) {
        NEW_TAB_REQUESTED.store(true, Ordering::SeqCst);
    }

    unsafe {
        let Some(superclass) = AnyClass::get(c"NSWindowController") else {
            REGISTERED.store(false, Ordering::SeqCst);
            return;
        };
        let cls = objc_allocateClassPair(
            (superclass as *const AnyClass).cast(),
            c"FermWindowController".as_ptr(),
            0,
        );
        if cls.is_null() {
            REGISTERED.store(false, Ordering::SeqCst);
            return;
        }
        let sel = sel_registerName(c"newWindowForTab:".as_ptr());
        class_addMethod(
            cls,
            sel,
            core::mem::transmute(handle_new_window_for_tab as unsafe extern "C" fn(_, _, _)),
            c"v@:@".as_ptr(),
        );
        objc_registerClassPair(cls);
    }
}

/// Creates an NSWindowController for the given window.
/// The controller responds to `newWindowForTab:`, which makes the native "+" button appear.
/// The returned `Retained` MUST be kept alive for the lifetime of the window.
pub fn create_window_controller(window: &Window) -> Option<Retained<AnyObject>> {
    let ns_window = get_ns_window(window)?;
    ensure_controller_class();

    unsafe {
        let cls = AnyClass::get(c"FermWindowController")?;
        let alloc: Retained<AnyObject> = msg_send![cls, alloc];
        let controller: Retained<AnyObject> = msg_send![alloc, initWithWindow: &*ns_window];
        Some(controller)
    }
}
