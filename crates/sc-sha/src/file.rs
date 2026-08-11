//! Per-content text hashing.

use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use sha2::{Digest, Sha256};

/// Input to [`crate::calculate_hash`].
#[derive(Debug, Copy, Clone)]
pub enum HashInput<'a> {
    /// Strict UTF-8 bytes read by the caller from a text file.
    TextFileBytes { utf8_file_bytes: &'a [u8] },
}

/// The typed result of a per-content hash operation.
#[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum HashResult {
    /// The normalized text-file identity.
    Template(TemplateSha256),
}

impl HashResult {
    /// Borrow the template identity when this result is a template hash.
    #[must_use]
    pub const fn template(&self) -> &TemplateSha256 {
        match self {
            Self::Template(hash) => hash,
        }
    }
}

/// Errors returned by [`crate::calculate_hash`].
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ShaError {
    /// The supplied bytes are not valid UTF-8.
    InvalidUtf8,
    /// A supplied digest is not exactly 32 bytes represented as lowercase or
    /// uppercase hexadecimal.
    InvalidDigestHex,
}

impl ShaError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidUtf8 => "SC_SHA_INVALID_UTF8",
            Self::InvalidDigestHex => "SC_SHA_INVALID_DIGEST",
        }
    }
}

impl Display for ShaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUtf8 => f.write_str(
                "text-file bytes are not valid UTF-8; encode the source as UTF-8 before hashing",
            ),
            Self::InvalidDigestHex => {
                f.write_str("SHA-256 digest must contain exactly 64 hexadecimal characters")
            }
        }
    }
}

impl std::error::Error for ShaError {}

/// A SHA-256 digest over normalized UTF-8 template text.
#[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TemplateSha256([u8; 32]);

impl TemplateSha256 {
    /// Construct a digest from its canonical raw 32-byte representation.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the raw 32-byte digest.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Render the digest as lowercase hexadecimal.
    #[must_use]
    pub fn to_hex(self) -> String {
        self.to_string()
    }

    /// Parse a canonical 64-character hexadecimal SHA-256 digest.
    ///
    /// # Errors
    ///
    /// Returns [`ShaError::InvalidDigestHex`] when the input is not exactly
    /// 64 hexadecimal characters.
    pub fn from_hex(value: &str) -> Result<Self, ShaError> {
        if value.len() != 64 {
            return Err(ShaError::InvalidDigestHex);
        }

        let mut bytes = [0; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            let high = hex_nibble(pair[0]).ok_or(ShaError::InvalidDigestHex)?;
            let low = hex_nibble(pair[1]).ok_or(ShaError::InvalidDigestHex)?;
            bytes[index] = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
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

/// Hash strict-UTF-8 text after applying universal newline normalization.
///
/// This matches the verified `synaptic-canvas-dolt` behavior at commit
/// `787f0507fa99be999c0a40d21b273cee05093a1f`: `read_text(encoding="utf-8")`
/// followed by `sha256(content.encode("utf-8"))`. CRLF and bare CR become LF;
/// all other Unicode, including a decoded BOM, is preserved.
///
/// # Errors
///
/// Returns [`ShaError::InvalidUtf8`] when the supplied bytes are not strict
/// UTF-8.
pub fn calculate_hash(input: HashInput<'_>) -> Result<HashResult, ShaError> {
    let HashInput::TextFileBytes { utf8_file_bytes } = input;
    let text = std::str::from_utf8(utf8_file_bytes).map_err(|_error| ShaError::InvalidUtf8)?;
    let normalized = normalize_newlines(text);
    let digest = Sha256::digest(normalized.as_bytes());
    let mut bytes = [0; 32];
    bytes.copy_from_slice(&digest);
    Ok(HashResult::Template(TemplateSha256(bytes)))
}

fn normalize_newlines(text: &str) -> Cow<'_, str> {
    if !text.as_bytes().contains(&b'\r') {
        return Cow::Borrowed(text);
    }

    let mut normalized = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\r' {
            if characters.peek() == Some(&'\n') {
                characters.next();
            }
            normalized.push('\n');
        } else {
            normalized.push(character);
        }
    }
    Cow::Owned(normalized)
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{HashInput, ShaError, TemplateSha256, calculate_hash, normalize_newlines};

    #[test]
    fn normalizes_crlf_and_bare_cr() {
        let lf = calculate_hash(HashInput::TextFileBytes {
            utf8_file_bytes: b"a\nb\n",
        })
        .expect("LF hash")
        .template()
        .to_owned();
        let crlf = calculate_hash(HashInput::TextFileBytes {
            utf8_file_bytes: b"a\r\nb\r\n",
        })
        .expect("CRLF hash")
        .template()
        .to_owned();
        let cr = calculate_hash(HashInput::TextFileBytes {
            utf8_file_bytes: b"a\rb\r",
        })
        .expect("CR hash")
        .template()
        .to_owned();
        assert_eq!(lf, crlf);
        assert_eq!(lf, cr);
    }

    #[test]
    fn rejects_invalid_utf8() {
        let error = calculate_hash(HashInput::TextFileBytes {
            utf8_file_bytes: &[0xff],
        })
        .expect_err("invalid UTF-8 must fail");
        assert_eq!(error, ShaError::InvalidUtf8);
        assert_eq!(error.code(), "SC_SHA_INVALID_UTF8");
        assert!(error.to_string().contains("encode the source as UTF-8"));
    }

    #[test]
    fn avoids_allocating_when_text_has_no_carriage_return() {
        assert!(matches!(normalize_newlines("plain text"), Cow::Borrowed(_)));
        assert!(matches!(normalize_newlines("plain\rtext"), Cow::Owned(_)));
    }

    #[test]
    fn parses_and_rejects_hex_digests() {
        let digest = TemplateSha256::from_hex(
            "5891B5B522D5DF086D0FF0B110FBD9D21BB4FC7163AF34D08286A2E846F6BE03",
        )
        .expect("valid digest");
        assert_eq!(
            digest.to_hex(),
            "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
        );
        assert_eq!(
            TemplateSha256::from_hex("not-a-digest").expect_err("invalid digest"),
            ShaError::InvalidDigestHex
        );
    }
}
