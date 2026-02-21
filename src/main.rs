#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod config;
mod core;
mod i18n;
mod gui;
mod pty;
mod update;

fn main() {
    gui::run();
}
