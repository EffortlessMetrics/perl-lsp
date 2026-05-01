pub fn normalize_tags(raw: &str) -> Vec<String> {
    raw.replace(',', " ").split_whitespace().map(str::to_lowercase).collect()
}
