use crate::gui::renderer::types::{scaled_px, FlatRectCmd, RoundedRectCmd, TextCmd};

use super::{SettingItem, SettingsCategory, SettingsOverlay, StepperHalf};

// ── Layout structs ─────────────────────────────────────────────────

/// Pre-computed layout for the settings overlay panel.
pub(in crate::gui) struct SettingsOverlayLayout {
    /// Full-screen semi-transparent dim overlay.
    pub dim_bg: FlatRectCmd,
    /// Drop shadow behind the panel (drawn before panel_bg).
    pub panel_shadow: RoundedRectCmd,
    /// Main panel background.
    pub panel_bg: RoundedRectCmd,
    /// Border overlay drawn on top of the panel background.
    pub panel_border: RoundedRectCmd,
    /// "Settings" title text at the top of the panel.
    pub title: TextCmd,
    /// Horizontal separator line below the title.
    pub title_separator: FlatRectCmd,
    /// Vertical separator between sidebar and content area.
    pub sidebar_separator: FlatRectCmd,
    /// Layout for each category entry in the sidebar.
    pub categories: Vec<CategoryLayout>,
    /// Layout for each setting item in the content area.
    pub items: Vec<ItemLayout>,
    /// "Esc to close" hint text at the bottom-right of the panel.
    pub close_hint: TextCmd,
    /// Close button (X) in the top-right corner.
    pub close_button: RoundedRectCmd,
    /// First diagonal line of the X icon (top-left to bottom-right).
    pub close_icon_line_a: (f32, f32, f32, f32),
    /// Second diagonal line of the X icon (top-right to bottom-left).
    pub close_icon_line_b: (f32, f32, f32, f32),
    /// Color for the X icon lines.
    pub close_icon_color: u32,
}

/// Layout for a single category row in the sidebar.
pub(in crate::gui) struct CategoryLayout {
    /// Background rectangle for hover/active state.
    pub bg: FlatRectCmd,
    /// Category label text.
    pub text: TextCmd,
    /// Whether this category is the currently active one.
    #[allow(dead_code)] // Read in tests; will be used for rendering differentiation.
    pub is_active: bool,
    /// Left accent bar for the active category (None if not active).
    pub indicator: Option<FlatRectCmd>,
}

/// Layout for a single setting item row in the content area.
pub(in crate::gui) struct ItemLayout {
    /// Optional hover background for this item row.
    pub row_bg: Option<FlatRectCmd>,
    /// Setting label text.
    pub label: TextCmd,
    /// Control-specific layout (stepper, dropdown).
    pub controls: ItemControlLayout,
}

/// Control-specific layout variants for setting items.
pub(in crate::gui) enum ItemControlLayout {
    /// Stepper: [-] value [+] for numeric values.
    Stepper {
        minus_btn: RoundedRectCmd,
        minus_text: TextCmd,
        value_text: TextCmd,
        plus_btn: RoundedRectCmd,
        plus_text: TextCmd,
    },
    /// Dropdown: button with current value + arrow, expandable list.
    Dropdown {
        button: RoundedRectCmd,
        button_text: TextCmd,
        arrow_text: TextCmd,
        /// Populated only when this dropdown is open.
        options: Vec<DropdownOptionLayout>,
    },
    /// Boolean toggle: clickable pill showing ON or OFF.
    Toggle {
        pill: RoundedRectCmd,
        pill_text: TextCmd,
    },
}

/// Layout for a single option in an open dropdown list.
pub(in crate::gui) struct DropdownOptionLayout {
    pub bg: FlatRectCmd,
    pub text: TextCmd,
    #[allow(dead_code)] // Available for future rendering differentiation.
    pub is_selected: bool,
    #[allow(dead_code)] // Available for future rendering differentiation.
    pub is_hovered: bool,
}

// ── Panel geometry constants (base pixels, before DPI scaling) ──────

const PANEL_WIDTH_FRACTION: f32 = 0.70;
const PANEL_MIN_WIDTH: u32 = 400;
const PANEL_MAX_WIDTH: u32 = 800;

const PANEL_HEIGHT_FRACTION: f32 = 0.80;
const PANEL_MIN_HEIGHT: u32 = 300;
const PANEL_MAX_HEIGHT: u32 = 700;

const PANEL_CORNER_RADIUS: u32 = 8;

/// Sidebar takes ~30% of panel width.
const SIDEBAR_FRACTION: f32 = 0.30;

/// Base height multiplier for category rows (cell_height * 2).
const CATEGORY_ROW_HEIGHT_MULT: u32 = 2;

