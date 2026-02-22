use super::translations::Translations;

static EN: Translations = Translations {
    // --- Context menu ---
    menu_copy: "Copy",
    menu_paste: "Paste",
    menu_select_all: "Select All",
    menu_clear_selection: "Clear Selection",
    menu_split_right: "Split Right",
    menu_split_down: "Split Down",
    menu_split_left: "Split Left",
    menu_split_up: "Split Up",
    menu_close_pane: "Close Pane",
    menu_clear_terminal: "Clear Terminal",
    menu_reset_terminal: "Reset Terminal",
    menu_rename: "Rename",
    menu_duplicate: "Duplicate",
    menu_close: "Close",

    // --- Close dialog ---
    close_dialog_title: "Close Ferrum?",
    close_dialog_body: "Closing this terminal window will stop all running processes in its tabs.",
    close_dialog_confirm: "Close",
    close_dialog_cancel: "Cancel",

    // --- Settings window ---
    settings_title: "Ferrum Settings",
    settings_tab_font: "Font",
    settings_tab_theme: "Theme",
    settings_tab_terminal: "Terminal",
    settings_tab_layout: "Layout",
    settings_tab_security: "Security",
    settings_reset_to_defaults: "Reset to Defaults",

    // --- Font tab ---
    font_size_label: "Font Size:",
    font_family_label: "Font Family:",
    font_line_padding_label: "Line Padding:",

    // --- Theme tab ---
    theme_label: "Theme:",

    // --- Terminal tab ---
    terminal_language_label: "Language:",
    terminal_max_scrollback_label: "Max Scrollback:",
    terminal_cursor_blink_label: "Cursor Blink (ms):",

    // --- Layout tab ---
    layout_window_padding_label: "Window Padding:",
    layout_pane_padding_label: "Pane Padding:",
    layout_scrollbar_width_label: "Scrollbar Width:",
    layout_tab_bar_height_label: "Tab Bar Height:",

    // --- Security tab ---
    security_mode_label: "Security Mode:",
    security_mode_disabled: "Disabled",
    security_mode_standard: "Standard",
    security_mode_custom: "Custom",
    security_paste_protection_label: "Paste Protection",
    security_paste_protection_desc: "Warn before pasting text with suspicious control characters",
    security_block_title_query_label: "Block Title Query",
    security_block_title_query_desc: "Block programs from reading the terminal window title",
    security_limit_cursor_jumps_label: "Limit Cursor Jumps",
    security_limit_cursor_jumps_desc: "Restrict how far escape sequences can move the cursor",
    security_clear_mouse_on_reset_label: "Clear Mouse on Reset",
    security_clear_mouse_on_reset_desc: "Disable mouse tracking modes when the terminal resets",

    // --- Security popup ---
    security_event_paste_newlines: "Paste with newlines detected",
    security_event_title_query_blocked: "OSC/CSI title query blocked",
    security_event_cursor_rewrite: "Cursor rewrite detected",
    security_event_mouse_leak_prevented: "Mouse reporting leak prevented",

    // --- macOS pin button ---
    macos_pin_window: "Pin Window",
    macos_unpin_window: "Unpin Window",
    macos_pin_tooltip: "Pin window on top",
    macos_unpin_tooltip: "Unpin window",
    macos_settings: "Settings",

    // --- Update ---
    update_available: "Update {} available",
    update_details: "Details",
    update_install: "Install",
    update_installing: "Installing…",
    settings_tab_updates: "Updates",
    update_current_version: "Current version",
    update_check_now: "Check for Updates",
    update_auto_check: "Auto-check for updates",
};

pub fn translations() -> &'static Translations {
    &EN
}
