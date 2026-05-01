pub(super) fn parse_error_fix_code_from_message(message: &str) -> Option<&'static str> {
    let message_lower = message.to_ascii_lowercase();
    if message_lower.contains("missing semicolon") {
        return Some("parse-error-missingsemicolon");
    }
    if message_lower.contains("unclosed string") || message_lower.contains("unterminated string") {
        return Some("parse-error-unclosedstring");
    }
    if message_lower.contains("unclosed parenthesis") {
        return Some("parse-error-unclosedparen");
    }
    if message_lower.contains("unclosed brace")
        || message_lower.contains("missing '}'")
        || message_lower.contains("unclosed `{`")
    {
        return Some("parse-error-unclosedbrace");
    }
    None
}