/// Base height multiplier for item rows (cell_height * 2.5, approximated as 5/2).
const ITEM_ROW_HEIGHT_NUMER: u32 = 5;
const ITEM_ROW_HEIGHT_DENOM: u32 = 2;

/// Stepper button size in base pixels.
const STEPPER_BTN_SIZE: u32 = 20;

/// Dropdown button height in base pixels.
const DROPDOWN_HEIGHT: u32 = 24;

/// Dropdown option row height in base pixels.
const DROPDOWN_OPTION_HEIGHT: u32 = 24;

/// Internal padding in base pixels.
const INNER_PAD: u32 = 8;

/// Small padding in base pixels.
const SMALL_PAD: u32 = 4;

// ── Layout computation ─────────────────────────────────────────────

/// Computes the full visual layout for the settings overlay.
///
/// All pixel values are DPI-scaled. The returned [`SettingsOverlayLayout`]
/// contains every rect and text span needed to draw the settings panel --
/// renderers just iterate and issue their backend-specific draw calls.
#[allow(clippy::too_many_arguments)]
pub(in crate::gui) fn compute_settings_layout(
    buf_width: u32,
    buf_height: u32,
    cell_width: u32,
    cell_height: u32,
    ui_scale: f64,
    overlay: &SettingsOverlay,
    palette_menu_bg: u32,
    palette_active_accent: u32,
    palette_text_active: u32,
    palette_text_inactive: u32,
    palette_bar_bg: u32,
    palette_close_hover_bg: u32,
) -> SettingsOverlayLayout {
    let sp = |base| scaled_px(base, ui_scale);

    // ── Panel dimensions ───────────────────────────────────────────
    let raw_w = (buf_width as f32 * PANEL_WIDTH_FRACTION) as u32;
    let panel_w = raw_w.clamp(PANEL_MIN_WIDTH.min(buf_width), PANEL_MAX_WIDTH.min(buf_width));

    let raw_h = (buf_height as f32 * PANEL_HEIGHT_FRACTION) as u32;
    let panel_h = raw_h.clamp(PANEL_MIN_HEIGHT.min(buf_height), PANEL_MAX_HEIGHT.min(buf_height));

    let panel_x = (buf_width.saturating_sub(panel_w)) / 2;
    let panel_y = (buf_height.saturating_sub(panel_h)) / 2;

    let px = panel_x as f32;
    let py = panel_y as f32;
    let pw = panel_w as f32;
    let ph = panel_h as f32;
    let radius = sp(PANEL_CORNER_RADIUS) as f32;

    // ── Dim background ─────────────────────────────────────────────
    let dim_bg = FlatRectCmd {
        x: 0.0,
        y: 0.0,
        w: buf_width as f32,
        h: buf_height as f32,
        color: 0x000000,
        opacity: 0.5,
    };

    // ── Panel background & border ──────────────────────────────────
    let panel_bg = RoundedRectCmd {
        x: px,
        y: py,
        w: pw,
        h: ph,
        radius,
        color: palette_menu_bg,
        opacity: 0.97,
    };

    let shadow_offset = sp(2) as f32;
    let panel_shadow = RoundedRectCmd {
        x: px + shadow_offset,
        y: py + shadow_offset,
        w: pw,
        h: ph,
        radius,
        color: 0x000000,
        opacity: 0.24,
    };

    let panel_border = RoundedRectCmd {
        x: px,
        y: py,
        w: pw,
        h: ph,
        radius,
        color: palette_active_accent,
        opacity: 0.12,
    };

    // ── Title ──────────────────────────────────────────────────────
    let pad = sp(INNER_PAD) as f32;
    let small = sp(SMALL_PAD) as f32;
    let title_x = px + pad;
    let title_y = py + small;

    let title = TextCmd {
        x: title_x,
        y: title_y,
        text: "Settings".to_string(),
        color: palette_text_active,
        opacity: 1.0,
    };

    // Title separator: full panel width minus padding on each side.
    let sep_y = py + cell_height as f32 + small * 2.0;
    let title_separator = FlatRectCmd {
        x: px + small,
        y: sep_y,
        w: pw - small * 2.0,
        h: 1.0,
        color: palette_active_accent,
        opacity: 0.3,
    };

    // ── Two-column split ───────────────────────────────────────────
    let content_top = sep_y + 1.0 + small;
    let sidebar_w = (pw * SIDEBAR_FRACTION) as u32;
    let content_x = px + sidebar_w as f32;
    let content_w = pw - sidebar_w as f32;
    let content_bottom = py + ph - cell_height as f32 - small;

    // Vertical sidebar separator line.
    let sidebar_separator = FlatRectCmd {
        x: content_x,
        y: content_top,
        w: 1.0,
        h: content_bottom - content_top,
        color: palette_active_accent,
        opacity: 0.15,
    };

    // ── Categories (sidebar) ───────────────────────────────────────
    let cat_row_h = cell_height * CATEGORY_ROW_HEIGHT_MULT;
    let categories = build_category_layouts(
        overlay,
        px,
        content_top,
        sidebar_w as f32,
        cat_row_h,
        pad,
        cell_height,
        palette_active_accent,
        palette_text_active,
        palette_text_inactive,
        palette_bar_bg,
    );

    // ── Items (content area) ───────────────────────────────────────
    let item_row_h = cell_height * ITEM_ROW_HEIGHT_NUMER / ITEM_ROW_HEIGHT_DENOM;
    let items_list = overlay.items();
    let items = build_item_layouts(
        &items_list,
        content_x + 1.0 + pad,
        content_top,
        content_w - 1.0 - pad * 2.0,
        item_row_h,
        cell_width,
        cell_height,
        ui_scale,
        overlay,
        palette_active_accent,
        palette_text_active,
        palette_text_inactive,
        palette_bar_bg,
    );

    // ── Close hint ─────────────────────────────────────────────────
    let hint_text = "Esc to close";
    let hint_text_w = hint_text.len() as f32 * cell_width as f32;
    let close_hint = TextCmd {
        x: px + pw - hint_text_w - pad,
        y: py + ph - cell_height as f32 - small,
        text: hint_text.to_string(),
        color: palette_text_inactive,
        opacity: 0.6,
    };

    // ── Close button (X) ───────────────────────────────────────────
    let close_size = sp(STEPPER_BTN_SIZE) as f32;
    let close_x = px + pw - close_size - pad;
    let close_y = py + pad;

    let (close_color, close_opacity) = if overlay.hovered_close {
        (palette_close_hover_bg, 0.8)
    } else {
        (palette_bar_bg, 0.6)
    };

    let close_button = RoundedRectCmd {
        x: close_x,
        y: close_y,
        w: close_size,
        h: close_size,
        radius: sp(SMALL_PAD) as f32,
        color: close_color,
        opacity: close_opacity,
    };

    let close_cx = close_x + close_size / 2.0;
    let close_cy = close_y + close_size / 2.0;
    let close_half = close_size * 0.25;
    let close_icon_line_a = (
        close_cx - close_half,
        close_cy - close_half,
        close_cx + close_half,
        close_cy + close_half,
    );
    let close_icon_line_b = (
        close_cx + close_half,
        close_cy - close_half,
        close_cx - close_half,
        close_cy + close_half,
    );

    SettingsOverlayLayout {
        dim_bg,
        panel_shadow,
        panel_bg,
        panel_border,
        title,
        title_separator,
        sidebar_separator,
        categories,
        items,
        close_hint,
        close_button,
        close_icon_line_a,
        close_icon_line_b,
        close_icon_color: palette_text_inactive,
    }
}

