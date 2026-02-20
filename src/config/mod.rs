mod fonts;
mod model;
mod persistence;
mod theme;

pub(crate) use fonts::font_data;
pub(crate) use model::{AppConfig, FontFamily, ThemeChoice};
pub(crate) use persistence::{config_base_dir, load_config, save_config};
pub(crate) use theme::ThemePalette;
