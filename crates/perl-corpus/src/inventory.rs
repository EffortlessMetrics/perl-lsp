use crate::files::{CorpusPaths, get_test_files_from};
use crate::lint::KNOWN_TAGS;
use crate::meta::Section;
use crate::metadata::IdSource;
use crate::parse_file;
use anyhow::Result;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// Inventory summary schema for the current perl-corpus state.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CorpusInventory {
    pub schema_version: u32,
    pub files: usize,
    pub sections: usize,
    pub cases: usize,
    pub ids: InventoryIds,
    pub tags: InventoryTags,
    pub flags: BTreeMap<String, usize>,
    pub markers: InventoryMarkers,
    pub generators: Vec<String>,
    pub concept_mapping_available: bool,
    pub expectations_available: bool,
    pub fixtures_without_concepts: Vec<String>,
    pub fixtures_without_expectations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InventoryIds {
    pub explicit: usize,
    pub generated: usize,
    pub missing_explicit_ids: usize,
    pub duplicate_effective_ids: Vec<String>,
    pub duplicate_explicit_ids: Vec<String>,
    pub id_source_breakdown: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InventoryTags {
    pub known: Vec<String>,
    pub unknown: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InventoryMarkers {
    pub expected_error: usize,
    pub wip: usize,
    pub parser_sensitive: usize,
}

/// Build an inventory using the discovered workspace corpus paths.
pub fn build_inventory() -> Result<CorpusInventory> {
    build_inventory_from_paths(&CorpusPaths::discover())
}

/// Build an inventory for an explicit workspace root.
pub fn build_inventory_from_paths(paths: &CorpusPaths) -> Result<CorpusInventory> {
    let files = get_test_files_from(paths);
    let mut sections = Vec::new();
    for file in &files {
        sections.extend(parse_file(file)?);
    }

    let mut inventory = inventory_from_sections(files.len(), &sections);
    let gold_root = paths.root.join("test_corpus/gold");
    populate_fixture_coverage(&gold_root, &mut inventory)?;
    Ok(inventory)
}

/// Build an inventory from explicit section data.
pub fn inventory_from_sections(file_count: usize, sections: &[Section]) -> CorpusInventory {
    let mut id_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut explicit = 0usize;
    let mut generated = 0usize;
    let mut missing_explicit_ids = 0usize;
    let mut explicit_id_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut id_source_breakdown: BTreeMap<String, usize> = BTreeMap::new();
    let mut known_tags = BTreeSet::new();
    let mut unknown_tags = BTreeSet::new();
    let known_tag_set: BTreeSet<&str> = KNOWN_TAGS.iter().copied().collect();
    let mut flags = BTreeMap::new();
    let mut markers = InventoryMarkers { expected_error: 0, wip: 0, parser_sensitive: 0 };

    for section in sections {
        *id_counts.entry(section.id.as_str()).or_default() += 1;
        match section.id_source {
            IdSource::Explicit => {
                explicit += 1;
                *id_source_breakdown.entry("explicit".to_string()).or_default() += 1;
                if let Some(explicit_id) = section.explicit_id.as_deref() {
                    *explicit_id_counts.entry(explicit_id).or_default() += 1;
                }
            }
            IdSource::Generated => {
                generated += 1;
                missing_explicit_ids += 1;
                *id_source_breakdown.entry("generated".to_string()).or_default() += 1;
            }
        }

        for tag in &section.tags {
            if known_tag_set.contains(tag.as_str()) {
                known_tags.insert(tag.clone());
            } else {
                unknown_tags.insert(tag.clone());
            }
        }

        for flag in &section.flags {
            *flags.entry(flag.clone()).or_default() += 1;
        }

        if section.has_flag("expected-error") {
            markers.expected_error += 1;
        }
        if section.has_flag("wip") || section.has_flag("todo") {
            markers.wip += 1;
        }
        if section.has_flag("parser-sensitive") {
            markers.parser_sensitive += 1;
        }
    }

    let duplicate_effective_ids = id_counts
        .into_iter()
        .filter_map(|(id, count)| (count > 1).then_some(id.to_string()))
        .collect::<Vec<_>>();
    let duplicate_explicit_ids = explicit_id_counts
        .into_iter()
        .filter_map(|(id, count)| (count > 1).then_some(id.to_string()))
        .collect::<Vec<_>>();

    CorpusInventory {
        schema_version: 2,
        files: file_count,
        sections: sections.len(),
        cases: sections.len(),
        ids: InventoryIds {
            explicit,
            generated,
            missing_explicit_ids,
            duplicate_effective_ids,
            duplicate_explicit_ids,
            id_source_breakdown,
        },
        tags: InventoryTags {
            known: known_tags.into_iter().collect(),
            unknown: unknown_tags.into_iter().collect(),
        },
        flags,
        markers,
        generators: generator_families(),
        concept_mapping_available: false,
        expectations_available: false,
        fixtures_without_concepts: Vec::new(),
        fixtures_without_expectations: Vec::new(),
    }
}

/// Stable list of generator families currently exposed by perl-corpus.
pub fn generator_families() -> Vec<String> {
    vec![
        "ambiguity",
        "builtins",
        "control_flow",
        "declarations",
        "expressions",
        "filetest",
        "format_statements",
        "glob",
        "heredoc",
        "io",
        "list_ops",
        "object_oriented",
        "phasers",
        "program",
        "quote_like",
        "qw",
        "regex",
        "sigils",
        "special_vars",
        "tie",
        "whitespace",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

fn populate_fixture_coverage(gold_root: &Path, inventory: &mut CorpusInventory) -> Result<()> {
    if !gold_root.exists() {
        return Ok(());
    }

    let mut fixtures = Vec::new();
    for entry in fs::read_dir(gold_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() || !path.join("fixture.pl").exists() {
            continue;
        }
        if let Some(name) = path.file_name().map(|f| f.to_string_lossy().to_string()) {
            fixtures.push((name, path));
        }
    }
    fixtures.sort_by(|a, b| a.0.cmp(&b.0));

    let mut has_expectation_file = false;
    let mut has_concept_file = false;
    let concept_file_names = ["expected_concepts.json", "concepts.json", "concepts.toml"];
    let mut fixtures_without_expectations = Vec::new();
    let mut fixtures_without_concepts = Vec::new();

    for (name, path) in fixtures {
        let has_expected = path.join("expected.json").exists();
        has_expectation_file |= has_expected;
        if !has_expected {
            fixtures_without_expectations.push(name.clone());
        }

        let has_concepts = concept_file_names.iter().any(|file| path.join(file).exists());
        has_concept_file |= has_concepts;
        if !has_concepts {
            fixtures_without_concepts.push(name);
        }
    }

    inventory.expectations_available = has_expectation_file;
    if has_expectation_file {
        inventory.fixtures_without_expectations = fixtures_without_expectations;
    }

    inventory.concept_mapping_available = has_concept_file;
    if has_concept_file {
        inventory.fixtures_without_concepts = fixtures_without_concepts;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_section(id: &str, tags: &[&str], flags: &[&str]) -> Section {
        Section {
            id: id.to_string(),
            id_source: IdSource::Explicit,
            explicit_id: Some(id.to_string()),
            generated_id: None,
            title: "title".to_string(),
            file: "sample.txt".to_string(),
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
            perl: None,
            flags: flags.iter().map(|flag| (*flag).to_string()).collect(),
            body: "my $x = 1;".to_string(),
            expected: None,
            line: Some(1),
        }
    }

    #[test]
    fn inventory_reports_missing_and_duplicate_ids() {
        let sections = vec![
            sample_section("case.1", &["regex"], &[]),
            sample_section("case.1", &["regex", "custom-tag"], &["parser-sensitive"]),
            Section {
                id: "generated.sample".to_string(),
                id_source: IdSource::Generated,
                explicit_id: None,
                generated_id: Some("generated.sample".to_string()),
                title: "title".to_string(),
                file: "sample.txt".to_string(),
                tags: vec!["custom-tag".to_string()],
                perl: None,
                flags: vec!["wip".to_string()],
                body: "my $x = 1;".to_string(),
                expected: None,
                line: Some(1),
            },
        ];

        let inventory = inventory_from_sections(2, &sections);

        assert_eq!(inventory.schema_version, 2);
        assert_eq!(inventory.files, 2);
        assert_eq!(inventory.sections, 3);
        assert_eq!(inventory.ids.explicit, 2);
        assert_eq!(inventory.ids.generated, 1);
        assert_eq!(inventory.ids.missing_explicit_ids, 1);
        assert_eq!(inventory.ids.duplicate_effective_ids, vec!["case.1".to_string()]);
        assert_eq!(inventory.tags.known, vec!["regex".to_string()]);
        assert_eq!(inventory.tags.unknown, vec!["custom-tag".to_string()]);
        assert_eq!(inventory.markers.parser_sensitive, 1);
        assert_eq!(inventory.markers.wip, 1);
    }

    #[test]
    fn inventory_is_deterministic() {
        let sections = vec![
            sample_section("z.case", &["z-unknown", "regex"], &["todo"]),
            sample_section("a.case", &["regex", "a-unknown"], &["parser-sensitive"]),
        ];

        let first = inventory_from_sections(1, &sections);
        let second = inventory_from_sections(1, &sections);
        assert_eq!(first, second);
        assert_eq!(first.tags.unknown, vec!["a-unknown".to_string(), "z-unknown".to_string()]);
        assert_eq!(first.ids.duplicate_effective_ids, Vec::<String>::new());
    }
}
