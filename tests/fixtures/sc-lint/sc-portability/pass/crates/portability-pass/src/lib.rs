//! A workspace fixture that uses platform-neutral path construction.

use std::path::PathBuf;

pub fn temporary_path() -> PathBuf {
    std::env::temp_dir().join("sc-compose-portability")
}
