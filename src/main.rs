#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod core;
mod gui;
mod pty;
mod update;

fn main() {
    gui::run();
}
