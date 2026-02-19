mod ffi;
mod pin;
mod tabs;

use objc2::rc::Retained;
use objc2_app_kit::{NSView, NSWindow};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

// Re-export all public functions from submodules.
pub use pin::{
    is_window_pinned, remove_toolbar_item, set_native_tab_group_pin_state, set_pin_button_state,
    setup_toolbar, take_pin_button_requests,
};
pub use tabs::{
    add_as_tab, configure_native_tabs, install_new_tab_handler, select_next_tab,
    select_previous_tab, select_tab, sync_native_tab_bar_visibility, take_new_tab_requests,
};

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
