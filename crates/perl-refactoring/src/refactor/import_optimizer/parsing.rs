use super::{ImportEntry, USE_STATEMENT_RE};

pub(super) fn parse_imports(content: &str) -> Result<Vec<ImportEntry>, String> {
    let mut imports = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        if let Some(caps) = USE_STATEMENT_RE.as_ref().map_err(|e| e.to_string())?.captures(line) {
            let module = caps[1].to_string();
            let symbols = caps.get(2).map_or_else(Vec::new, |m| parse_symbol_list(m.as_str()));
            imports.push(ImportEntry { module, symbols, line: idx + 1 });
        }
    }
    Ok(imports)
}

fn parse_symbol_list(symbols: &str) -> Vec<String> {
    symbols
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_matches(|c| c == ',' || c == ';' || c == '"'))
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
