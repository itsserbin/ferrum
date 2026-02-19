use crate::gui::*;

use super::super::super::state::RenameState;

fn selected_range(anchor: Option<usize>, cursor: usize) -> Option<(usize, usize)> {
    let anchor = anchor?;
    if anchor == cursor {
        None
    } else {
        Some((anchor.min(cursor), anchor.max(cursor)))
    }
}

fn prev_char_boundary(s: &str, idx: usize) -> usize {
    let idx = idx.min(s.len());
    if idx == 0 {
        return 0;
    }

    let mut prev = 0;
    for (i, _) in s[..idx].char_indices() {
        prev = i;
    }
    prev
}

fn next_char_boundary(s: &str, idx: usize) -> usize {
    let idx = idx.min(s.len());
    if idx >= s.len() {
        return s.len();
    }
    idx + s[idx..].chars().next().map_or(0, char::len_utf8)
}

fn word_left_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());

    while idx > 0 {
        let prev = prev_char_boundary(s, idx);
        let ch = s[prev..idx].chars().next().unwrap_or(' ');
        if !ch.is_whitespace() {
            break;
        }
        idx = prev;
    }

    while idx > 0 {
        let prev = prev_char_boundary(s, idx);
        let ch = s[prev..idx].chars().next().unwrap_or(' ');
        if ch.is_whitespace() {
            break;
        }
        idx = prev;
    }

    idx
}

fn word_right_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());

    while idx < s.len() {
        let next = next_char_boundary(s, idx);
        let ch = s[idx..next].chars().next().unwrap_or(' ');
        if !ch.is_whitespace() {
            break;
        }
        idx = next;
    }

    while idx < s.len() {
        let next = next_char_boundary(s, idx);
        let ch = s[idx..next].chars().next().unwrap_or(' ');
        if ch.is_whitespace() {
            break;
        }
        idx = next;
    }

    idx
}

/// Handles ArrowLeft, ArrowRight, Home, End navigation within the rename field.
fn handle_rename_navigation(rename: &mut RenameState, key: &NamedKey, ctrl: bool, shift: bool) {
    match key {
        NamedKey::Home => {
            if shift {
                if rename.selection_anchor.is_none() {
                    rename.selection_anchor = Some(rename.cursor);
                }
            } else {
                rename.selection_anchor = None;
            }
            rename.cursor = 0;
        }
        NamedKey::End => {
            if shift {
                if rename.selection_anchor.is_none() {
                    rename.selection_anchor = Some(rename.cursor);
                }
            } else {
                rename.selection_anchor = None;
            }
            rename.cursor = rename.text.len();
        }
        NamedKey::ArrowLeft => {
            if !shift {
                if let Some((start, _)) = selected_range(rename.selection_anchor, rename.cursor) {
                    rename.cursor = start;
                } else {
                    rename.cursor = if ctrl {
                        word_left_boundary(&rename.text, rename.cursor)
                    } else {
                        prev_char_boundary(&rename.text, rename.cursor)
                    };
                }
                rename.selection_anchor = None;
            } else {
                if rename.selection_anchor.is_none() {
                    rename.selection_anchor = Some(rename.cursor);
                }
                rename.cursor = if ctrl {
                    word_left_boundary(&rename.text, rename.cursor)
                } else {
                    prev_char_boundary(&rename.text, rename.cursor)
                };
            }
        }
        NamedKey::ArrowRight => {
            if !shift {
                if let Some((_, end)) = selected_range(rename.selection_anchor, rename.cursor) {
                    rename.cursor = end;
                } else {
                    rename.cursor = if ctrl {
                        word_right_boundary(&rename.text, rename.cursor)
                    } else {
                        next_char_boundary(&rename.text, rename.cursor)
                    };
                }
                rename.selection_anchor = None;
            } else {
                if rename.selection_anchor.is_none() {
                    rename.selection_anchor = Some(rename.cursor);
                }
                rename.cursor = if ctrl {
                    word_right_boundary(&rename.text, rename.cursor)
                } else {
                    next_char_boundary(&rename.text, rename.cursor)
                };
            }
        }
        _ => {}
    }
}

