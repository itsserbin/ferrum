//! Lightweight security event tracking for terminal/session anomalies.
//!
//! The guard keeps short-lived events for UI signaling (badge, popup) with
//! deduplication to avoid noisy repeats.

use std::time::{Duration, Instant};

/// Runtime toggles for security checks and emitted events.
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

/// Type of security event emitted by the terminal parser/runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecurityEventKind {
    PasteInjection,
    TitleQuery,
    CursorRewrite,
    MouseLeak,
}

impl SecurityEventKind {
    /// Human-readable label used in the UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::PasteInjection => "Paste with newlines detected",
            Self::TitleQuery => "OSC/CSI title query blocked",
            Self::CursorRewrite => "Cursor rewrite detected",
            Self::MouseLeak => "Mouse reporting leak prevented",
        }
    }
}

/// A security event instance with timestamp.
#[derive(Clone, Copy, Debug)]
pub struct SecurityEvent {
    pub kind: SecurityEventKind,
    pub timestamp: Instant,
}

/// Collects and filters security events for one terminal tab.
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
    /// Event lifetime in the active queue.
    const EVENT_TTL: Duration = Duration::from_secs(30);
    /// Deduplication window for repeating same-kind events.
    const DEDUPE_WINDOW: Duration = Duration::from_millis(500);

    /// Creates a guard with default config and empty event list.
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

    /// Records an event if enabled, keeping only live and non-duplicate entries.
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

    /// Records paste-injection event when payload contains line breaks.
    pub fn check_paste_payload(&mut self, text: &str) {
        if text.contains('\n') || text.contains('\r') {
            self.record(SecurityEventKind::PasteInjection);
        }
    }

    /// Counts currently active (non-expired) events.
    pub fn active_event_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.timestamp.elapsed() <= Self::EVENT_TTL)
            .count()
    }

    /// Fast check for any active security events.
    pub fn has_events(&self) -> bool {
        self.active_event_count() > 0
    }

    /// Returns active events and clears the internal queue.
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
