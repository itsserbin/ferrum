use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use objc2::MainThreadMarker;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2_app_kit::{
    NSBezelStyle, NSButton, NSFloatingWindowLevel, NSImage, NSLayoutAttribute, NSNormalWindowLevel,
    NSTitlebarAccessoryViewController, NSWindow,
};
use objc2_foundation::ns_string;
use winit::window::Window;

use super::ffi::*;
use super::get_ns_window;

/// Flag: pin button was clicked, need to toggle pin state.
static PIN_BUTTON_CLICKED: AtomicUsize = AtomicUsize::new(0);

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
    PIN_BUTTON_CLICKED.fetch_add(1, Ordering::SeqCst);
}

/// Returns and resets the pin-button click counter.
pub fn take_pin_button_requests() -> usize {
    PIN_BUTTON_CLICKED.swap(0, Ordering::SeqCst)
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
        if !win_cls.is_null() && !class_addMethod(win_cls, sel_pin_action_ptr, imp, types) {
            class_replaceMethod(win_cls, sel_pin_action_ptr, imp, types);
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

/// Removes toolbar bookkeeping for a window being destroyed.
pub fn remove_toolbar_item(window: &Window) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };
    let window_ptr = Retained::as_ptr(&ns_window) as usize;
    let mut map = TOOLBAR_ITEMS.lock().unwrap();
    let Some(items) = map.as_mut() else {
        return;
    };
    items.remove(&window_ptr);
    if items.is_empty() {
        *map = None;
    }
}