/// Handles Backspace and Delete within the rename field, including selection deletion.
fn handle_rename_deletion(rename: &mut RenameState, key: &NamedKey, ctrl: bool) {
    match key {
        NamedKey::Backspace => {
            if let Some((start, end)) = selected_range(rename.selection_anchor, rename.cursor) {
                rename.text.replace_range(start..end, "");
                rename.cursor = start;
            } else {
                let start = if ctrl {
                    word_left_boundary(&rename.text, rename.cursor)
                } else {
                    prev_char_boundary(&rename.text, rename.cursor)
                };
                if start < rename.cursor {
                    rename.text.replace_range(start..rename.cursor, "");
                    rename.cursor = start;
                }
            }
            rename.selection_anchor = None;
        }
        NamedKey::Delete => {
            if let Some((start, end)) = selected_range(rename.selection_anchor, rename.cursor) {
                rename.text.replace_range(start..end, "");
                rename.cursor = start;
            } else {
                let end = if ctrl {
                    word_right_boundary(&rename.text, rename.cursor)
                } else {
                    next_char_boundary(&rename.text, rename.cursor)
                };
                if end > rename.cursor {
                    rename.text.replace_range(rename.cursor..end, "");
                }
            }
            rename.selection_anchor = None;
        }
        _ => {}
    }
}

/// Handles character input within the rename field, replacing any selection.
fn handle_rename_text_input(rename: &mut RenameState, s: &str) {
    if s.is_empty() || s.chars().any(char::is_control) {
        return;
    }
    if let Some((start, end)) = selected_range(rename.selection_anchor, rename.cursor) {
        rename.text.replace_range(start..end, "");
        rename.cursor = start;
    }
    rename.text.insert_str(rename.cursor, s);
    rename.cursor += s.len();
    rename.selection_anchor = None;
}

impl FerrumWindow {
    pub(in crate::gui::events::keyboard) fn handle_rename_input(&mut self, key: &Key) -> bool {
        enum RenameOutcome {
            Continue,
            Commit,
            Cancel,
        }

        let ctrl = self.is_action_modifier();
        let alt = self.modifiers.alt_key();
        let shift = self.modifiers.shift_key();

        // Ctrl/Alt shortcuts that should commit rename and pass through to normal handling.
        if ctrl || alt {
            let pass_through = match key {
                Key::Character(c) if ctrl && !alt && c.as_str().eq_ignore_ascii_case("a") => false,
                Key::Named(NamedKey::Tab) if ctrl => true,
                Key::Character(_) if ctrl => true,
                Key::Character(_) if alt => true,
                _ => false,
            };
            if pass_through {
                self.commit_rename();
                return false;
            }
        }

        let allow_text_input = !ctrl && !alt;
        let outcome = {
            let Some(rename) = self.renaming_tab.as_mut() else {
                return false;
            };

            match key {
                Key::Named(NamedKey::Enter) => RenameOutcome::Commit,
                Key::Named(NamedKey::Escape) => RenameOutcome::Cancel,
                Key::Character(c) if ctrl && !alt && c.as_str().eq_ignore_ascii_case("a") => {
                    rename.selection_anchor = Some(0);
                    rename.cursor = rename.text.len();
                    RenameOutcome::Continue
                }
                Key::Named(
                    nav @ (NamedKey::Home
                    | NamedKey::End
                    | NamedKey::ArrowLeft
                    | NamedKey::ArrowRight),
                ) => {
                    handle_rename_navigation(rename, nav, ctrl, shift);
                    RenameOutcome::Continue
                }
                Key::Named(del @ (NamedKey::Backspace | NamedKey::Delete)) => {
                    handle_rename_deletion(rename, del, ctrl);
                    RenameOutcome::Continue
                }
                Key::Character(c) if allow_text_input => {
                    handle_rename_text_input(rename, c.as_str());
                    RenameOutcome::Continue
                }
                _ => RenameOutcome::Continue,
            }
        };

        match outcome {
            RenameOutcome::Continue => {}
            RenameOutcome::Commit => {
                self.commit_rename();
            }
            RenameOutcome::Cancel => {
                self.cancel_rename();
            }
        }
        true
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui_events_keyboard_rename.rs"]
mod tests;
