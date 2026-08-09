/// Intentionally triggers `unused_mut` under `-D warnings`.
pub fn clippy_failure() -> i32 {
    let mut value = 1;
    value
}
