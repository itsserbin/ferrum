use super::*;

fn mods(ctrl: bool, shift: bool, alt: bool) -> ModifiersState {
    let mut state = ModifiersState::empty();
    state.set(ModifiersState::CONTROL, ctrl);
    state.set(ModifiersState::SHIFT, shift);
    state.set(ModifiersState::ALT, alt);
    state
}

#[test]
fn ctrl_arrow_left_uses_xterm_modifier_encoding() {
    let bytes = key_to_bytes(
        &Key::Named(NamedKey::ArrowLeft),
        mods(true, false, false),
        false,
    )
    .expect("Ctrl+Left should be encoded");
    assert_eq!(bytes, b"\x1b[1;5D");
}

#[test]
fn shift_ctrl_arrow_right_encodes_combined_modifier() {
    let bytes = key_to_bytes(
        &Key::Named(NamedKey::ArrowRight),
        mods(true, true, false),
        false,
    )
    .expect("Ctrl+Shift+Right should be encoded");
    assert_eq!(bytes, b"\x1b[1;6C");
}

#[test]
fn shift_arrow_left_uses_plain_arrow_encoding() {
    let bytes = key_to_bytes(
        &Key::Named(NamedKey::ArrowLeft),
        mods(false, true, false),
        false,
    )
    .expect("Shift+Left should be encoded");
    assert_eq!(bytes, b"\x1b[1;2D");
}

#[test]
fn alt_arrow_right_moves_by_word() {
    let bytes = key_to_bytes(
        &Key::Named(NamedKey::ArrowRight),
        mods(false, false, true),
        false,
    )
    .expect("Alt+Right should be encoded");
    assert_eq!(bytes, b"\x1bf");
}

#[test]
fn alt_shift_arrow_right_uses_combined_modifier_encoding() {
    let bytes = key_to_bytes(
        &Key::Named(NamedKey::ArrowRight),
        mods(false, true, true),
        false,
    )
    .expect("Alt+Shift+Right should be encoded");
    assert_eq!(bytes, b"\x1b[1;4C");
}

#[test]
fn home_and_end_respect_cursor_mode_without_modifiers() {
    let home_normal = key_to_bytes(&Key::Named(NamedKey::Home), ModifiersState::empty(), false)
        .expect("Home should be encoded in normal mode");
    let end_normal = key_to_bytes(&Key::Named(NamedKey::End), ModifiersState::empty(), false)
        .expect("End should be encoded in normal mode");
    let home_app = key_to_bytes(&Key::Named(NamedKey::Home), ModifiersState::empty(), true)
        .expect("Home should be encoded in application mode");
    let end_app = key_to_bytes(&Key::Named(NamedKey::End), ModifiersState::empty(), true)
        .expect("End should be encoded in application mode");

    assert_eq!(home_normal, b"\x1b[H");
    assert_eq!(end_normal, b"\x1b[F");
    assert_eq!(home_app, b"\x1bOH");
    assert_eq!(end_app, b"\x1bOF");
}

#[test]
fn ctrl_home_uses_csi_modifier_form() {
    let bytes = key_to_bytes(&Key::Named(NamedKey::Home), mods(true, false, false), false)
        .expect("Ctrl+Home should be encoded");
    assert_eq!(bytes, b"\x1b[1;5H");
}

#[test]
fn ctrl_space_produces_nul() {
    let bytes = key_to_bytes(
        &Key::Named(NamedKey::Space),
        mods(true, false, false),
        false,
    )
    .expect("Ctrl+Space should be encoded");
    assert_eq!(bytes, vec![0x00]);
}

#[test]
fn ctrl_backspace_deletes_previous_word() {
    let bytes = key_to_bytes(
        &Key::Named(NamedKey::Backspace),
        mods(true, false, false),
        false,
    )
    .expect("Ctrl+Backspace should be encoded");
    assert_eq!(bytes, vec![0x17]);
}

#[test]
fn plain_backspace_is_del() {
    let bytes = key_to_bytes(
        &Key::Named(NamedKey::Backspace),
        mods(false, false, false),
        false,
    )
    .expect("Backspace should be encoded");
    assert_eq!(bytes, vec![0x7f]);
}

#[test]
fn alt_character_is_escaped() {
    let bytes = key_to_bytes(&Key::Character("f".into()), mods(false, false, true), false)
        .expect("Alt+f should be encoded");
    assert_eq!(bytes, b"\x1bf");
}
