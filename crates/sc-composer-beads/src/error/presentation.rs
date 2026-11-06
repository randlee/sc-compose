//! Terminal-safe diagnostic and recovery text.

use std::fmt::Write as _;

/// Quote one argument for a shell-copyable recovery command.
///
/// The result is a POSIX `sh` word and needs nothing beyond `sh` and the
/// `printf` builtin. An argument without control or formatting characters is a
/// single-quoted word (`'` becomes `'"'"'`). Each control or formatting
/// character is instead spelled with octal `printf` escapes inside a
/// double-quoted command substitution, so no raw control character reaches the
/// terminal and no Bash-only `$'...'` quoting is used. The substitution also
/// covers the plain text that follows the escapes: POSIX command substitution
/// drops trailing newlines, which would otherwise lose a newline. A newline
/// that ends the argument cannot be kept that way, so it is shown as the
/// visible text `\u{000A}` instead; recovery commands never carry one.
///
/// The word is not valid in `PowerShell` (an embedded quote there is a doubled
/// `''`, and it has no `printf`) or in `cmd.exe`. On Windows, run recovery
/// commands from Git Bash or WSL.
#[must_use]
pub fn shell_quote(argument: &str) -> String {
    if !argument.chars().any(needs_shell_escape) {
        return single_quoted(argument);
    }
    let body = argument.trim_end_matches('\n');
    let trailing_newlines = argument.len() - body.len();
    let characters: Vec<char> = body.chars().collect();
    let mut quoted = String::new();
    let mut index = 0;
    while index < characters.len() {
        let plain_end = characters[index..]
            .iter()
            .position(|character| needs_shell_escape(*character))
            .map_or(characters.len(), |offset| index + offset);
        if plain_end > index {
            quoted.push_str(&single_quoted(
                &characters[index..plain_end].iter().collect::<String>(),
            ));
        }
        index = plain_end;
        if index == characters.len() {
            break;
        }
        // The escapes and the plain text up to the next escape share one
        // substitution so it never ends in a newline.
        let mut end = index;
        while end < characters.len() && needs_shell_escape(characters[end]) {
            end += 1;
        }
        while end < characters.len() && !needs_shell_escape(characters[end]) {
            end += 1;
        }
        quoted.push_str("\"$(printf '");
        for character in &characters[index..end] {
            let mut encoded = [0; 4];
            for byte in character.encode_utf8(&mut encoded).bytes() {
                // String formatting is infallible.
                let _ = write!(quoted, "\\{byte:03o}");
            }
        }
        quoted.push_str("')\"");
        index = end;
    }
    for _ in 0..trailing_newlines {
        quoted.push_str("'\\u{000A}'");
    }
    quoted
}

fn single_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\"'\"'"))
}

fn needs_shell_escape(character: char) -> bool {
    character.is_control()
        || matches!(character, '\u{2028}' | '\u{2029}')
        || is_format_character(character)
}

fn is_format_character(character: char) -> bool {
    matches!(u32::from(character),
        0x00AD | 0x0600..=0x0605 | 0x061C | 0x06DD | 0x070F | 0x0890..=0x0891 | 0x08E2
        | 0x180E | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064 | 0x2066..=0x206F
        | 0xFEFF | 0xFFF9..=0xFFFB | 0x110BD | 0x110CD | 0x13430..=0x1343F
        | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A | 0xE0001 | 0xE0020..=0xE007F)
}

/// Render terminal control and formatting characters as visible Unicode escapes.
#[must_use]
pub fn escape_human_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if needs_shell_escape(character) {
            let _ = write!(escaped, "\\u{{{:04X}}}", u32::from(character));
        } else {
            escaped.push(character);
        }
    }
    escaped
}
