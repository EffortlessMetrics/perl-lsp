use super::{ImportEntry, USE_STATEMENT_RE};

pub(super) fn parse_imports(content: &str) -> Result<Vec<ImportEntry>, String> {
    let mut imports = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        if let Some(caps) = USE_STATEMENT_RE.as_ref().map_err(|e| e.to_string())?.captures(line) {
            let Some(module_match) = caps.get(1) else {
                continue;
            };
            let module = module_match.as_str().to_string();
            let symbols = symbol_capture(&caps).map_or_else(Vec::new, parse_symbol_list);
            imports.push(ImportEntry { module, symbols, line: idx + 1 });
        }
    }
    Ok(imports)
}

fn symbol_capture<'a>(caps: &'a regex::Captures<'a>) -> Option<&'a str> {
    (2..=6).find_map(|idx| caps.get(idx).map(|m| m.as_str()))
}

fn parse_symbol_list(symbols: &str) -> Vec<String> {
    symbols
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_matches(|c| c == ',' || c == ';' || c == '"' || c == '\''))
        .map(str::to_string)
        .collect()
}

pub(super) fn non_use_content(content: &str) -> String {
    content
        .lines()
        .filter(|line| {
            !line.trim_start().starts_with("use ") && !line.trim_start().starts_with('#')
        })
        .collect::<Vec<_>>()
        .join("\n")
}
