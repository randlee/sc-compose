//! This fixture deliberately emits a Clippy warning under `-D warnings`.

pub fn has_entries(items: &[u8]) -> bool {
    items.len() > 0
}
