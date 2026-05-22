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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_imports_bare_pragma() -> Result<(), String> {
        let content = "use strict;\nuse warnings;\n";
        let imports = parse_imports(content)?;
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].module, "strict");
        assert!(imports[0].symbols.is_empty());
        assert_eq!(imports[0].line, 1);
        assert_eq!(imports[1].module, "warnings");
        assert_eq!(imports[1].line, 2);
        Ok(())
    }

    #[test]
    fn test_parse_imports_with_qw_symbols() -> Result<(), String> {
        let content = "use List::Util qw(first max min);\n";
        let imports = parse_imports(content)?;
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].module, "List::Util");
        assert_eq!(imports[0].symbols, vec!["first", "max", "min"]);
        Ok(())
    }

    #[test]
    fn test_parse_imports_mixed_module_name_formats() -> Result<(), String> {
        // Module names with double-colon namespacing
        let content = "use Data::Dumper;\nuse Scalar::Util qw(blessed weaken);\n";
        let imports = parse_imports(content)?;
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].module, "Data::Dumper");
        assert!(imports[0].symbols.is_empty());
        assert_eq!(imports[1].module, "Scalar::Util");
        assert_eq!(imports[1].symbols, vec!["blessed", "weaken"]);
        Ok(())
    }

    #[test]
    fn test_parse_imports_line_numbers_are_one_indexed() -> Result<(), String> {
        let content = "#!/usr/bin/perl\nuse strict;\nuse warnings;\n";
        let imports = parse_imports(content)?;
        // The shebang line is line 1, so imports start at line 2
        assert_eq!(imports[0].line, 2);
        assert_eq!(imports[1].line, 3);
        Ok(())
    }

    #[test]
    fn test_parse_imports_empty_content_returns_empty() -> Result<(), String> {
        let imports = parse_imports("")?;
        assert!(imports.is_empty());
        Ok(())
    }

    #[test]
    fn test_parse_imports_no_use_statements_returns_empty() -> Result<(), String> {
        let content = "my $x = 42;\nprint \"hello\\n\";\n";
        let imports = parse_imports(content)?;
        assert!(imports.is_empty());
        Ok(())
    }

    #[test]
    fn test_non_use_content_removes_use_lines() {
        let content = "use strict;\nmy $x = 1;\nuse warnings;\nprint $x;\n";
        let result = non_use_content(content);
        assert!(!result.contains("use strict"));
        assert!(!result.contains("use warnings"));
        assert!(result.contains("my $x = 1"));
        assert!(result.contains("print $x"));
    }

    #[test]
    fn test_non_use_content_removes_comment_lines() {
        let content = "# This is a comment\nmy $x = 1;\n# Another comment\n";
        let result = non_use_content(content);
        assert!(!result.contains("# This is a comment"));
        assert!(!result.contains("# Another comment"));
        assert!(result.contains("my $x = 1"));
    }

    #[test]
    fn test_non_use_content_preserves_indented_use_in_code() {
        // A 'use' that appears mid-line (not as trim_start prefix) is kept
        let content = "my $s = \"use something\";\nuse strict;\n";
        let result = non_use_content(content);
        // The string line does NOT start with "use " so it is kept
        assert!(result.contains("my $s"));
        // The actual use statement is removed
        assert!(!result.contains("use strict"));
    }

    #[test]
    fn test_non_use_content_empty_input_returns_empty() {
        let result = non_use_content("");
        assert!(result.is_empty());
    }
}
