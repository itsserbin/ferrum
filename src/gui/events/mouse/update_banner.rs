use winit::event::ElementState;

use crate::gui::renderer::shared::banner_layout::compute_update_banner_layout;
use crate::gui::renderer::shared::tab_math::TabLayoutMetrics;
use crate::gui::state::UpdateInstallState;
use crate::gui::*;
use crate::update::AvailableRelease;

impl FerrumWindow {
    /// Handles a left-button release on the update banner.
    ///
    /// Returns `true` if the click was consumed by the banner.
    pub(super) fn handle_update_banner_click(
        &mut self,
        state: ElementState,
        mx: f64,
        my: f64,
        available_release: Option<&AvailableRelease>,
    ) -> bool {
        if state != ElementState::Released {
            return false;
        }
        let Some(release) = available_release else {
            return false;
        };
        if self.update_banner_dismissed {
            return false;
        }
        if self.update_install_state != UpdateInstallState::Idle {
            return false;
        }

        let size = self.window.inner_size();
        let tab_bar_h = self.backend.tab_bar_height_px();
        let m = TabLayoutMetrics {
            cell_width: self.backend.cell_width(),
            cell_height: self.backend.cell_height(),
            ui_scale: self.backend.ui_scale(),
            tab_bar_height: tab_bar_h,
        };
        let Some(layout) = compute_update_banner_layout(
            &release.tag_name,
            &m,
            size.width,
            size.height,
            tab_bar_h,
        ) else {
            return false;
        };

        let px = mx as i32;
        let py = my as i32;

        // Check if click is within the banner background at all.
        let (bg_x, bg_y, bg_w, bg_h) = layout.bg_rect();
        let in_banner = px >= bg_x
            && px < bg_x + bg_w as i32
            && py >= bg_y
            && py < bg_y + bg_h as i32;
        if !in_banner {
            return false;
        }

        // Dismiss button
        let (dsx, dsy, dsw, dsh) = layout.dismiss_rect();
        if px >= dsx as i32
            && px < (dsx + dsw) as i32
            && py >= dsy as i32
            && py < (dsy + dsh) as i32
        {
            self.update_banner_dismissed = true;
            self.window.request_redraw();
            return true;
        }

        // Details button
        let (dtx, dty, dtw, dth) = layout.details_rect();
        if px >= dtx as i32
            && px < (dtx + dtw) as i32
            && py >= dty as i32
            && py < (dty + dth) as i32
        {
            open_url(&release.html_url);
            return true;
        }

        // Install button
        let (ix, iy, iw, ih) = layout.install_rect();
        if px >= ix as i32
            && px < (ix + iw) as i32
            && py >= iy as i32
            && py < (iy + ih) as i32
        {
            self.start_update_install(release);
            return true;
        }

        // Clicked banner background but not a button — consume to prevent pass-through.
        true
    }

    /// Starts the update install process (actual installer spawned in Task 9).
    pub(super) fn start_update_install(&mut self, release: &AvailableRelease) {
        self.update_install_state = UpdateInstallState::Installing;
        self.window.request_redraw();
        crate::update_installer::spawn_installer(&release.tag_name);
    }
}

/// Opens a URL in the system default browser.
fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", url])
        .spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}
