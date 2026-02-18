use std::sync::atomic::{AtomicBool, Ordering};

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSView, NSWindow, NSWindowTabbingMode};
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
// Pin Button (NSToolbar) Support
// =============================================================================

use std::collections::HashMap;
use std::sync::Mutex;

/// Flag: pin button was clicked, need to toggle pin state.
static PIN_BUTTON_CLICKED: AtomicBool = AtomicBool::new(false);

/// Map from NSWindow pointer to its toolbar item (for updating icon state).
/// We store the raw pointer to the NSToolbarItem so we can update it later.
static TOOLBAR_ITEMS: Mutex<Option<HashMap<usize, *mut core::ffi::c_void>>> = Mutex::new(None);

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

/// Sets up an NSToolbar with a pin button for the given window.
///
/// The toolbar appears right after the traffic lights (red/yellow/green buttons).
/// The pin button uses SF Symbols "pin" (outline) and "pin.fill" (filled) icons.
pub fn setup_toolbar(window: &Window) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };

    // SAFETY: This block creates an NSToolbar with a pin button item.
    // All operations use valid Objective-C selectors and proper memory management.
    //
    // 1. objc_getClass: Gets class objects for NSToolbar, NSToolbarItem, NSImage
    // 2. objc_msgSend with alloc/init: Standard Objective-C object creation pattern
    // 3. All selectors used are documented AppKit APIs available on macOS 10.13+
    // 4. The toolbar is retained by the window when set via setToolbar:
    // 5. We store the toolbar item pointer for later icon updates
    unsafe {
        // Get class pointers
        unsafe extern "C" {
            fn objc_getClass(name: *const core::ffi::c_char) -> *const core::ffi::c_void;
        }

        let toolbar_cls = objc_getClass(c"NSToolbar".as_ptr());
        let toolbar_item_cls = objc_getClass(c"NSToolbarItem".as_ptr());
        let image_cls = objc_getClass(c"NSImage".as_ptr());

        if toolbar_cls.is_null() || toolbar_item_cls.is_null() || image_cls.is_null() {
            eprintln!("[ferrum] Failed to get NSToolbar/NSToolbarItem/NSImage classes");
            return;
        }

        // Create toolbar identifier
        let toolbar_id = ns_string!("com.ferrum.toolbar");
        let pin_item_id = ns_string!("com.ferrum.toolbar.pin");

        // Selectors
        let sel_alloc = sel_registerName(c"alloc".as_ptr());
        let sel_init_with_id = sel_registerName(c"initWithIdentifier:".as_ptr());
        let sel_set_delegate = sel_registerName(c"setDelegate:".as_ptr());
        let sel_set_toolbar = sel_registerName(c"setToolbar:".as_ptr());
        let sel_set_image = sel_registerName(c"setImage:".as_ptr());
        let sel_set_label = sel_registerName(c"setLabel:".as_ptr());
        let sel_set_target = sel_registerName(c"setTarget:".as_ptr());
        let sel_set_action = sel_registerName(c"setAction:".as_ptr());
        let sel_image_with_system_symbol =
            sel_registerName(c"imageWithSystemSymbolName:accessibilityDescription:".as_ptr());

        // Create NSToolbar
        let toolbar_alloc = objc_msgSend(toolbar_cls, sel_alloc);
        if toolbar_alloc.is_null() {
            return;
        }
        let toolbar = objc_msgSend(toolbar_alloc, sel_init_with_id, toolbar_id);
        if toolbar.is_null() {
            return;
        }

        // Create NSToolbarItem for the pin button
        let item_alloc = objc_msgSend(toolbar_item_cls, sel_alloc);
        if item_alloc.is_null() {
            return;
        }
        let item = objc_msgSend(item_alloc, sel_init_with_id, pin_item_id);
        if item.is_null() {
            return;
        }

        // Set up SF Symbol image for the pin (outline version = unpinned)
        let symbol_name = ns_string!("pin");
        let accessibility_desc = ns_string!("Pin Window");
        let image = objc_msgSend(
            image_cls,
            sel_image_with_system_symbol,
            symbol_name,
            accessibility_desc,
        );

        if !image.is_null() {
            let _: () = msg_send![item as *const AnyObject, setImage: image];
        }

        // Set label and tooltip
        let label = ns_string!("Pin");
        let _: () = msg_send![item as *const AnyObject, setLabel: label];
        let _: () = msg_send![item as *const AnyObject, setToolTip: ns_string!("Pin window on top")];

        // Install the click handler on the window delegate (or window class)
        let sel_pin_action = sel_registerName(c"ferrumPinButtonClicked:".as_ptr());
        let imp: unsafe extern "C" fn() =
            core::mem::transmute(handle_pin_button_click as unsafe extern "C" fn(_, _, _));
        let types = c"v@:@".as_ptr();

        // Add method to window's class for the action
        let win_cls = object_getClass(Retained::as_ptr(&ns_window).cast());
        if !win_cls.is_null() {
            class_addMethod(win_cls, sel_pin_action, imp, types);
        }

        // Set target and action for the toolbar item
        let _: () = msg_send![item as *const AnyObject, setTarget: &*ns_window];
        let _: () = msg_send![item as *const AnyObject, setAction: sel_pin_action];

        // Store the item pointer for later updates
        let window_ptr = Retained::as_ptr(&ns_window) as usize;
        {
            let mut map = TOOLBAR_ITEMS.lock().unwrap();
            let items = map.get_or_insert_with(HashMap::new);
            items.insert(window_ptr, item);
        }

        // Create a simple toolbar delegate that returns our item
        // We'll use the window as the delegate and add the necessary methods
        let sel_toolbar_items = sel_registerName(c"toolbarAllowedItemIdentifiers:".as_ptr());
        let sel_toolbar_default = sel_registerName(c"toolbarDefaultItemIdentifiers:".as_ptr());
        let sel_toolbar_item_for_id =
            sel_registerName(c"toolbar:itemForItemIdentifier:willBeInsertedIntoToolbar:".as_ptr());

        // We need to implement delegate methods. Since this is complex, let's use
        // a simpler approach: directly insert the item into the toolbar.
        // NSToolbar has insertItemWithItemIdentifier:atIndex: but it requires the delegate
        // to provide items. Instead, let's use the view-based toolbar item approach.

        // Alternative: Create a button-based toolbar item directly
        let button_cls = objc_getClass(c"NSButton".as_ptr());
        if !button_cls.is_null() {
            let sel_button_with_image =
                sel_registerName(c"buttonWithImage:target:action:".as_ptr());

            // Create button with SF Symbol
            let button = objc_msgSend(
                button_cls,
                sel_button_with_image,
                image,
                Retained::as_ptr(&ns_window),
                sel_pin_action,
            );

            if !button.is_null() {
                // Set the button as the view for the toolbar item
                let _: () = msg_send![item as *const AnyObject, setView: button];

                // Set bordered style for the button
                let _: () = msg_send![button as *const AnyObject, setBordered: false];
                let _: () = msg_send![button as *const AnyObject, setBezelStyle: 0i64]; // NSBezelStyleRegularSquare
            }
        }

        // Configure toolbar display mode and size
        let sel_set_display_mode = sel_registerName(c"setDisplayMode:".as_ptr());
        let sel_set_size_mode = sel_registerName(c"setSizeMode:".as_ptr());
        let _: () = msg_send![toolbar as *const AnyObject, setDisplayMode: 1i64]; // NSToolbarDisplayModeIconOnly
        let _: () = msg_send![toolbar as *const AnyObject, setSizeMode: 1i64]; // NSToolbarSizeModeSmall

        // For a toolbar without delegate, we need to use a different approach.
        // Let's add the titlebar accessory view instead, which is simpler and
        // positions the button near the traffic lights.

        // Create NSTitlebarAccessoryViewController
        let accessory_vc_cls = objc_getClass(c"NSTitlebarAccessoryViewController".as_ptr());
        if accessory_vc_cls.is_null() {
            eprintln!("[ferrum] Failed to get NSTitlebarAccessoryViewController class");
            return;
        }

        let sel_init = sel_registerName(c"init".as_ptr());
        let sel_set_view = sel_registerName(c"setView:".as_ptr());
        let sel_set_layout_attribute = sel_registerName(c"setLayoutAttribute:".as_ptr());
        let sel_add_accessory = sel_registerName(c"addTitlebarAccessoryViewController:".as_ptr());

        let accessory_vc_alloc = objc_msgSend(accessory_vc_cls, sel_alloc);
        if accessory_vc_alloc.is_null() {
            return;
        }
        let accessory_vc = objc_msgSend(accessory_vc_alloc, sel_init);
        if accessory_vc.is_null() {
            return;
        }

        // Create button for the accessory
        if !button_cls.is_null() && !image.is_null() {
            let sel_button_with_image =
                sel_registerName(c"buttonWithImage:target:action:".as_ptr());

            let button = objc_msgSend(
                button_cls,
                sel_button_with_image,
                image,
                Retained::as_ptr(&ns_window),
                sel_pin_action,
            );

            if !button.is_null() {
                // Style the button
                let _: () = msg_send![button as *const AnyObject, setBordered: false];
                let _: () = msg_send![button as *const AnyObject, setBezelStyle: 0i64];

                // Set frame size for the button
                let sel_set_frame = sel_registerName(c"setFrame:".as_ptr());
                // CGRect: origin (x, y), size (width, height) - using 24x24 button
                #[repr(C)]
                struct CGRect {
                    x: f64,
                    y: f64,
                    width: f64,
                    height: f64,
                }
                let frame = CGRect {
                    x: 0.0,
                    y: 0.0,
                    width: 28.0,
                    height: 28.0,
                };
                let _: () = msg_send![button as *const AnyObject, setFrame: frame];

                // Store button reference for later icon updates
                {
                    let mut map = TOOLBAR_ITEMS.lock().unwrap();
                    let items = map.get_or_insert_with(HashMap::new);
                    items.insert(window_ptr, button);
                }

                // Set the button as the view for the accessory
                let _: () = msg_send![accessory_vc as *const AnyObject, setView: button];

                // Set layout attribute to position after traffic lights (left side)
                // NSLayoutAttributeLeft = 1, NSLayoutAttributeRight = 2
                // NSLayoutAttributeLeading = 5 positions after traffic lights
                let _: () = msg_send![accessory_vc as *const AnyObject, setLayoutAttribute: 5i64];

                // Add accessory to window
                let _: () = msg_send![&ns_window, addTitlebarAccessoryViewController: accessory_vc];

                eprintln!("[ferrum] Pin button toolbar installed");
            }
        }
    }
}

