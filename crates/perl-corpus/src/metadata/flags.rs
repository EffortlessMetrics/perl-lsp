pub fn normalize_flags(raw: &str) -> Vec<String> {
    raw.replace(',', " ").split_whitespace().map(str::to_string).collect()
}
