use std::collections::BTreeMap;

use super::{DuplicateImport, ImportEntry};

pub(super) fn find_duplicate_imports(imports: &[ImportEntry]) -> Vec<DuplicateImport> {
    let mut module_to_lines: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for imp in imports {
        module_to_lines.entry(imp.module.clone()).or_default().push(imp.line);
    }

    module_to_lines
        .iter()
        .filter(|(_, lines)| lines.len() > 1)
        .map(|(module, lines)| DuplicateImport {
            module: module.clone(),
            lines: lines.clone(),
            can_merge: true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(module: &str, line: usize) -> ImportEntry {
        ImportEntry { module: module.to_string(), symbols: vec![], line }
    }

    #[test]
    fn test_find_duplicate_imports_empty_returns_empty() {
        let result = find_duplicate_imports(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_duplicate_imports_no_duplicates_returns_empty() {
        let imports =
            vec![make_entry("strict", 1), make_entry("warnings", 2), make_entry("Data::Dumper", 3)];
        let result = find_duplicate_imports(&imports);
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_duplicate_imports_single_duplicate_detected() {
        let imports = vec![
            make_entry("strict", 1),
            make_entry("Data::Dumper", 2),
            make_entry("Data::Dumper", 5),
        ];
        let result = find_duplicate_imports(&imports);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].module, "Data::Dumper");
        assert_eq!(result[0].lines.len(), 2);
        assert!(result[0].lines.contains(&2));
        assert!(result[0].lines.contains(&5));
    }

    #[test]
    fn test_find_duplicate_imports_can_merge_is_always_true() {
        let imports = vec![make_entry("JSON", 1), make_entry("JSON", 4)];
        let result = find_duplicate_imports(&imports);
        assert_eq!(result.len(), 1);
        assert!(result[0].can_merge);
    }

    #[test]
    fn test_find_duplicate_imports_multiple_duplicates() {
        let imports = vec![
            make_entry("strict", 1),
            make_entry("JSON", 2),
            make_entry("JSON", 3),
            make_entry("YAML", 4),
            make_entry("YAML", 5),
            make_entry("YAML", 6),
        ];
        let result = find_duplicate_imports(&imports);
        assert_eq!(result.len(), 2);
        let json_dup = result.iter().find(|d| d.module == "JSON");
        assert!(json_dup.is_some());
        let yaml_dup = result.iter().find(|d| d.module == "YAML");
        assert!(yaml_dup.is_some_and(|d| d.lines.len() == 3));
    }
}
