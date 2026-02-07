//! Character classification utilities shared between email and URL scanning.

/// Check if a character is valid in an email local-part (before @).
/// Based on RFC 5321 "Atom" / RFC 5322 "atext", plus RFC 6531 for internationalization.
pub(crate) fn is_email_local_char(c: char) -> bool {
    match c {
        'a'..='z'
        | 'A'..='Z'
        | '0'..='9'
        | '!'
        | '#'
        | '$'
        | '%'
        | '&'
        | '\''
        | '*'
        | '+'
        | '-'
        | '/'
        | '='
        | '?'
        | '^'
        | '_'
        | '`'
        | '{'
        | '|'
        | '}'
        | '~' => true,
        // Allow international characters (RFC 6531) but exclude Unicode whitespace
        // (e.g., NBSP, EM SPACE, IDEOGRAPHIC SPACE) which should act as word boundaries
        _ => c >= '\u{80}' && !c.is_whitespace(),
    }
}