// ── Category row builder ───────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn build_category_layouts(
    overlay: &SettingsOverlay,
    panel_x: f32,
    content_top: f32,
    sidebar_w: f32,
    row_h: u32,
    pad: f32,
    cell_height: u32,
    accent: u32,
    text_active: u32,
    text_inactive: u32,
    bar_bg: u32,
) -> Vec<CategoryLayout> {
    SettingsCategory::CATEGORIES
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let is_active = *cat == overlay.active_category;
            let is_hovered = overlay.hovered_category == Some(i);

            let row_y = content_top + (i as u32 * row_h) as f32;

            // Background: active = accent at low opacity, hovered = bar_bg.
            let (bg_color, bg_opacity) = if is_active {
                (accent, 0.15_f32)
            } else if is_hovered {
                (bar_bg, 0.5)
            } else {
                (0x000000, 0.0)
            };

            let bg = FlatRectCmd {
                x: panel_x,
                y: row_y,
                w: sidebar_w,
                h: row_h as f32,
                color: bg_color,
                opacity: bg_opacity,
            };

            // Text centered vertically in the row.
            let text_y = row_y + (row_h as f32 - cell_height as f32) / 2.0;
            let text_color = if is_active { text_active } else { text_inactive };

            let text = TextCmd {
                x: panel_x + pad,
                y: text_y,
                text: cat.to_string(),
                color: text_color,
                opacity: 1.0,
            };

            let indicator = if is_active {
                Some(FlatRectCmd {
                    x: panel_x,
                    y: row_y,
                    w: 2.0,
                    h: row_h as f32,
                    color: accent,
                    opacity: 1.0,
                })
            } else {
                None
            };

            CategoryLayout {
                bg,
                text,
                is_active,
                indicator,
            }
        })
        .collect()
}

