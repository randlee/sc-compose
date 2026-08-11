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
}

impl ShaError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidUtf8 => "SC_SHA_INVALID_UTF8",
        }
    }
}

impl Display for ShaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUtf8 => f.write_str("text-file bytes are not valid UTF-8"),
        }
    }
}

impl std::error::Error for ShaError {}

/// A SHA-256 digest over normalized UTF-8 template text.
#[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TemplateSha256([u8; 32]);

impl TemplateSha256 {
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

    use super::{HashInput, ShaError, calculate_hash, normalize_newlines};

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
    }

    #[test]
    fn avoids_allocating_when_text_has_no_carriage_return() {
        assert!(matches!(normalize_newlines("plain text"), Cow::Borrowed(_)));
        assert!(matches!(normalize_newlines("plain\rtext"), Cow::Owned(_)));
    }
}
