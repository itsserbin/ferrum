mod keyboard;
#[cfg(not(target_os = "linux"))]
mod menu_actions;
mod mouse;
mod pty;
mod redraw;
mod render_cpu;
mod render_gpu;
mod render_shared;
mod settings_apply;
mod settings_toggle;
