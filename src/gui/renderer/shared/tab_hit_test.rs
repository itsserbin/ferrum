//! Shared hit-testing logic for the tab bar.
//!
//! Pure functions that determine what element is under a given point.
//! Used by both CPU and GPU renderers to avoid duplicating hit-test logic.

#[cfg(not(target_os = "macos"))]
use super::super::TabInfo;
use super::super::TabBarHit;
use super::tab_math::{self, TabLayoutMetrics};

#[cfg(not(target_os = "macos"))]
use super::super::WindowButton;

/// Hit-tests the tab bar and returns the clicked target.
pub fn hit_test_tab_bar(
    x: f64,
    y: f64,
    tab_count: usize,
    buf_width: u32,
    m: &TabLayoutMetrics,
) -> TabBarHit {
    if y >= m.tab_bar_height as f64 {
        return TabBarHit::Empty;
    }

    #[cfg(not(target_os = "macos"))]
    if let Some(btn) = window_button_at_position(x, y, buf_width, m) {
        return TabBarHit::WindowButton(btn);
    }

    #[cfg(not(target_os = "macos"))]
    {
        let pin_rect = tab_math::pin_button_rect(m).to_tuple();
        if tab_math::point_in_rect(x, y, pin_rect) {
            return TabBarHit::PinButton;
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let gear_rect = tab_math::gear_button_rect(m);
        if tab_math::point_in_rect(x, y, gear_rect.to_tuple()) {
            return TabBarHit::SettingsButton;
        }
    }

    let tw = tab_math::calculate_tab_width(m, tab_count, buf_width);
    let tab_strip_start = tab_math::tab_strip_start_x(m);

    let plus_rect = tab_math::plus_button_rect(m, tab_count, tw).to_tuple();
    if tab_math::point_in_rect(x, y, plus_rect) {
        return TabBarHit::NewTab;
    }

    if x < tab_strip_start as f64 {
        return TabBarHit::Empty;
    }

    let rel_x = x as u32 - tab_strip_start;
    let tab_index = rel_x / tw;
    if (tab_index as usize) < tab_count {
        let idx = tab_index as usize;
        let close_rect = tab_math::close_button_rect(m, idx, tw).to_tuple();
        if tab_math::point_in_rect(x, y, close_rect) {
            return TabBarHit::CloseTab(idx);
        }
        return TabBarHit::Tab(idx);
    }

    TabBarHit::Empty
}

/// Hit-tests tab hover target (without button checks).
pub fn hit_test_tab_hover(
    x: f64,
    y: f64,
    tab_count: usize,
    buf_width: u32,
    m: &TabLayoutMetrics,
) -> Option<usize> {
    if y >= m.tab_bar_height as f64 || tab_count == 0 {
        return None;
    }
    let tw = tab_math::calculate_tab_width(m, tab_count, buf_width);
    let tab_strip_start = tab_math::tab_strip_start_x(m);
    if x < tab_strip_start as f64 {
        return None;
    }
    let rel_x = x as u32 - tab_strip_start;
    let idx = rel_x / tw;
    if (idx as usize) < tab_count {
        Some(idx as usize)
    } else {
        None
    }
}


/// Hit-test window control buttons (non-macOS only).
#[cfg(not(target_os = "macos"))]
pub fn window_button_at_position(
    x: f64,
    y: f64,
    buf_width: u32,
    m: &TabLayoutMetrics,
) -> Option<WindowButton> {
    if y >= m.tab_bar_height as f64 {
        return None;
    }
    let btn_w = m.scaled_px(tab_math::WIN_BTN_WIDTH);
    let close_x = buf_width.saturating_sub(btn_w);
    let max_x = buf_width.saturating_sub(btn_w * 2);
    let minimize_x = buf_width.saturating_sub(btn_w * 3);

    if x >= close_x as f64 && x < buf_width as f64 {
        Some(WindowButton::Close)
    } else if x >= max_x as f64 && x < (max_x + btn_w) as f64 {
        Some(WindowButton::Maximize)
    } else if x >= minimize_x as f64 && x < (minimize_x + btn_w) as f64 {
        Some(WindowButton::Minimize)
    } else {
        None
    }
}

#[cfg(not(target_os = "macos"))]
/// Returns full tab title when hover should show a tooltip.
pub fn tab_hover_tooltip<'a>(
    tabs: &'a [TabInfo<'a>],
    hovered_tab: Option<usize>,
    buf_width: u32,
    m: &TabLayoutMetrics,
) -> Option<&'a str> {
    let idx = hovered_tab?;
    let tab = tabs.get(idx)?;
    if tab.is_renaming || tab.title.is_empty() {
        return None;
    }

    let tw = tab_math::calculate_tab_width(m, tabs.len(), buf_width);
    if tab_math::should_show_number(m, tw) {
        return Some(tab.title);
    }

    // When hovered, close button is always shown.
    let max_chars = tab_math::tab_title_max_chars(m, tw, true);
    let title_chars = tab.title.chars().count();
    (title_chars > max_chars).then_some(tab.title)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics() -> TabLayoutMetrics {
        TabLayoutMetrics {
            cell_width: 9,
            cell_height: 20,
            ui_scale: 1.0,
            tab_bar_height: 36,
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn tab(title: &'static str) -> TabInfo<'static> {
        TabInfo {
            title,
            #[cfg(not(target_os = "macos"))]
            index: 0,
            #[cfg(not(target_os = "macos"))]
            is_active: false,
            #[cfg(not(target_os = "macos"))]
            hover_progress: 0.0,
            #[cfg(not(target_os = "macos"))]
            close_hover_progress: 0.0,
            #[cfg(not(target_os = "macos"))]
            is_renaming: false,
            #[cfg(not(target_os = "macos"))]
            rename_text: None,
            #[cfg(not(target_os = "macos"))]
            rename_cursor: 0,
            #[cfg(not(target_os = "macos"))]
            rename_selection: None,
        }
    }


}
