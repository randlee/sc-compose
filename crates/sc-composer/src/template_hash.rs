//! Raw-byte SHA-256 template identity compatible with `synaptic-canvas-dolt`.
//!
//! The hash covers the complete source-file byte sequence. It intentionally
//! preserves byte-order marks, line endings, and the presence or absence of a
//! final newline.

use std::fmt::{Display, Formatter};

use sha2::{Digest, Sha256};

/// A SHA-256 digest over an unmodified template source file.
///
/// This is the template identity format used by `synaptic-canvas-dolt`: 32 raw
/// digest bytes represented as lowercase hexadecimal when displayed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TemplateSha256([u8; 32]);

impl TemplateSha256 {
    /// Borrow the raw 32-byte SHA-256 digest.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Render this digest as lowercase hexadecimal.
    #[must_use]
    pub fn to_hex(self) -> String {
        self.to_string()
    }
}

impl Display for TemplateSha256 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Hash complete template source bytes with the Dolt-compatible SHA-256 form.
///
/// The input is consumed byte-for-byte. In particular, this function performs
/// no UTF-8 decoding, line-ending normalization, BOM removal, or final-newline
/// adjustment.
#[must_use]
pub fn template_sha256(raw_file_bytes: &[u8]) -> TemplateSha256 {
    let digest = Sha256::digest(raw_file_bytes);
    let mut bytes = [0; 32];
    bytes.copy_from_slice(&digest);
    TemplateSha256(bytes)
}

#[cfg(test)]
mod tests {
    use super::template_sha256;

    #[test]
    fn matches_sha256_golden_vectors_without_normalizing_raw_bytes() {
        let vectors = [
            (
                b"".as_slice(),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                b"hello\n".as_slice(),
                "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03",
            ),
            (
                b"hello\r\n".as_slice(),
                "cd2eca3535741f27a8ae40c31b0c41d4057a7a7b912b33b9aed86485d1c84676",
            ),
            (
                b"\xEF\xBB\xBFhello".as_slice(),
                "7489ebbcc2a00056ddaaaac190bce473e5c03696ea1bd8ed83cf59a174283862",
            ),
            (
                b"hello".as_slice(),
                "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            ),
        ];

        for (raw_bytes, expected) in vectors {
            assert_eq!(template_sha256(raw_bytes).to_hex(), expected);
        }
    }

    #[test]
    fn line_endings_bom_and_final_newline_change_template_identity() {
        let lf = template_sha256(b"hello\n");
        let crlf = template_sha256(b"hello\r\n");
        let bom = template_sha256(b"\xEF\xBB\xBFhello");
        let no_newline = template_sha256(b"hello");

        assert_ne!(lf, crlf);
        assert_ne!(lf, bom);
        assert_ne!(lf, no_newline);
    }
}
