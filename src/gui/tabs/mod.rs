pub(in crate::gui) mod create;
mod manage;
mod pty_reader;

pub(in crate::gui) fn normalized_active_index_after_remove(
    active: usize,
    len_before: usize,
    removed_index: usize,
) -> Option<usize> {
    if len_before == 0 || removed_index >= len_before {
        return None;
    }

    let len_after = len_before - 1;
    if len_after == 0 {
        return None;
    }

    let next_active = if active > removed_index {
        active.saturating_sub(1)
    } else {
        active
    };

    Some(next_active.min(len_after - 1))
}

#[cfg(test)]
#[path = "../../../tests/unit/gui_tabs.rs"]
mod tests;
