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
    /// "Details" button label on the update banner.
    pub update_details: &'static str,
    /// "Install" button label on the update banner.
    pub update_install: &'static str,
    /// "Installing…" label shown while update is in progress.
    pub update_installing: &'static str,
    /// Settings tab label for Updates.
    pub settings_tab_updates: &'static str,
    /// "Current version" label in Updates settings tab.
    pub update_current_version: &'static str,
    /// "Check for Updates" button label in Updates settings tab.
    pub update_check_now: &'static str,
    /// "Auto-check for updates" toggle label.
    pub update_auto_check: &'static str,
    /// Status shown while the manual update check is running.
    pub update_checking: &'static str,
    /// Status shown when the manual check found no newer version.
    pub update_up_to_date: &'static str,
}

impl Translations {
    /// Returns `true` when every field contains a non-empty string.
    ///
    /// This also ensures platform-specific fields (macOS pin button labels,
    /// tab-bar height label) are referenced on all targets, preventing
    /// dead-code warnings.
    pub fn all_non_empty(&self) -> bool {
        let fields: &[&str] = &[
            self.menu_copy,
            self.menu_paste,
            self.menu_select_all,
            self.menu_clear_selection,
            self.menu_split_right,
            self.menu_split_down,
            self.menu_split_left,
            self.menu_split_up,
            self.menu_close_pane,
            self.menu_clear_terminal,
            self.menu_reset_terminal,
            self.menu_rename,
            self.menu_duplicate,
            self.menu_close,
            self.close_dialog_title,
            self.close_dialog_body,
            self.close_dialog_confirm,
            self.close_dialog_cancel,
            self.settings_title,
            self.settings_tab_font,
            self.settings_tab_theme,
            self.settings_tab_terminal,
            self.settings_tab_layout,
            self.settings_tab_security,
            self.settings_reset_to_defaults,
            self.font_size_label,
            self.font_family_label,
            self.font_line_padding_label,
            self.theme_label,
            self.terminal_language_label,
            self.terminal_max_scrollback_label,
            self.terminal_cursor_blink_label,
            self.layout_window_padding_label,
            self.layout_pane_padding_label,
            self.layout_scrollbar_width_label,
            self.layout_tab_bar_height_label,
            self.security_mode_label,
            self.security_mode_disabled,
            self.security_mode_standard,
            self.security_mode_custom,
            self.security_paste_protection_label,
            self.security_paste_protection_desc,
            self.security_block_title_query_label,
            self.security_block_title_query_desc,
            self.security_limit_cursor_jumps_label,
            self.security_limit_cursor_jumps_desc,
            self.security_clear_mouse_on_reset_label,
            self.security_clear_mouse_on_reset_desc,
            self.security_event_paste_newlines,
            self.security_event_title_query_blocked,
            self.security_event_cursor_rewrite,
            self.security_event_mouse_leak_prevented,
            self.macos_pin_window,
            self.macos_unpin_window,
            self.macos_pin_tooltip,
            self.macos_unpin_tooltip,
            self.macos_settings,
            self.update_available,
            self.update_details,
            self.update_install,
            self.update_installing,
            self.settings_tab_updates,
            self.update_current_version,
            self.update_check_now,
            self.update_auto_check,
            self.update_checking,
            self.update_up_to_date,
        ];
        fields.iter().all(|s| !s.is_empty())
    }
}
