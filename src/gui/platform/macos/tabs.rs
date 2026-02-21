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
/// Must match Objective-C action ABI/signature (`v@:@`).
/// Parameters are intentionally unused; body only performs an atomic increment.
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
    // SAFETY: `ns_window` is valid; all selectors exist on supported macOS and signatures match.
    unsafe {
        ns_window.setTabbingMode(NSWindowTabbingMode::Preferred);
        let identifier = ns_string!("com.ferrum.terminal");
        let _: () = msg_send![&ns_window, setTabbingIdentifier: identifier];
    }

    // Diagnostic: print window/app properties.
    unsafe {
        let mask = ns_window.styleMask();
        let toolbar_style = ns_window.toolbarStyle();
        let title_vis = ns_window.titleVisibility();
        let transparent = ns_window.titlebarAppearsTransparent();
        let has_toolbar = ns_window.toolbar().is_some();
        let tabbing_mode = ns_window.tabbingMode();
        let full_size = mask.0 & (1 << 15) != 0;

        eprintln!("[ferrum-diag] styleMask: {:?}", mask);
        eprintln!("[ferrum-diag] toolbarStyle: {:?}", toolbar_style.0);
        eprintln!("[ferrum-diag] titleVisibility: {:?}", title_vis.0);
        eprintln!("[ferrum-diag] titlebarAppearsTransparent: {}", transparent);
        eprintln!("[ferrum-diag] hasToolbar: {}", has_toolbar);
        eprintln!("[ferrum-diag] tabbingMode: {:?}", tabbing_mode.0);
        eprintln!("[ferrum-diag] fullSizeContentView: {}", full_size);

        // NSApp-level diagnostics
        let sel_shared = sel_registerName(c"sharedApplication".as_ptr());
        let ns_app_cls = {
            unsafe extern "C" {
                fn objc_getClass(name: *const core::ffi::c_char) -> *const core::ffi::c_void;
            }
            unsafe { objc_getClass(c"NSApplication".as_ptr()) }
        };
        if !ns_app_cls.is_null() {
            let ns_app = unsafe { objc_msgSend(ns_app_cls, sel_shared) };
            if !ns_app.is_null() {
                let sel_policy = sel_registerName(c"activationPolicy".as_ptr());
                let policy: isize = unsafe { core::mem::transmute(objc_msgSend(ns_app, sel_policy)) };
                eprintln!("[ferrum-diag] activationPolicy: {}", policy);
            }
        }

        // Check if running from a bundle
        unsafe extern "C" {
            fn CFBundleGetMainBundle() -> *const core::ffi::c_void;
            fn CFBundleGetIdentifier(bundle: *const core::ffi::c_void) -> *const core::ffi::c_void;
        }
        let bundle = unsafe { CFBundleGetMainBundle() };
        let has_bundle_id = if bundle.is_null() {
            false
        } else {
            !unsafe { CFBundleGetIdentifier(bundle) }.is_null()
        };
        eprintln!("[ferrum-diag] hasMainBundle: {}", !bundle.is_null());
        eprintln!("[ferrum-diag] hasBundleIdentifier: {}", has_bundle_id);

        // Binary path
        eprintln!("[ferrum-diag] exe: {:?}", std::env::current_exe().ok());
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

    // SAFETY: selector/type encoding and IMP signature match `newWindowForTab:`.
    // Class/object pointers come from Objective-C runtime calls and are null-checked before injection.
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

        let win_cls = object_getClass(Retained::as_ptr(&ns_window).cast());
        inject(win_cls);

        let sel_delegate = sel_registerName(c"delegate".as_ptr());
        let win_delegate = objc_msgSend(Retained::as_ptr(&ns_window).cast(), sel_delegate);
        if !win_delegate.is_null() {
            inject(object_getClass(win_delegate));
        }

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
                inject(object_getClass(ns_app));

                let app_delegate = objc_msgSend(ns_app, sel_delegate);
                if !app_delegate.is_null() {
                    inject(object_getClass(app_delegate));
                }
            }
        }

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
    // SAFETY: both NSWindow handles are valid; selector and argument types match Objective-C API.
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
    // SAFETY: selectors and return types match, and index is bounds-checked before objectAtIndex.
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
    // SAFETY: valid NSWindow; selector exists and accepts nil sender.
    unsafe {
        let _: () = msg_send![&ns_window, selectNextTab: std::ptr::null::<AnyObject>()];
    }
}

/// Selects the previous tab in the native macOS tab group.
pub fn select_previous_tab(window: &Window) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };
    // SAFETY: valid NSWindow; selector exists and accepts nil sender.
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
