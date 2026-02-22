mod fonts;
mod model;
mod persistence;
mod theme;

pub(crate) use fonts::load_fonts;
pub(crate) use model::{AppConfig, FontFamily, SecurityMode, ThemeChoice};
pub(crate) use model::{FontConfig, LayoutConfig, SecuritySettings, TerminalConfig, UpdatesConfig};
pub(crate) use persistence::{config_base_dir, load_config, save_config};
pub(crate) use theme::ThemePalette;
