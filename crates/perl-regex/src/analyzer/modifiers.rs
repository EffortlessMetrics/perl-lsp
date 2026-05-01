pub(crate) fn describe_modifier(modifier: char) -> Option<&'static str> {
    match modifier {
        'i' => Some("case-insensitive matching"),
        'm' => Some("multiline mode: ^ and $ match line boundaries"),
        's' => Some("single-line mode: dot matches newline"),
        'x' => Some("extended mode: whitespace and comments allowed"),
        'g' => Some("global: match all occurrences"),
        'a' => Some("ASCII-safe character classes"),
        'd' => Some("native platform character set semantics"),
        'l' => Some("locale-dependent character semantics"),
        'u' => Some("Unicode character semantics"),
        'n' => Some("non-capturing by default for unnamed groups"),
        'p' => Some("preserve string for ${^PREMATCH}, ${^MATCH}, ${^POSTMATCH}"),
        'r' => Some("non-destructive substitution result"),
        'c' => Some("keep current match position for /g scans"),
        'o' => Some("compile pattern only once"),
        'e' => Some("evaluate replacement as code in substitutions"),
        _ => None,
    }
}
