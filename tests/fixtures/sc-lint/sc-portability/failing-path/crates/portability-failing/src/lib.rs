//! Production code remains platform-neutral; the test intentionally exposes a
//! hardcoded Unix path so the analyzer's finding stays observable.

pub fn fixture_name() -> &'static str {
    "portability-failing"
}

#[cfg(test)]
mod tests {
    #[test]
    fn uses_a_platform_specific_path() {
        let _path = std::path::PathBuf::from("/tmp/sc-compose-portability");
    }
}
