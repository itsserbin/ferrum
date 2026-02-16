use super::*;
use std::time::{Duration, Instant};

#[test]
fn default_config_enables_all_protections() {
    let config = SecurityConfig::default();
    assert!(config.paste_protection);
    assert!(config.block_title_query);
    assert!(config.limit_cursor_jumps);
    assert!(config.clear_mouse_on_reset);
}

#[test]
fn paste_payload_with_newline_records_event() {
    let mut guard = SecurityGuard::new();
    guard.check_paste_payload("echo test\nrm -rf /");
    assert!(guard.has_events());
    assert_eq!(guard.active_event_count(), 1);
    assert_eq!(
        guard.take_active_events()[0].kind,
        SecurityEventKind::PasteInjection
    );
}

#[test]
fn active_event_count_ignores_expired_entries() {
    let mut guard = SecurityGuard::new();
    guard.events.push(SecurityEvent {
        kind: SecurityEventKind::TitleQuery,
        timestamp: Instant::now() - Duration::from_secs(31),
    });
    guard.events.push(SecurityEvent {
        kind: SecurityEventKind::CursorRewrite,
        timestamp: Instant::now(),
    });

    assert_eq!(guard.active_event_count(), 1);
}
