use super::*;

#[test]
fn selected_range_returns_ordered_bounds() {
    assert_eq!(selected_range(None, 3), None);
    assert_eq!(selected_range(Some(2), 2), None);
    assert_eq!(selected_range(Some(5), 2), Some((2, 5)));
}

#[test]
fn char_boundaries_handle_utf8() {
    let s = "aб🙂z";
    let z = s.len();
    let smile = prev_char_boundary(s, z);
    let b = prev_char_boundary(s, smile);

    assert_eq!(&s[smile..z], "z");
    assert_eq!(next_char_boundary(s, b), smile);
}

#[test]
fn word_boundaries_skip_surrounding_whitespace() {
    let s = "one   two three";
    let idx = s.find("two").unwrap_or(0);
    let right = word_right_boundary(s, idx);
    let left = word_left_boundary(s, right);

    assert_eq!(&s[idx..right], "two");
    assert_eq!(left, idx);
}
