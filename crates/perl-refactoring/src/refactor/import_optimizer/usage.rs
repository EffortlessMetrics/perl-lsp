use std::collections::{BTreeMap, BTreeSet};

use super::{
    COMMENT_RE, DUMPER_RE, ImportEntry, MissingImport, QUALIFIED_USAGE_RE, REGEX_LITERAL_RE,
    STRING_RE, UnusedImport, get_known_module_exports, is_pragma_module,
};

pub(super) fn find_unused_imports(
    imports: &[ImportEntry],
    non_use_content: &str,
) -> Result<Vec<UnusedImport>, String> {
    let mut unused_imports = Vec::new();

    for imp in imports {
        let unused_symbols = unused_symbols_for_import(imp, non_use_content)?;
        if !unused_symbols.is_empty() {
            unused_imports.push(UnusedImport {
                module: imp.module.clone(),
                symbols: unused_symbols,
                line: imp.line,
                reason: "Symbols not used in code".to_string(),
            });
        }
    }

    Ok(unused_imports)
}

fn unused_symbols_for_import(
    imp: &ImportEntry,
    non_use_content: &str,
) -> Result<Vec<String>, String> {
    if !imp.symbols.is_empty() {
        return Ok(imp
            .symbols
            .iter()
            .filter(|sym| !contains_word(non_use_content, sym))
            .cloned()
            .collect());
    }

    if is_pragma_module(&imp.module) || bare_import_is_used(imp, non_use_content)? {
        return Ok(Vec::new());
    }

    Ok(vec!["(bare import)".to_string()])
}

fn bare_import_is_used(imp: &ImportEntry, non_use_content: &str) -> Result<bool, String> {
    let Some(known_exports) = get_known_module_exports(&imp.module) else {
        return Ok(true);
    };

    if !known_exports.is_empty() {
        return Ok(true);
    }

    if contains_word(non_use_content, &imp.module)
        || non_use_content.contains(&format!("{}::", imp.module))
    {
        return Ok(true);
    }

    if imp.module == "Data::Dumper"
        && DUMPER_RE.as_ref().map_err(|e| e.to_string())?.is_match(non_use_content)
    {
        return Ok(true);
    }

    Ok(known_exports.iter().any(|export| contains_word(non_use_content, export)))
}

pub(super) fn find_missing_imports(
    content: &str,
    imports: &[ImportEntry],
) -> Result<Vec<MissingImport>, String> {
    let imported_modules: BTreeSet<String> = imports.iter().map(|imp| imp.module.clone()).collect();
    let stripped = strip_non_code_for_usage_scan(content)?;
    let mut usage_map: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for caps in QUALIFIED_USAGE_RE.as_ref().map_err(|e| e.to_string())?.captures_iter(&stripped) {
        if let (Some(module_match), Some(symbol_match)) = (caps.get(1), caps.get(2)) {
            let module = module_match.as_str().to_string();
            if imported_modules.contains(&module) || is_pragma_module(&module) {
                continue;
            }
            usage_map.entry(module).or_default().push(symbol_match.as_str().to_string());
        }
    }

    let last_import_line = imports.iter().map(|i| i.line).max().unwrap_or(0);
    Ok(usage_map
        .into_iter()
        .map(|(module, mut symbols)| {
            symbols.sort();
            symbols.dedup();
            MissingImport {
                module,
                symbols,
                suggested_location: last_import_line + 1,
                confidence: 0.8,
            }
        })
        .collect())
}

fn strip_non_code_for_usage_scan(content: &str) -> Result<String, String> {
    let stripped =
        STRING_RE.as_ref().map_err(|e| e.to_string())?.replace_all(content, " ").to_string();
    let stripped = REGEX_LITERAL_RE
        .as_ref()
        .map_err(|e| e.to_string())?
        .replace_all(&stripped, " ")
        .to_string();
    Ok(COMMENT_RE.as_ref().map_err(|e| e.to_string())?.replace_all(&stripped, " ").to_string())
}

fn contains_word(haystack: &str, needle: &str) -> bool {
    haystack.match_indices(needle).any(|(idx, _)| {
        let before = haystack[..idx].chars().next_back();
        let after = haystack[idx + needle.len()..].chars().next();
        !is_word_char(before) && !is_word_char(after)
    })
}

