pub(crate) fn normalize_flags(value: Option<&String>) -> Vec<String> {
    value
        .map(|s| {
            s.replace(',', " ")
                .split_whitespace()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}