// ── Item row builder ───────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn build_item_layouts(
    items: &[SettingItem],
    area_x: f32,
    area_y: f32,
    area_w: f32,
    row_h: u32,
    cell_width: u32,
    cell_height: u32,
    ui_scale: f64,
    overlay: &SettingsOverlay,
    accent: u32,
    text_active: u32,
    text_inactive: u32,
    bar_bg: u32,
) -> Vec<ItemLayout> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let row_y = area_y + (i as u32 * row_h) as f32;
            let label_y = row_y + (row_h as f32 - cell_height as f32) / 2.0;

            let is_hovered = overlay.hovered_item == Some(i);
            let row_bg = if is_hovered {
                Some(FlatRectCmd {
                    x: area_x,
                    y: row_y,
                    w: area_w,
                    h: row_h as f32,
                    color: bar_bg,
                    opacity: 0.2,
                })
            } else {
                None
            };

            let label_text = item_label(item);
            let label = TextCmd {
                x: area_x,
                y: label_y,
                text: label_text.to_string(),
                color: text_active,
                opacity: 1.0,
            };

            // Controls start at ~45% of the area width to leave room for label.
            let control_x = area_x + area_w * 0.45;
            let control_w = area_w * 0.55;

            let controls = match item {
                SettingItem::FloatSlider { value, .. } => build_stepper_control(
                    control_x,
                    label_y,
                    control_w,
                    cell_height,
                    ui_scale,
                    &format_float_value(*value),
                    accent,
                    text_inactive,
                    bar_bg,
                    overlay,
                    i,
                ),
                SettingItem::IntSlider { value, .. } => build_stepper_control(
                    control_x,
                    label_y,
                    control_w,
                    cell_height,
                    ui_scale,
                    &value.to_string(),
                    accent,
                    text_inactive,
                    bar_bg,
                    overlay,
                    i,
                ),
                SettingItem::LargeIntSlider { value, .. } => build_stepper_control(
                    control_x,
                    label_y,
                    control_w,
                    cell_height,
                    ui_scale,
                    &value.to_string(),
                    accent,
                    text_inactive,
                    bar_bg,
                    overlay,
                    i,
                ),
                SettingItem::EnumChoice {
                    options, selected, ..
                } => build_dropdown_control(
                    control_x,
                    label_y,
                    control_w,
                    cell_width,
                    cell_height,
                    ui_scale,
                    options,
                    *selected,
                    i,
                    overlay,
                    accent,
                    text_active,
                    text_inactive,
                    bar_bg,
                ),
                SettingItem::BoolToggle { value, .. } => build_toggle_control(
                    control_x,
                    label_y,
                    cell_height,
                    ui_scale,
                    *value,
                    accent,
                    text_active,
                    text_inactive,
                    bar_bg,
                    overlay,
                    i,
                ),
            };

            ItemLayout { row_bg, label, controls }
        })
        .collect()
}

/// Extracts the label string from any `SettingItem` variant.
fn item_label(item: &SettingItem) -> &'static str {
    match item {
        SettingItem::FloatSlider { label, .. }
        | SettingItem::IntSlider { label, .. }
        | SettingItem::LargeIntSlider { label, .. }
        | SettingItem::EnumChoice { label, .. }
        | SettingItem::BoolToggle { label, .. } => label,
    }
}

/// Formats a float value for display (e.g. "14.0").
fn format_float_value(v: f32) -> String {
    if v.fract().abs() < f32::EPSILON {
        format!("{:.0}", v)
    } else {
        format!("{:.1}", v)
    }
}

