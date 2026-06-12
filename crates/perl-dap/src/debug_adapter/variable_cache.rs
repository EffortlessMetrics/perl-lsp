use crate::types::Variable;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VariableCacheKind {
    Root,
    Child,
    /// Cached result from evaluate/setExpression/setVariable for structured expansion.
    EvaluateResult,
}

#[derive(Debug, Clone)]
struct VariableCacheEntry {
    kind: VariableCacheKind,
    full: Vec<Variable>,
    page_slices: HashMap<(usize, usize), Vec<Variable>>,
}

#[derive(Debug, Default)]
pub(super) struct VariableCache {
    entries: HashMap<i32, VariableCacheEntry>,
}

impl VariableCache {
    pub(super) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(super) fn upsert(
        &mut self,
        reference: i32,
        kind: VariableCacheKind,
        variables: Vec<Variable>,
    ) {
        let _ = self.entries.insert(
            reference,
            VariableCacheEntry { kind, full: variables, page_slices: HashMap::new() },
        );
    }

    pub(super) fn get_page(
        &mut self,
        reference: i32,
        start: usize,
        count: usize,
    ) -> Option<Vec<Variable>> {
        let entry = self.entries.get_mut(&reference)?;
        let key = (start, count);
        if let Some(cached) = entry.page_slices.get(&key) {
            return Some(cached.clone());
        }

        let page = slice_variables(&entry.full, start, count);
        let _ = entry.page_slices.insert(key, page.clone());
        Some(page)
    }

    pub(super) fn all_variables(&self) -> impl Iterator<Item = &Variable> {
        self.entries
            .values()
            .filter(|entry| entry.kind == VariableCacheKind::Root)
            .chain(self.entries.values().filter(|entry| entry.kind == VariableCacheKind::Child))
            .chain(
                self.entries
                    .values()
                    .filter(|entry| entry.kind == VariableCacheKind::EvaluateResult),
            )
            .flat_map(|entry| entry.full.iter())
    }
}

pub(super) fn slice_variables(variables: &[Variable], start: usize, count: usize) -> Vec<Variable> {
    variables.iter().skip(start).take(count).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_variable(name: &str) -> Variable {
        Variable {
            name: name.to_string(),
            value: "test".to_string(),
            type_: None,
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
        }
    }

    /// all_variables() includes EvaluateResult-kind entries.
    ///
    /// Covered changed lines: ~63-66  EvaluateResult chain in all_variables.
    #[test]
    fn all_variables_includes_evaluate_result_entries() {
        let mut cache = VariableCache::default();
        cache.upsert(1, VariableCacheKind::Root, vec![make_variable("root_var")]);
        cache.upsert(2, VariableCacheKind::EvaluateResult, vec![make_variable("eval_result")]);

        let names: Vec<&str> = cache.all_variables().map(|v| v.name.as_str()).collect();
        assert!(
            names.contains(&"root_var"),
            "all_variables must include Root entries; got {names:?}"
        );
        assert!(
            names.contains(&"eval_result"),
            "all_variables must include EvaluateResult entries; got {names:?}"
        );
    }

    /// all_variables() with only EvaluateResult entries returns those entries.
    ///
    /// Exercises the EvaluateResult chain path when Root/Child are absent.
    #[test]
    fn all_variables_evaluate_result_only() {
        let mut cache = VariableCache::default();
        cache.upsert(
            10,
            VariableCacheKind::EvaluateResult,
            vec![make_variable("x"), make_variable("y")],
        );

        let names: Vec<&str> = cache.all_variables().map(|v| v.name.as_str()).collect();
        assert_eq!(names.len(), 2, "expected 2 EvaluateResult variables; got {names:?}");
        assert!(names.contains(&"x"), "must contain 'x'");
        assert!(names.contains(&"y"), "must contain 'y'");
    }
}
