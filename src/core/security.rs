use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct SecurityConfig {
    pub paste_protection: bool,
    pub block_title_query: bool,
    pub limit_cursor_jumps: bool,
    pub clear_mouse_on_reset: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            paste_protection: true,
            block_title_query: true,
            limit_cursor_jumps: true,
            clear_mouse_on_reset: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecurityEventKind {
    PasteInjection,
    TitleQuery,
    CursorRewrite,
    MouseLeak,
}

impl SecurityEventKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::PasteInjection => "Paste with newlines detected",
            Self::TitleQuery => "OSC/CSI title query blocked",
            Self::CursorRewrite => "Cursor rewrite detected",
            Self::MouseLeak => "Mouse reporting leak prevented",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SecurityEvent {
    pub kind: SecurityEventKind,
    pub timestamp: Instant,
}

pub struct SecurityGuard {
    pub config: SecurityConfig,
    pub events: Vec<SecurityEvent>,
}

impl Default for SecurityGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityGuard {
    const EVENT_TTL: Duration = Duration::from_secs(30);
    const DEDUPE_WINDOW: Duration = Duration::from_millis(500);

    pub fn new() -> Self {
        Self {
            config: SecurityConfig::default(),
            events: Vec::new(),
        }
    }

    fn is_event_enabled(&self, kind: SecurityEventKind) -> bool {
        match kind {
            SecurityEventKind::PasteInjection => self.config.paste_protection,
            SecurityEventKind::TitleQuery => self.config.block_title_query,
            SecurityEventKind::CursorRewrite => self.config.limit_cursor_jumps,
            SecurityEventKind::MouseLeak => self.config.clear_mouse_on_reset,
        }
    }

    pub fn record(&mut self, kind: SecurityEventKind) {
        if !self.is_event_enabled(kind) {
            return;
        }

        let now = Instant::now();
        self.events
            .retain(|event| now.duration_since(event.timestamp) <= Self::EVENT_TTL);

        if self.events.last().is_some_and(|event| {
            event.kind == kind && now.duration_since(event.timestamp) <= Self::DEDUPE_WINDOW
        }) {
            return;
        }

        self.events.push(SecurityEvent {
            kind,
            timestamp: now,
        });
    }

    pub fn should_wrap_paste(&self) -> bool {
        self.config.paste_protection
    }

    pub fn check_paste_payload(&mut self, text: &str) {
        if text.contains('\n') || text.contains('\r') {
            self.record(SecurityEventKind::PasteInjection);
        }
    }

    pub fn active_event_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.timestamp.elapsed() <= Self::EVENT_TTL)
            .count()
    }

    pub fn has_events(&self) -> bool {
        self.active_event_count() > 0
    }

    pub fn take_active_events(&mut self) -> Vec<SecurityEvent> {
        let active: Vec<SecurityEvent> = self
            .events
            .iter()
            .copied()
            .filter(|event| event.timestamp.elapsed() <= Self::EVENT_TTL)
            .collect();
        self.events.clear();
        active
    }
}

#[cfg(test)]
#[path = "../../tests/unit/core_security.rs"]
mod tests;
