//! Shared hit-testing logic for the tab bar.
//!
//! Pure functions that determine what element is under a given point.
//! Used by both CPU and GPU renderers to avoid duplicating hit-test logic.

#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::{TabBarHit, TabInfo};
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

/// Returns tab index when pointer is over a security badge.
pub fn hit_test_tab_security_badge(
    x: f64,
    y: f64,
    tabs: &[TabInfo],
    buf_width: u32,
    m: &TabLayoutMetrics,
) -> Option<usize> {
    if tabs.is_empty() {
        return None;
    }

    // Security badges are not rendered when tabs collapse to number mode.
    let tw = tab_math::calculate_tab_width(m, tabs.len(), buf_width);
    if tab_math::should_show_number(m, tw) {
        return None;
    }

    for (idx, tab) in tabs.iter().enumerate() {
        if tab.security_count == 0 {
            continue;
        }
        let Some(rect) =
            tab_math::security_badge_rect(m, idx, tabs.len(), buf_width, tab.security_count)
        else {
            continue;
        };
        if tab_math::point_in_rect(x, y, rect.to_tuple()) {
            return Some(idx);
        }
    }
    None
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
    let max_chars = tab_math::tab_title_max_chars(m, tw, true, tab.security_count);
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

    fn tab(title: &'static str, security_count: usize) -> TabInfo<'static> {
        TabInfo {
            title,
            index: 0,
            is_active: false,
            security_count,
            hover_progress: 0.0,
            close_hover_progress: 0.0,
            is_renaming: false,
            rename_text: None,
            rename_cursor: 0,
            rename_selection: None,
        }
    }

    #[test]
    fn security_badge_hit_test_disabled_in_number_mode() {
        let m = metrics();
        let tabs = [tab("a", 3), tab("b", 1)];
        // Force narrow tabs => number mode.
        let buf_width = 220;
        let tw = tab_math::calculate_tab_width(&m, tabs.len(), buf_width);
        assert!(tab_math::should_show_number(&m, tw));

        // Even if geometry exists, hit-test should be disabled when badge is not rendered.
        let rect =
            tab_math::security_badge_rect(&m, 0, tabs.len(), buf_width, tabs[0].security_count)
                .expect("badge rect should exist geometrically");
        let hit = hit_test_tab_security_badge(
            rect.x as f64 + rect.w as f64 / 2.0,
            rect.y as f64 + rect.h as f64 / 2.0,
            &tabs,
            buf_width,
            &m,
        );
        assert_eq!(hit, None);
    }

    #[test]
    fn security_badge_hit_test_works_in_title_mode() {
        let m = metrics();
        let tabs = [tab("long title", 2), tab("other", 0)];
        let buf_width = 1200;
        let tw = tab_math::calculate_tab_width(&m, tabs.len(), buf_width);
        assert!(!tab_math::should_show_number(&m, tw));

        let rect =
            tab_math::security_badge_rect(&m, 0, tabs.len(), buf_width, tabs[0].security_count)
                .expect("badge rect should exist");
        let hit = hit_test_tab_security_badge(
            rect.x as f64 + rect.w as f64 / 2.0,
            rect.y as f64 + rect.h as f64 / 2.0,
            &tabs,
            buf_width,
            &m,
        );
        assert_eq!(hit, Some(0));
    }
}
