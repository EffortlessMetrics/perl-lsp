use crate::types::Variable;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VariableCacheKind {
    Root,
    Child,
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
            .flat_map(|entry| entry.full.iter())
    }
}

pub(super) fn slice_variables(variables: &[Variable], start: usize, count: usize) -> Vec<Variable> {
    variables.iter().skip(start).take(count).cloned().collect()
}
