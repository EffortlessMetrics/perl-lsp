use super::{DuplicateImport, ImportEntry, OrganizationSuggestion, SuggestionPriority};

pub(super) fn organization_suggestions(
    imports: &[ImportEntry],
    duplicate_imports: &[DuplicateImport],
) -> Vec<OrganizationSuggestion> {
    let mut suggestions = Vec::new();
    add_sort_imports_suggestion(imports, &mut suggestions);
    add_duplicate_imports_suggestion(duplicate_imports, &mut suggestions);
    add_symbol_organization_suggestion(imports, &mut suggestions);
    suggestions
}

fn add_sort_imports_suggestion(
    imports: &[ImportEntry],
    suggestions: &mut Vec<OrganizationSuggestion>,
) {
    let module_order: Vec<String> = imports.iter().map(|i| i.module.clone()).collect();
    let mut sorted_order = module_order.clone();
    sorted_order.sort();
    if module_order != sorted_order {
        suggestions.push(OrganizationSuggestion {
            description: "Sort import statements alphabetically".to_string(),
            priority: SuggestionPriority::Low,
        });
    }
}

fn add_duplicate_imports_suggestion(
    duplicate_imports: &[DuplicateImport],
    suggestions: &mut Vec<OrganizationSuggestion>,
) {
    if duplicate_imports.is_empty() {
        return;
    }

    let modules = duplicate_imports.iter().map(|d| d.module.clone()).collect::<Vec<_>>().join(", ");
    suggestions.push(OrganizationSuggestion {
        description: format!("Remove duplicate imports for modules: {}", modules),
        priority: SuggestionPriority::Medium,
    });
}

fn add_symbol_organization_suggestion(
    imports: &[ImportEntry],
    suggestions: &mut Vec<OrganizationSuggestion>,
) {
    if imports.iter().any(symbols_need_organization) {
        suggestions.push(OrganizationSuggestion {
            description: "Sort and deduplicate symbols within import statements".to_string(),
            priority: SuggestionPriority::Low,
        });
    }
}

fn symbols_need_organization(imp: &ImportEntry) -> bool {
    if imp.symbols.len() <= 1 {
        return false;
    }
    let mut sorted = imp.symbols.clone();
    sorted.sort();
    sorted.dedup();
    sorted != imp.symbols
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(module: &str, symbols: &[&str]) -> ImportEntry {
        ImportEntry {
            module: module.to_string(),
            symbols: symbols.iter().map(|s| s.to_string()).collect(),
            line: 1,
        }
    }

    fn make_dup(module: &str) -> DuplicateImport {
        DuplicateImport { module: module.to_string(), lines: vec![1, 3], can_merge: true }
    }

    #[test]
    fn test_organization_suggestions_empty_imports_no_suggestions() {
        let result = organization_suggestions(&[], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_organization_suggestions_sorted_imports_no_sort_suggestion() {
        // Already sorted alphabetically — no sort suggestion expected
        let imports = vec![make_entry("Data::Dumper", &[]), make_entry("JSON", &[])];
        let result = organization_suggestions(&imports, &[]);
        assert!(!result.iter().any(|s| s.description.contains("Sort import")));
    }

    #[test]
    fn test_organization_suggestions_unsorted_imports_triggers_sort_suggestion() {
        // "warnings" before "strict" is not alphabetical
        let imports = vec![make_entry("warnings", &[]), make_entry("strict", &[])];
        let result = organization_suggestions(&imports, &[]);
        assert!(result.iter().any(|s| s.description.contains("Sort import")));
        // The sort suggestion has Low priority
        let sort_sugg = result.iter().find(|s| s.description.contains("Sort import"));
        assert!(sort_sugg.is_some_and(|s| s.priority == SuggestionPriority::Low));
    }

    #[test]
    fn test_organization_suggestions_duplicate_imports_triggers_dedupe_suggestion() {
        let imports = vec![make_entry("JSON", &[])];
        let dups = vec![make_dup("JSON")];
        let result = organization_suggestions(&imports, &dups);
        assert!(result.iter().any(|s| s.description.contains("Remove duplicate imports")));
        let dup_sugg = result.iter().find(|s| s.description.contains("Remove duplicate imports"));
        assert!(dup_sugg.is_some_and(|s| s.priority == SuggestionPriority::Medium));
    }

    #[test]
    fn test_organization_suggestions_unsorted_symbols_triggers_symbol_suggestion() {
        // "min" before "max" is unsorted
        let imports = vec![make_entry("List::Util", &["min", "max"])];
        let result = organization_suggestions(&imports, &[]);
        assert!(result.iter().any(|s| s.description.contains("Sort and deduplicate symbols")));
    }

    #[test]
    fn test_organization_suggestions_duplicate_symbols_triggers_symbol_suggestion() {
        // "max" appears twice — dedup needed
        let imports = vec![make_entry("List::Util", &["max", "max", "min"])];
        let result = organization_suggestions(&imports, &[]);
        assert!(result.iter().any(|s| s.description.contains("Sort and deduplicate symbols")));
    }

    #[test]
    fn test_organization_suggestions_single_symbol_no_symbol_suggestion() {
        // Single symbol cannot be unsorted/deduplicated
        let imports = vec![make_entry("Carp", &["croak"])];
        let result = organization_suggestions(&imports, &[]);
        assert!(!result.iter().any(|s| s.description.contains("Sort and deduplicate symbols")));
    }

    #[test]
    fn test_organization_suggestions_no_symbols_no_symbol_suggestion() {
        let imports = vec![make_entry("strict", &[])];
        let result = organization_suggestions(&imports, &[]);
        assert!(!result.iter().any(|s| s.description.contains("Sort and deduplicate symbols")));
    }

    #[test]
    fn test_organization_suggestions_duplicate_module_name_in_description() {
        let imports = vec![make_entry("JSON", &[])];
        let dups = vec![make_dup("JSON")];
        let result = organization_suggestions(&imports, &dups);
        let dup_sugg = result.iter().find(|s| s.description.contains("Remove duplicate imports"));
        assert!(dup_sugg.is_some_and(|s| s.description.contains("JSON")));
    }
}
