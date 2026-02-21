use crate::config::AppConfig;
use crate::gui::*;

impl FerrumWindow {
    /// Applies config changes from the settings overlay to the renderer and terminals.
    ///
    /// Called after every slider/enum interaction for live preview.
    pub(crate) fn apply_config_change(&mut self, config: &AppConfig) {
        // Snapshot old palette for recoloring.
        let old_fg = self.backend.palette().default_fg;
        let old_bg = self.backend.palette().default_bg;
        let old_ansi = self.backend.palette().ansi;

        // Apply to renderer (font, metrics, palette).
        self.backend.apply_config(config);

        // Resolve new palette for comparison.
        let new_palette = config.theme.resolve();
        let theme_changed = old_fg != new_palette.default_fg || old_bg != new_palette.default_bg;

        // Recolor terminal cells if theme changed.
        if theme_changed {
            for tab in &mut self.tabs {
                tab.pane_tree.for_each_leaf_mut(&mut |leaf| {
                    leaf.terminal.recolor(
                        old_fg,
                        old_bg,
                        &old_ansi,
                        new_palette.default_fg,
                        new_palette.default_bg,
                        &new_palette.ansi,
                    );
                });
            }
        }

        // Update cursor blink interval.
        self.cursor_blink_interval_ms = config.terminal.cursor_blink_interval_ms;

        // Update terminal max scrollback and security config.
        let sec = config.security.to_runtime();
        for tab in &mut self.tabs {
            tab.pane_tree.for_each_leaf_mut(&mut |leaf| {
                leaf.terminal.max_scrollback = config.terminal.max_scrollback;
                leaf.terminal.security_config = sec;
                leaf.security.config = sec;
            });
        }
    }

    /// Closes the settings overlay, persisting the config and signaling update.
    pub(in crate::gui) fn close_settings_overlay(&mut self) {
        if let Some(overlay) = self.settings_overlay.take() {
            crate::config::save_config(&overlay.editing_config);
            self.pending_config = Some(overlay.editing_config);
        }
        self.window.request_redraw();
    }
}
