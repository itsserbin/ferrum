#[cfg(any(target_os = "macos", target_os = "windows"))]
use muda::ContextMenu;
use muda::{Menu, MenuId, MenuItem, PredefinedMenuItem};

/// Identifiers for context menu actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuAction {
    // Terminal context menu
    Copy,
    Paste,
    SelectAll,
    ClearSelection,
    SplitRight,
    SplitDown,
    SplitLeft,
    SplitUp,
    ClosePane,
    ClearTerminal,
    ResetTerminal,
    // Tab context menu
    RenameTab,
    DuplicateTab,
    CloseTab,
}

/// Builds the terminal area context menu.
/// `has_selection`: whether text is currently selected
/// `has_multiple_panes`: whether this tab has >1 pane
pub(super) fn build_terminal_context_menu(
    has_selection: bool,
    has_multiple_panes: bool,
) -> (Menu, Vec<(MenuId, MenuAction)>) {
    let menu = Menu::new();
    let mut action_map = Vec::new();

    let copy_item = MenuItem::new("Copy", has_selection, None);
    action_map.push((copy_item.id().clone(), MenuAction::Copy));
    let paste_item = MenuItem::new("Paste", true, None);
    action_map.push((paste_item.id().clone(), MenuAction::Paste));
    let select_all = MenuItem::new("Select All", true, None);
    action_map.push((select_all.id().clone(), MenuAction::SelectAll));
    let clear_sel = MenuItem::new("Clear Selection", has_selection, None);
    action_map.push((clear_sel.id().clone(), MenuAction::ClearSelection));

    let _ = menu.append_items(&[
        &copy_item,
        &paste_item,
        &select_all,
        &clear_sel,
        &PredefinedMenuItem::separator(),
    ]);

    let split_right = MenuItem::new("Split Right", true, None);
    action_map.push((split_right.id().clone(), MenuAction::SplitRight));
    let split_down = MenuItem::new("Split Down", true, None);
    action_map.push((split_down.id().clone(), MenuAction::SplitDown));
    let split_left = MenuItem::new("Split Left", true, None);
    action_map.push((split_left.id().clone(), MenuAction::SplitLeft));
    let split_up = MenuItem::new("Split Up", true, None);
    action_map.push((split_up.id().clone(), MenuAction::SplitUp));

    let _ = menu.append_items(&[
        &split_right,
        &split_down,
        &split_left,
        &split_up,
        &PredefinedMenuItem::separator(),
    ]);

    if has_multiple_panes {
        let close_pane = MenuItem::new("Close Pane", true, None);
        action_map.push((close_pane.id().clone(), MenuAction::ClosePane));
        let _ = menu.append_items(&[&close_pane, &PredefinedMenuItem::separator()]);
    }

    let clear_term = MenuItem::new("Clear Terminal", true, None);
    action_map.push((clear_term.id().clone(), MenuAction::ClearTerminal));
    let reset_term = MenuItem::new("Reset Terminal", true, None);
    action_map.push((reset_term.id().clone(), MenuAction::ResetTerminal));

    let _ = menu.append_items(&[&clear_term, &reset_term]);

    (menu, action_map)
}

/// Builds the tab bar context menu.
pub(super) fn build_tab_context_menu() -> (Menu, Vec<(MenuId, MenuAction)>) {
    let menu = Menu::new();
    let mut action_map = Vec::new();

    let rename = MenuItem::new("Rename", true, None);
    action_map.push((rename.id().clone(), MenuAction::RenameTab));
    let duplicate = MenuItem::new("Duplicate", true, None);
    action_map.push((duplicate.id().clone(), MenuAction::DuplicateTab));
    let close = MenuItem::new("Close", true, None);
    action_map.push((close.id().clone(), MenuAction::CloseTab));

    let _ = menu.append_items(&[
        &rename,
        &PredefinedMenuItem::separator(),
        &duplicate,
        &PredefinedMenuItem::separator(),
        &close,
    ]);

    (menu, action_map)
}

/// Shows a context menu natively for the given window.
pub(super) fn show_context_menu(
    window: &winit::window::Window,
    menu: &Menu,
    position: Option<muda::dpi::Position>,
) {
    #[cfg(target_os = "windows")]
    {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::Win32(win32) = handle.as_raw() {
                unsafe {
                    menu.show_context_menu_for_hwnd(win32.hwnd.get() as isize, position);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = window.window_handle()
            && let RawWindowHandle::AppKit(appkit) = handle.as_raw()
        {
            unsafe {
                menu.show_context_menu_for_nsview(
                    appkit.ns_view.as_ptr() as *const std::ffi::c_void,
                    position,
                );
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let _ = (window, menu, position);
        eprintln!("Native context menus not yet supported on Linux");
    }
}
