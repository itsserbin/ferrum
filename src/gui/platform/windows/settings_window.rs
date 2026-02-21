use crate::config::AppConfig;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);

pub fn is_settings_window_open() -> bool {
    WINDOW_OPEN.load(Ordering::Relaxed)
}

pub fn open_settings_window(config: &AppConfig, tx: mpsc::Sender<AppConfig>) {
    if WINDOW_OPEN.load(Ordering::Relaxed) {
        // TODO: bring window to front
        return;
    }
    WINDOW_OPEN.store(true, Ordering::Relaxed);
    // TODO: implement Win32 settings window
    let _ = (config, tx);
    eprintln!("[ferrum] Win32 settings window not yet implemented");
    WINDOW_OPEN.store(false, Ordering::Relaxed);
}

pub fn close_settings_window() {
    WINDOW_OPEN.store(false, Ordering::Relaxed);
}
