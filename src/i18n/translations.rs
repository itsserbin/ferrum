/// All user-facing strings for the Ferrum UI.
///
/// Fields are grouped by area: context menu, close dialog, settings window,
/// settings tabs, security, security popup, macOS pin button, and update.
pub struct Translations {
    // --- Context menu ---
    pub menu_copy: &'static str,
    pub menu_paste: &'static str,
    pub menu_select_all: &'static str,
    pub menu_clear_selection: &'static str,
    pub menu_split_right: &'static str,
    pub menu_split_down: &'static str,
    pub menu_split_left: &'static str,
    pub menu_split_up: &'static str,
    pub menu_close_pane: &'static str,
    pub menu_clear_terminal: &'static str,
    pub menu_reset_terminal: &'static str,
    pub menu_rename: &'static str,
    pub menu_duplicate: &'static str,
    pub menu_close: &'static str,

    // --- Close dialog ---
    pub close_dialog_title: &'static str,
    pub close_dialog_body: &'static str,
    pub close_dialog_confirm: &'static str,
    pub close_dialog_cancel: &'static str,

    // --- Settings window ---
    pub settings_title: &'static str,
    pub settings_tab_font: &'static str,
    pub settings_tab_theme: &'static str,
    pub settings_tab_terminal: &'static str,
    pub settings_tab_layout: &'static str,
    pub settings_tab_security: &'static str,
    pub settings_reset_to_defaults: &'static str,

    // --- Font tab ---
    pub font_size_label: &'static str,
    pub font_family_label: &'static str,
    pub font_line_padding_label: &'static str,

    // --- Theme tab ---
    pub theme_label: &'static str,

    // --- Terminal tab ---
    pub terminal_language_label: &'static str,
    pub terminal_max_scrollback_label: &'static str,
    pub terminal_cursor_blink_label: &'static str,

    // --- Layout tab ---
    pub layout_window_padding_label: &'static str,
    pub layout_pane_padding_label: &'static str,
    pub layout_scrollbar_width_label: &'static str,
    pub layout_tab_bar_height_label: &'static str,

    // --- Security tab ---
    pub security_mode_label: &'static str,
    pub security_mode_disabled: &'static str,
    pub security_mode_standard: &'static str,
    pub security_mode_custom: &'static str,
    pub security_paste_protection_label: &'static str,
    pub security_paste_protection_desc: &'static str,
    pub security_block_title_query_label: &'static str,
    pub security_block_title_query_desc: &'static str,
    pub security_limit_cursor_jumps_label: &'static str,
    pub security_limit_cursor_jumps_desc: &'static str,
    pub security_clear_mouse_on_reset_label: &'static str,
    pub security_clear_mouse_on_reset_desc: &'static str,

    // --- Security popup ---
    pub security_popup_title: &'static str,
    pub security_event_paste_newlines: &'static str,
    pub security_event_title_query_blocked: &'static str,
    pub security_event_cursor_rewrite: &'static str,
    pub security_event_mouse_leak_prevented: &'static str,

    // --- macOS pin button ---
    pub macos_pin_window: &'static str,
    pub macos_unpin_window: &'static str,
    pub macos_pin_tooltip: &'static str,
    pub macos_unpin_tooltip: &'static str,
    pub macos_settings: &'static str,

    // --- Update ---
    /// Format string — use `{}` as placeholder for the version tag.
    pub update_available: &'static str,
}
