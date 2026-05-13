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
