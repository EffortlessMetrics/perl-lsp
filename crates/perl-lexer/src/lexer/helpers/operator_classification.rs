/// Fast lookup table for compound operator second characters.
const COMPOUND_SECOND_CHARS: &[u8] = b"=<>&|+->.~*:";

#[inline]
pub(crate) fn is_compound_operator(first: char, second: char) -> bool {
    // Optimized compound operator lookup using perfect hashing for common cases.
    // Convert to bytes for faster comparison; most operators are ASCII.
    if first.is_ascii() && second.is_ascii() {
        let first_byte = first as u8;
        let second_byte = second as u8;

        if !COMPOUND_SECOND_CHARS.contains(&second_byte) {
            return false;
        }

        // Use lookup table approach for maximum performance.
        match (first_byte, second_byte) {
            // Assignment operators.
            (b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^' | b'.', b'=') => true,

            // Comparison operators.
            (b'<' | b'>' | b'=' | b'!', b'=') => true,

            // Pattern operators.
            (b'=' | b'!', b'~') => true,

            // Increment/decrement.
            (b'+', b'+') | (b'-', b'-') => true,

            // Logical operators.
            (b'&', b'&') | (b'|', b'|') => true,

            // Shift operators.
            (b'<', b'<') | (b'>', b'>') => true,

            // Other compound operators.
            (b'*', b'*')
            | (b'/', b'/')
            | (b'-' | b'=', b'>')
            | (b'.', b'.')
            | (b'~', b'~')
            | (b':', b':') => true,

            _ => false,
        }
    } else {
        // Fallback for non-ASCII, which should be rare.
        matches!(
            (first, second),
            ('+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' | '.' | '<' | '>' | '=' | '!', '=')
                | ('=' | '!' | '~', '~')
                | ('+', '+')
                | ('-', '-' | '>')
                | ('&', '&')
                | ('|', '|')
                | ('<', '<')
                | ('>' | '=', '>')
                | ('*', '*')
                | ('/', '/')
                | ('.', '.')
                | (':', ':')
        )
    }
}
