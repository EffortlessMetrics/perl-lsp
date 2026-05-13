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
