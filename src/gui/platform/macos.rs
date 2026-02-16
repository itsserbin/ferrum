use std::sync::atomic::{AtomicBool, Ordering};

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol};
use objc2::sel;
use objc2_app_kit::{NSView, NSWindow, NSWindowTabbingMode};
use objc2_foundation::ns_string;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

/// Flag: native "+" button was clicked, need to create a new tab.
static NEW_TAB_REQUESTED: AtomicBool = AtomicBool::new(false);

// Raw ObjC runtime C functions — stable ABI, avoids objc2 version conflicts.
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

/// Sync native macOS tab bar visibility:
/// hide when only one native tab remains, show for 2+ tabs.
pub fn sync_native_tab_bar_visibility(window: &Window) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };
    // Avoid runtime selector-encoding mismatch here: use typed Sel for respondsToSelector.
    if !ns_window.respondsToSelector(sel!(setTabBarVisible:)) {
        return;
    }

    let tab_count = unsafe {
        // Use tabGroup.windows when possible because it tracks group membership
        // independently of whether the tab bar is currently visible.
        let tab_group: Option<Retained<AnyObject>> = msg_send![&ns_window, tabGroup];
        if let Some(group) = tab_group {
            let windows: Option<Retained<AnyObject>> = msg_send![&group, windows];
            windows
                .as_ref()
                .map(|window_list| {
                    let count: usize = msg_send![window_list, count];
                    count
                })
                .unwrap_or(1)
        } else {
            ns_window
                .tabbedWindows()
                .map(|windows| windows.len())
                .unwrap_or(1)
        }
    };
    let visible = tab_count > 1;

    unsafe {
        let _: () = msg_send![&ns_window, setTabBarVisible: visible];
    }
}

/// Returns true and resets the flag if the native "+" button was clicked.
pub fn take_new_tab_request() -> bool {
    NEW_TAB_REQUESTED.swap(false, Ordering::SeqCst)
}
