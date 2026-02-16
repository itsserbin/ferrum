use crate::gui::*;

fn csi_modifier_param(modifiers: ModifiersState) -> Option<u8> {
    let mut param = 1;
    let mut has_modifier = false;

    if modifiers.shift_key() {
        param += 1;
        has_modifier = true;
    }
    if modifiers.alt_key() {
        param += 2;
        has_modifier = true;
    }
    if modifiers.control_key() {
        param += 4;
        has_modifier = true;
    }

    has_modifier.then_some(param)
}

fn with_alt_prefix(mut bytes: Vec<u8>, modifiers: ModifiersState) -> Vec<u8> {
    if modifiers.alt_key() {
        let mut prefixed = Vec::with_capacity(bytes.len() + 1);
        prefixed.push(0x1b);
        prefixed.append(&mut bytes);
        prefixed
    } else {
        bytes
    }
}

fn ctrl_char_code(ch: char) -> Option<u8> {
    let ch = ch.to_ascii_lowercase();
    if ch.is_ascii_lowercase() {
        return Some(ch as u8 - b'a' + 1);
    }

    match ch {
        ' ' | '@' => Some(0x00),
        '[' => Some(0x1b),
        '\\' => Some(0x1c),
        ']' => Some(0x1d),
        '^' => Some(0x1e),
        '_' => Some(0x1f),
        '?' => Some(0x7f),
        _ => None,
    }
}

fn csi_with_modifier(final_byte: char, modifier_param: u8) -> Vec<u8> {
    format!("\x1b[1;{}{}", modifier_param, final_byte).into_bytes()
}

fn csi_tilde(code: u8, modifier_param: Option<u8>) -> Vec<u8> {
    match modifier_param {
        Some(param) => format!("\x1b[{};{}~", code, param).into_bytes(),
        None => format!("\x1b[{}~", code).into_bytes(),
    }
}

fn encode_arrow_key(final_byte: char, decckm: bool, modifier_param: Option<u8>) -> Vec<u8> {
    if let Some(param) = modifier_param {
        return csi_with_modifier(final_byte, param);
    }

    if decckm {
        format!("\x1bO{}", final_byte).into_bytes()
    } else {
        format!("\x1b[{}", final_byte).into_bytes()
    }
}

fn encode_home_end_key(final_byte: char, decckm: bool, modifier_param: Option<u8>) -> Vec<u8> {
    if let Some(param) = modifier_param {
        return csi_with_modifier(final_byte, param);
    }

    if decckm {
        format!("\x1bO{}", final_byte).into_bytes()
    } else {
        format!("\x1b[{}", final_byte).into_bytes()
    }
}

/// Encodes mouse events as SGR (?1006) or legacy X10 bytes.
pub(super) fn encode_mouse_event(
    button: u8,
    col: usize,
    row: usize,
    pressed: bool,
    sgr: bool,
) -> Vec<u8> {
    if sgr {
        // SGR: \x1b[<button;col;rowM (press) / ...m (release)
        let suffix = if pressed { 'M' } else { 'm' };
        format!("\x1b[<{};{};{}{}", button, col + 1, row + 1, suffix).into_bytes()
    } else {
        // Legacy X10: \x1b[M{cb}{cx}{cy} with single-byte coordinates.
        let cb = if pressed { button + 32 } else { 3 + 32 }; // Release is encoded as button 3.
        let cx = (col as u8).saturating_add(33);
        let cy = (row as u8).saturating_add(33);
        vec![0x1b, b'[', b'M', cb, cx, cy]
    }
}

/// Converts logical key input into PTY byte sequences.
pub(super) fn key_to_bytes(key: &Key, modifiers: ModifiersState, decckm: bool) -> Option<Vec<u8>> {
    match key {
        Key::Character(c) => {
            let ch = c.chars().next()?;
            let mut bytes = if modifiers.control_key() {
                match ctrl_char_code(ch) {
                    Some(code) => vec![code],
                    None => c.as_bytes().to_vec(),
                }
            } else {
                c.as_bytes().to_vec()
            };

            if modifiers.alt_key() {
                let mut prefixed = Vec::with_capacity(bytes.len() + 1);
                prefixed.push(0x1b);
                prefixed.append(&mut bytes);
                return Some(prefixed);
            }

            Some(bytes)
        }
        Key::Named(named) => {
            let modifier_param = csi_modifier_param(modifiers);

            match named {
                NamedKey::Enter => Some(with_alt_prefix(vec![b'\r'], modifiers)),
                NamedKey::Backspace => {
                    let byte = if modifiers.control_key() { 0x17 } else { 0x7f };
                    Some(with_alt_prefix(vec![byte], modifiers))
                }
                NamedKey::Tab => {
                    if modifiers.shift_key() && !modifiers.control_key() && !modifiers.alt_key() {
                        Some(b"\x1b[Z".to_vec()) // Back-tab
                    } else {
                        Some(with_alt_prefix(vec![b'\t'], modifiers))
                    }
                }
                NamedKey::Space => {
                    let byte = if modifiers.control_key() { 0x00 } else { b' ' };
                    Some(with_alt_prefix(vec![byte], modifiers))
                }
                NamedKey::Escape => Some(vec![0x1b]),
                NamedKey::ArrowUp => Some(encode_arrow_key('A', decckm, modifier_param)),
                NamedKey::ArrowDown => Some(encode_arrow_key('B', decckm, modifier_param)),
                NamedKey::ArrowRight => {
                    if modifiers.alt_key() && modifiers.shift_key() && !modifiers.control_key() {
                        Some(encode_arrow_key('C', decckm, Some(4)))
                    } else if modifiers.alt_key() && !modifiers.control_key() {
                        Some(b"\x1bf".to_vec()) // Meta+f — forward word
                    } else if modifiers.shift_key()
                        && !modifiers.control_key()
                        && !modifiers.alt_key()
                    {
                        Some(encode_arrow_key('C', decckm, Some(2)))
                    } else {
                        Some(encode_arrow_key('C', decckm, modifier_param))
                    }
                }
                NamedKey::ArrowLeft => {
                    if modifiers.alt_key() && modifiers.shift_key() && !modifiers.control_key() {
                        Some(encode_arrow_key('D', decckm, Some(4)))
                    } else if modifiers.alt_key() && !modifiers.control_key() {
                        Some(b"\x1bb".to_vec()) // Meta+b — backward word
                    } else if modifiers.shift_key()
                        && !modifiers.control_key()
                        && !modifiers.alt_key()
                    {
                        Some(encode_arrow_key('D', decckm, Some(2)))
                    } else {
                        Some(encode_arrow_key('D', decckm, modifier_param))
                    }
                }
                NamedKey::Home => Some(encode_home_end_key('H', decckm, modifier_param)),
                NamedKey::End => Some(encode_home_end_key('F', decckm, modifier_param)),
                NamedKey::Insert => Some(csi_tilde(2, modifier_param)),
                NamedKey::Delete => Some(csi_tilde(3, modifier_param)),
                NamedKey::PageUp => Some(csi_tilde(5, modifier_param)),
                NamedKey::PageDown => Some(csi_tilde(6, modifier_param)),
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/gui_input.rs"]
mod tests;