/// Updates the pin button icon state (outline = unpinned, filled = pinned).
pub fn set_pin_button_state(window: &Window, pinned: bool) {
    let Some(ns_window) = get_ns_window(window) else {
        return;
    };

    let window_ptr = Retained::as_ptr(&ns_window) as usize;
    let button_ptr = {
        let map = TOOLBAR_ITEMS.lock().unwrap();
        map.as_ref().and_then(|m| m.get(&window_ptr).copied())
    };

    let Some(button) = button_ptr else {
        return;
    };

    // SAFETY: button is a valid NSButton pointer stored during setup_toolbar().
    // We update its image to reflect the pinned state.
    unsafe {
        unsafe extern "C" {
            fn objc_getClass(name: *const core::ffi::c_char) -> *const core::ffi::c_void;
        }

        let image_cls = objc_getClass(c"NSImage".as_ptr());
        if image_cls.is_null() {
            return;
        }

        let sel_image_with_system_symbol =
            sel_registerName(c"imageWithSystemSymbolName:accessibilityDescription:".as_ptr());

        // Use "pin.fill" for pinned state, "pin" for unpinned
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

        let image = objc_msgSend(
            image_cls,
            sel_image_with_system_symbol,
            symbol_name,
            accessibility_desc,
        );

        if !image.is_null() {
            let _: () = msg_send![button as *const AnyObject, setImage: image];
        }

        // Update tooltip
        let tooltip = if pinned {
            ns_string!("Unpin window")
        } else {
            ns_string!("Pin window on top")
        };
        let _: () = msg_send![button as *const AnyObject, setToolTip: tooltip];
    }
}