// ── Stepper control ─────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn build_stepper_control(
    x: f32,
    row_center_y: f32,
    _available_w: f32,
    cell_height: u32,
    ui_scale: f64,
    value_str: &str,
    accent: u32,
    text_inactive: u32,
    bar_bg: u32,
    overlay: &SettingsOverlay,
    item_index: usize,
) -> ItemControlLayout {
    let sp = |base| scaled_px(base, ui_scale);
    let btn_size = sp(STEPPER_BTN_SIZE) as f32;
    let btn_radius = sp(SMALL_PAD) as f32;
    let gap = sp(SMALL_PAD) as f32;

    let btn_y = row_center_y + (cell_height as f32 - btn_size) / 2.0;

    // Minus button.
    let minus_hovered = overlay.hovered_stepper == Some((item_index, StepperHalf::Minus));
    let minus_btn = RoundedRectCmd {
        x,
        y: btn_y,
        w: btn_size,
        h: btn_size,
        radius: btn_radius,
        color: bar_bg,
        opacity: if minus_hovered { 1.0 } else { 0.6 },
    };

    let minus_text = TextCmd {
        x: x + (btn_size - cell_height as f32 * 0.5) / 2.0,
        y: row_center_y,
        text: "-".to_string(),
        color: accent,
        opacity: 1.0,
    };

    // Value text centered between buttons.
    let value_x = x + btn_size + gap;
    // Reserve enough space for value text.
    let value_text_w = (value_str.len() as f32 + 1.0) * cell_height as f32 * 0.5;
    let value_text = TextCmd {
        x: value_x,
        y: row_center_y,
        text: value_str.to_string(),
        color: text_inactive,
        opacity: 1.0,
    };

    // Plus button.
    let plus_x = value_x + value_text_w + gap;
    let plus_hovered = overlay.hovered_stepper == Some((item_index, StepperHalf::Plus));
    let plus_btn = RoundedRectCmd {
        x: plus_x,
        y: btn_y,
        w: btn_size,
        h: btn_size,
        radius: btn_radius,
        color: bar_bg,
        opacity: if plus_hovered { 1.0 } else { 0.6 },
    };

    let plus_text = TextCmd {
        x: plus_x + (btn_size - cell_height as f32 * 0.5) / 2.0,
        y: row_center_y,
        text: "+".to_string(),
        color: accent,
        opacity: 1.0,
    };

    ItemControlLayout::Stepper {
        minus_btn,
        minus_text,
        value_text,
        plus_btn,
        plus_text,
    }
}

// ── Toggle control ──────────────────────────────────────────────────

/// Toggle pill height in base pixels.
const TOGGLE_HEIGHT: u32 = 22;
/// Toggle pill width in base pixels.
const TOGGLE_WIDTH: u32 = 40;

#[allow(clippy::too_many_arguments)]
fn build_toggle_control(
    x: f32,
    row_center_y: f32,
    cell_height: u32,
    ui_scale: f64,
    value: bool,
    accent: u32,
    _text_active: u32,
    text_inactive: u32,
    bar_bg: u32,
    overlay: &SettingsOverlay,
    item_index: usize,
) -> ItemControlLayout {
    let sp = |base| scaled_px(base, ui_scale);
    let pill_w = sp(TOGGLE_WIDTH) as f32;
    let pill_h = sp(TOGGLE_HEIGHT) as f32;
    let btn_radius = sp(SMALL_PAD) as f32;

    let pill_y = row_center_y + (cell_height as f32 - pill_h) / 2.0;

    let is_hovered = overlay.hovered_stepper == Some((item_index, StepperHalf::Minus));

    let (pill_color, pill_opacity) = if value {
        (accent, if is_hovered { 0.9 } else { 0.7 })
    } else {
        (bar_bg, if is_hovered { 0.8 } else { 0.5 })
    };

    let pill = RoundedRectCmd {
        x,
        y: pill_y,
        w: pill_w,
        h: pill_h,
        radius: btn_radius,
        color: pill_color,
        opacity: pill_opacity,
    };

    let label = if value { "ON" } else { "OFF" };
    let text_color = if value { accent } else { text_inactive };

    let pill_text = TextCmd {
        x: x + (pill_w - label.len() as f32 * cell_height as f32 * 0.5) / 2.0,
        y: row_center_y,
        text: label.to_string(),
        color: text_color,
        opacity: 1.0,
    };

    ItemControlLayout::Toggle { pill, pill_text }
}

