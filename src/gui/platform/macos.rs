use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{msg_send, msg_send_id};
use objc2_app_kit::{NSView, NSWindow, NSWindowTabbingMode};
use objc2_foundation::ns_string;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

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
        let _: () = msg_send![&existing_ns, addTabbedWindow: &*new_ns ordered: 1i64];
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
            msg_send_id![&ns_window, tabbedWindows];
        let Some(windows) = tabbed else { return };
        let count: usize = msg_send![&windows, count];
        if count == 0 {
            return;
        }
        let idx = index.min(count - 1);
        let target: Retained<NSWindow> = msg_send_id![&windows, objectAtIndex: idx];
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
