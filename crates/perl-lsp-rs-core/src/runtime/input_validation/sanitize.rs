/// Sanitizes a string by removing potentially dangerous control characters.
pub fn sanitize_string(input: &str) -> String {
    input
        .chars()
        .filter(|character| {
            *character == '\t'
                || *character == '\n'
                || *character == '\r'
                || (*character >= ' ' && *character <= '~')
                || *character as u32 > 127
        })
        .collect()
}