// ── Dropdown control ────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn build_dropdown_control(
    x: f32,
    row_center_y: f32,
    available_w: f32,
    cell_width: u32,
    cell_height: u32,
    ui_scale: f64,
    options: &[&str],
    selected: usize,
    item_index: usize,
    overlay: &SettingsOverlay,
    accent: u32,
    text_active: u32,
    text_inactive: u32,
    bar_bg: u32,
) -> ItemControlLayout {
    let sp = |base| scaled_px(base, ui_scale);
    let dd_h = sp(DROPDOWN_HEIGHT) as f32;
    let btn_radius = sp(SMALL_PAD) as f32;
    let pad = sp(INNER_PAD) as f32;

    let btn_y = row_center_y + (cell_height as f32 - dd_h) / 2.0;
    let btn_w = available_w.min(cell_width as f32 * 20.0);

    let btn_hovered = overlay.hovered_dropdown == Some(item_index);
    let button = RoundedRectCmd {
        x,
        y: btn_y,
        w: btn_w,
        h: dd_h,
        radius: btn_radius,
        color: bar_bg,
        opacity: if btn_hovered { 0.85 } else { 0.6 },
    };

    let selected_text = options.get(selected).copied().unwrap_or("");
    let button_text = TextCmd {
        x: x + pad,
        y: row_center_y,
        text: selected_text.to_string(),
        color: text_active,
        opacity: 1.0,
    };

    let arrow_text = TextCmd {
        x: x + btn_w - pad - cell_width as f32,
        y: row_center_y,
        text: "v".to_string(),
        color: text_inactive,
        opacity: 0.7,
    };

    // Build dropdown options only when this dropdown is open.
    let dropdown_options = if overlay.open_dropdown == Some(item_index) {
        let opt_h = sp(DROPDOWN_OPTION_HEIGHT) as f32;
        options
            .iter()
            .enumerate()
            .map(|(j, opt)| {
                let opt_y = btn_y + dd_h + j as f32 * opt_h;
                let is_selected = j == selected;
                let is_hovered = overlay.hovered_dropdown_option == Some(j);

                let (bg_color, bg_opacity) = if is_selected {
                    (accent, 0.20)
                } else if is_hovered {
                    (bar_bg, 0.8)
                } else {
                    (bar_bg, 0.5)
                };

                let bg = FlatRectCmd {
                    x,
                    y: opt_y,
                    w: btn_w,
                    h: opt_h,
                    color: bg_color,
                    opacity: bg_opacity,
                };

                let opt_text_color = if is_selected { text_active } else { text_inactive };
                let text = TextCmd {
                    x: x + pad,
                    y: opt_y + (opt_h - cell_height as f32) / 2.0,
                    text: opt.to_string(),
                    color: opt_text_color,
                    opacity: 1.0,
                };

                DropdownOptionLayout {
                    bg,
                    text,
                    is_selected,
                    is_hovered,
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    ItemControlLayout::Dropdown {
        button,
        button_text,
        arrow_text,
        options: dropdown_options,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    fn default_overlay() -> SettingsOverlay {
        SettingsOverlay::new(&AppConfig::default())
    }

    const TEST_MENU_BG: u32 = 0x1E2433;
    const TEST_ACCENT: u32 = 0xB4BEFE;
    const TEST_TEXT_ACTIVE: u32 = 0xD2DBEB;
    const TEST_TEXT_INACTIVE: u32 = 0x6C7480;
    const TEST_BAR_BG: u32 = 0x1E2127;
    const TEST_CLOSE_HOVER_BG: u32 = 0x454B59;

    fn compute_test_layout(overlay: &SettingsOverlay) -> SettingsOverlayLayout {
        compute_settings_layout(
            800,
            600,
            8,
            16,
            1.0,
            overlay,
            TEST_MENU_BG,
            TEST_ACCENT,
            TEST_TEXT_ACTIVE,
            TEST_TEXT_INACTIVE,
            TEST_BAR_BG,
            TEST_CLOSE_HOVER_BG,
        )
    }

    #[test]
    fn dim_bg_covers_full_screen() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        assert_eq!(layout.dim_bg.x, 0.0);
        assert_eq!(layout.dim_bg.y, 0.0);
        assert_eq!(layout.dim_bg.w, 800.0);
        assert_eq!(layout.dim_bg.h, 600.0);
        assert_eq!(layout.dim_bg.color, 0x000000);
    }

    #[test]
    fn panel_is_centered() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        let panel_right = layout.panel_bg.x + layout.panel_bg.w;
        let panel_bottom = layout.panel_bg.y + layout.panel_bg.h;
        // Panel should be within buffer bounds.
        assert!(panel_right <= 800.0);
        assert!(panel_bottom <= 600.0);
        // Centered: left margin ~ right margin.
        let left_margin = layout.panel_bg.x;
        let right_margin = 800.0 - panel_right;
        assert!((left_margin - right_margin).abs() < 2.0);
    }

    #[test]
    fn panel_border_matches_bg() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        assert_eq!(layout.panel_border.x, layout.panel_bg.x);
        assert_eq!(layout.panel_border.y, layout.panel_bg.y);
        assert_eq!(layout.panel_border.w, layout.panel_bg.w);
        assert_eq!(layout.panel_border.h, layout.panel_bg.h);
    }

    #[test]
    fn title_text_is_settings() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        assert_eq!(layout.title.text, "Settings");
        assert_eq!(layout.title.color, TEST_TEXT_ACTIVE);
    }

    #[test]
    fn categories_match_count() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        assert_eq!(layout.categories.len(), SettingsCategory::CATEGORIES.len());
    }

    #[test]
    fn first_category_is_active_by_default() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        assert!(layout.categories[0].is_active);
        assert!(!layout.categories[1].is_active);
    }

    #[test]
    fn font_category_has_three_items() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        assert_eq!(layout.items.len(), 3);
    }

    #[test]
    fn theme_category_has_one_item() {
        let config = AppConfig::default();
        let mut overlay = SettingsOverlay::new(&config);
        overlay.active_category = SettingsCategory::Theme;
        let layout = compute_test_layout(&overlay);
        assert_eq!(layout.items.len(), 1);
    }

    #[test]
    fn close_hint_text_is_correct() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        assert_eq!(layout.close_hint.text, "Esc to close");
        assert_eq!(layout.close_hint.color, TEST_TEXT_INACTIVE);
    }

    #[test]
    fn stepper_has_minus_and_plus_buttons() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        // First Font item is FloatSlider for font size -> Stepper.
        match &layout.items[0].controls {
            ItemControlLayout::Stepper {
                minus_text,
                plus_text,
                value_text,
                ..
            } => {
                assert_eq!(minus_text.text, "-");
                assert_eq!(plus_text.text, "+");
                assert!(!value_text.text.is_empty());
            }
            _ => panic!("expected Stepper control for font size"),
        }
    }

    #[test]
    fn dropdown_shows_selected_option() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        // Second Font item is EnumChoice for font family -> Dropdown.
        match &layout.items[1].controls {
            ItemControlLayout::Dropdown {
                button_text,
                options,
                ..
            } => {
                assert_eq!(button_text.text, "JetBrains Mono");
                // Dropdown is closed by default, so no options.
                assert!(options.is_empty());
            }
            _ => panic!("expected Dropdown control for font family"),
        }
    }

    #[test]
    fn open_dropdown_populates_options() {
        let config = AppConfig::default();
        let mut overlay = SettingsOverlay::new(&config);
        // Font Family is at item index 1.
        overlay.open_dropdown = Some(1);
        let layout = compute_test_layout(&overlay);
        match &layout.items[1].controls {
            ItemControlLayout::Dropdown { options, .. } => {
                assert_eq!(options.len(), 2);
            }
            _ => panic!("expected Dropdown control for font family"),
        }
    }

    #[test]
    fn close_button_is_in_top_right() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        // Close button should be near the top-right of the panel.
        let panel_right = layout.panel_bg.x + layout.panel_bg.w;
        assert!(layout.close_button.x + layout.close_button.w <= panel_right);
        assert!(layout.close_button.y >= layout.panel_bg.y);
    }

    #[test]
    fn hidpi_scales_panel_radius() {
        let overlay = default_overlay();
        let layout_1x = compute_settings_layout(
            800,
            600,
            8,
            16,
            1.0,
            &overlay,
            TEST_MENU_BG,
            TEST_ACCENT,
            TEST_TEXT_ACTIVE,
            TEST_TEXT_INACTIVE,
            TEST_BAR_BG,
            TEST_CLOSE_HOVER_BG,
        );
        let layout_2x = compute_settings_layout(
            800,
            600,
            16,
            32,
            2.0,
            &overlay,
            TEST_MENU_BG,
            TEST_ACCENT,
            TEST_TEXT_ACTIVE,
            TEST_TEXT_INACTIVE,
            TEST_BAR_BG,
            TEST_CLOSE_HOVER_BG,
        );
        assert!(layout_2x.panel_bg.radius > layout_1x.panel_bg.radius);
    }

    #[test]
    fn small_buffer_clamps_panel_to_buffer() {
        let overlay = default_overlay();
        let layout = compute_settings_layout(
            200,
            200,
            8,
            16,
            1.0,
            &overlay,
            TEST_MENU_BG,
            TEST_ACCENT,
            TEST_TEXT_ACTIVE,
            TEST_TEXT_INACTIVE,
            TEST_BAR_BG,
            TEST_CLOSE_HOVER_BG,
        );
        assert!(layout.panel_bg.x + layout.panel_bg.w <= 200.0);
        assert!(layout.panel_bg.y + layout.panel_bg.h <= 200.0);
    }

    #[test]
    fn sidebar_separator_is_vertical() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        assert_eq!(layout.sidebar_separator.w, 1.0);
        assert!(layout.sidebar_separator.h > 0.0);
    }

    #[test]
    fn title_separator_spans_panel_width() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        let pad = scaled_px(SMALL_PAD, 1.0) as f32;
        let expected_w = layout.panel_bg.w - pad * 2.0;
        assert!((layout.title_separator.w - expected_w).abs() < 1.0);
    }

    #[test]
    fn panel_shadow_has_offset() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        let offset = layout.panel_shadow.x - layout.panel_bg.x;
        assert!(offset > 0.0, "shadow should be offset to the right");
        assert_eq!(
            layout.panel_shadow.x - layout.panel_bg.x,
            layout.panel_shadow.y - layout.panel_bg.y,
            "shadow X and Y offsets should be equal"
        );
        assert_eq!(layout.panel_shadow.color, 0x000000);
        assert!((layout.panel_shadow.opacity - 0.24).abs() < 0.01);
    }

    #[test]
    fn panel_border_uses_accent_color() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        assert_eq!(layout.panel_border.color, TEST_ACCENT);
        assert!((layout.panel_border.opacity - 0.12).abs() < 0.01);
    }

    #[test]
    fn active_category_has_indicator() {
        let overlay = default_overlay();
        let layout = compute_test_layout(&overlay);
        assert!(layout.categories[0].indicator.is_some());
        let ind = layout.categories[0].indicator.as_ref().unwrap();
        assert_eq!(ind.w, 2.0);
        assert_eq!(ind.color, TEST_ACCENT);
        assert!(layout.categories[1].indicator.is_none());
    }

    #[test]
    fn close_button_hover_changes_opacity() {
        let config = AppConfig::default();
        let mut overlay = SettingsOverlay::new(&config);
        let layout_normal = compute_test_layout(&overlay);
        overlay.hovered_close = true;
        let layout_hovered = compute_test_layout(&overlay);
        assert!(layout_hovered.close_button.opacity > layout_normal.close_button.opacity);
    }

    #[test]
    fn stepper_hover_changes_opacity() {
        let config = AppConfig::default();
        let mut overlay = SettingsOverlay::new(&config);
        let layout_normal = compute_test_layout(&overlay);
        overlay.hovered_stepper = Some((0, StepperHalf::Minus));
        let layout_hovered = compute_test_layout(&overlay);
        match (
            &layout_normal.items[0].controls,
            &layout_hovered.items[0].controls,
        ) {
            (
                ItemControlLayout::Stepper {
                    minus_btn: normal, ..
                },
                ItemControlLayout::Stepper {
                    minus_btn: hovered, ..
                },
            ) => {
                assert!(hovered.opacity > normal.opacity);
            }
            _ => panic!("expected Stepper"),
        }
    }

    #[test]
    fn dropdown_hover_changes_opacity() {
        let config = AppConfig::default();
        let mut overlay = SettingsOverlay::new(&config);
        let layout_normal = compute_test_layout(&overlay);
        overlay.hovered_dropdown = Some(1); // Font Family dropdown
        let layout_hovered = compute_test_layout(&overlay);
        match (
            &layout_normal.items[1].controls,
            &layout_hovered.items[1].controls,
        ) {
            (
                ItemControlLayout::Dropdown {
                    button: normal, ..
                },
                ItemControlLayout::Dropdown {
                    button: hovered, ..
                },
            ) => {
                assert!(hovered.opacity > normal.opacity);
            }
            _ => panic!("expected Dropdown"),
        }
    }

    #[test]
    fn item_row_hover_produces_background() {
        let config = AppConfig::default();
        let mut overlay = SettingsOverlay::new(&config);
        assert!(compute_test_layout(&overlay).items[0].row_bg.is_none());
        overlay.hovered_item = Some(0);
        let layout = compute_test_layout(&overlay);
        assert!(layout.items[0].row_bg.is_some());
        let bg = layout.items[0].row_bg.as_ref().unwrap();
        assert_eq!(bg.color, TEST_BAR_BG);
        assert!((bg.opacity - 0.2).abs() < 0.01);
    }
}
