use crate::gui::*;

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

impl FerrumWindow {
    pub(in crate::gui::events::keyboard) fn handle_rename_input(&mut self, key: &Key) -> bool {
        enum RenameOutcome {
            Continue,
            Commit,
            Cancel,
        }

        let ctrl = self.modifiers.control_key();
        let alt = self.modifiers.alt_key();
        let shift = self.modifiers.shift_key();

        // Ctrl/Alt shortcuts that should commit rename and pass through to normal handling.
        if ctrl || alt {
            let pass_through = match key {
                // Ctrl+A (select all in rename) is handled below, NOT passed through.
                Key::Character(c) if ctrl && !alt && c.as_str().eq_ignore_ascii_case("a") => false,
                // Tab shortcuts: Ctrl+Tab, Ctrl+Shift+Tab.
                Key::Named(NamedKey::Tab) if ctrl => true,
                // Ctrl+T (new tab), Ctrl+W (close tab), Ctrl+digit (switch tab),
                // Ctrl+Shift+T (reopen tab).
                Key::Character(_) if ctrl => true,
                // Alt+digit (switch tab).
                Key::Character(_) if alt => true,
                _ => false,
            };
            if pass_through {
                self.commit_rename();
                return false; // Let the shortcut handler process this key.
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
                Key::Named(NamedKey::Home) => {
                    if shift {
                        if rename.selection_anchor.is_none() {
                            rename.selection_anchor = Some(rename.cursor);
                        }
                    } else {
                        rename.selection_anchor = None;
                    }
                    rename.cursor = 0;
                    RenameOutcome::Continue
                }
                Key::Named(NamedKey::End) => {
                    if shift {
                        if rename.selection_anchor.is_none() {
                            rename.selection_anchor = Some(rename.cursor);
                        }
                    } else {
                        rename.selection_anchor = None;
                    }
                    rename.cursor = rename.text.len();
                    RenameOutcome::Continue
                }
                Key::Named(NamedKey::ArrowLeft) => {
                    if !shift {
                        if let Some((start, _)) =
                            selected_range(rename.selection_anchor, rename.cursor)
                        {
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
                    RenameOutcome::Continue
                }
                Key::Named(NamedKey::ArrowRight) => {
                    if !shift {
                        if let Some((_, end)) =
                            selected_range(rename.selection_anchor, rename.cursor)
                        {
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
                    RenameOutcome::Continue
                }
                Key::Named(NamedKey::Backspace) => {
                    if let Some((start, end)) =
                        selected_range(rename.selection_anchor, rename.cursor)
                    {
                        rename.text.replace_range(start..end, "");
                        rename.cursor = start;
                        rename.selection_anchor = None;
                        RenameOutcome::Continue
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
                        rename.selection_anchor = None;
                        RenameOutcome::Continue
                    }
                }
                Key::Named(NamedKey::Delete) => {
                    if let Some((start, end)) =
                        selected_range(rename.selection_anchor, rename.cursor)
                    {
                        rename.text.replace_range(start..end, "");
                        rename.cursor = start;
                        rename.selection_anchor = None;
                        RenameOutcome::Continue
                    } else {
                        let end = if ctrl {
                            word_right_boundary(&rename.text, rename.cursor)
                        } else {
                            next_char_boundary(&rename.text, rename.cursor)
                        };
                        if end > rename.cursor {
                            rename.text.replace_range(rename.cursor..end, "");
                        }
                        rename.selection_anchor = None;
                        RenameOutcome::Continue
                    }
                }
                Key::Character(c) if allow_text_input => {
                    let s = c.as_str();
                    if s.is_empty() || s.chars().any(char::is_control) {
                        RenameOutcome::Continue
                    } else {
                        if let Some((start, end)) =
                            selected_range(rename.selection_anchor, rename.cursor)
                        {
                            rename.text.replace_range(start..end, "");
                            rename.cursor = start;
                        }
                        rename.text.insert_str(rename.cursor, s);
                        rename.cursor += s.len();
                        rename.selection_anchor = None;
                        RenameOutcome::Continue
                    }
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
