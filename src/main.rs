#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod config;
mod core;
mod i18n;
mod gui;
mod pty;
mod update;
mod update_installer;

fn main() {
    gui::run();
}