fn is_word_char(ch: Option<char>) -> bool {
    ch.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(module: &str, symbols: &[&str], line: usize) -> ImportEntry {
        ImportEntry {
            module: module.to_string(),
            symbols: symbols.iter().map(|s| s.to_string()).collect(),
            line,
        }
    }

    // --- find_unused_imports tests ---

    #[test]
    fn test_find_unused_imports_empty_imports_returns_empty() -> Result<(), String> {
        let result = find_unused_imports(&[], "some code here")?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_find_unused_imports_bare_pragma_not_reported_unused() -> Result<(), String> {
        // Pragma modules (strict, warnings) are never unused
        let imports = vec![make_entry("strict", &[], 1), make_entry("warnings", &[], 2)];
        let result = find_unused_imports(&imports, "my $x = 1;")?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_find_unused_imports_used_symbol_not_reported() -> Result<(), String> {
        // "first" is used in the non-use content
        let imports = vec![make_entry("List::Util", &["first", "max"], 1)];
        let content = "my $f = first { $_ > 0 } @items;\nmy $m = max(@nums);\n";
        let result = find_unused_imports(&imports, content)?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_find_unused_imports_unused_symbol_reported() -> Result<(), String> {
        // "min" is imported but never appears in the content
        let imports = vec![make_entry("List::Util", &["max", "min"], 1)];
        let content = "my $m = max(1, 2);";
        let result = find_unused_imports(&imports, content)?;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].module, "List::Util");
        assert_eq!(result[0].symbols, vec!["min"]);
        Ok(())
    }

    #[test]
    fn test_find_unused_imports_all_symbols_used_no_report() -> Result<(), String> {
        let imports = vec![make_entry("Scalar::Util", &["blessed", "reftype"], 1)];
        let content = "my $b = blessed($obj);\nmy $r = reftype($ref);\n";
        let result = find_unused_imports(&imports, content)?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_find_unused_imports_word_boundary_prevents_false_positive() -> Result<(), String> {
        // "max" appears as part of "maximum" — must not count as used
        let imports = vec![make_entry("List::Util", &["max"], 1)];
        let content = "my $x = maximum_value();";
        let result = find_unused_imports(&imports, content)?;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].symbols, vec!["max"]);
        Ok(())
    }

    // --- find_missing_imports tests ---

    #[test]
    fn test_find_missing_imports_no_qualified_usage_returns_empty() -> Result<(), String> {
        let content = "use strict;\nmy $x = 1;\n";
        let imports = vec![make_entry("strict", &[], 1)];
        let result = find_missing_imports(content, &imports)?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_find_missing_imports_already_imported_module_not_reported() -> Result<(), String> {
        let content = "use JSON;\nmy $j = JSON::encode_json({});\n";
        let imports = vec![make_entry("JSON", &[], 1)];
        let result = find_missing_imports(content, &imports)?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_find_missing_imports_unimported_module_detected() -> Result<(), String> {
        let content = "use strict;\nmy $r = HTTP::Tiny::new();\n";
        let imports = vec![make_entry("strict", &[], 1)];
        let result = find_missing_imports(content, &imports)?;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].module, "HTTP::Tiny");
        Ok(())
    }

    #[test]
    fn test_find_missing_imports_suggested_location_after_last_import() -> Result<(), String> {
        let content = "use strict;\nuse warnings;\nmy $r = HTTP::Tiny::new();\n";
        let imports = vec![make_entry("strict", &[], 1), make_entry("warnings", &[], 2)];
        let result = find_missing_imports(content, &imports)?;
        assert_eq!(result.len(), 1);
        // suggested_location = last import line + 1 = 2 + 1 = 3
        assert_eq!(result[0].suggested_location, 3);
        Ok(())
    }

    #[test]
    fn test_find_missing_imports_qualified_usage_in_string_not_reported() -> Result<(), String> {
        // Module usage inside a string literal should not trigger a missing import
        let content = "use strict;\nmy $s = \"JSON::encode_json is a function\";\n";
        let imports = vec![make_entry("strict", &[], 1)];
        let result = find_missing_imports(content, &imports)?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_find_missing_imports_confidence_is_positive() -> Result<(), String> {
        let content = "use strict;\nmy $r = HTTP::Tiny::new();\n";
        let imports = vec![make_entry("strict", &[], 1)];
        let result = find_missing_imports(content, &imports)?;
        assert_eq!(result.len(), 1);
        assert!(result[0].confidence > 0.0);
        Ok(())
    }
}
