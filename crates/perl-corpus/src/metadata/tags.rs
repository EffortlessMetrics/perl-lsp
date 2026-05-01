pub(crate) fn normalize_tags(value: Option<&String>) -> Vec<String> {
    value
        .map(|s| {
            s.replace(',', " ").split_whitespace().map(|t| t.to_lowercase()).collect::<Vec<_>>()
        })
        .unwrap_or_default()
}
