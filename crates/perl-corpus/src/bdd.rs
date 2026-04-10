//! Behavior-driven scenario catalog for `perl-corpus`.
//!
//! These scenarios provide stable, human-readable acceptance criteria that
//! downstream tests and docs can reference.

/// A behavior-driven scenario for a `perl-corpus` workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BddScenario {
    /// Stable scenario identifier.
    pub id: &'static str,
    /// Functional area this scenario belongs to.
    pub area: &'static str,
    /// Preconditions for the scenario.
    pub given: &'static str,
    /// Trigger or action being executed.
    pub when: &'static str,
    /// Expected user-observable outcome.
    pub then: &'static str,
}

const BDD_SCENARIOS: &[BddScenario] = &[
    BddScenario {
        id: "corpus.parse-file.metadata-and-body",
        area: "parsing",
        given: "a corpus file with section metadata and an AST block separated by ---",
        when: "parse_file is called",
        then: "metadata is parsed, body code is preserved, and expected output blocks are excluded",
    },
    BddScenario {
        id: "corpus.parse-file.auto-id",
        area: "parsing",
        given: "a section without an explicit @id",
        when: "parse_file is called",
        then: "a deterministic slug-based id is generated for the section",
    },
    BddScenario {
        id: "corpus.parse-dir.stable-order",
        area: "parsing",
        given: "a corpus directory with multiple .txt files",
        when: "parse_dir is called",
        then: "all sections are returned in stable file-and-id sorted order",
    },
    BddScenario {
        id: "corpus.query.by-tag",
        area: "query",
        given: "parsed sections containing heterogeneous tags",
        when: "find_by_tag is called with a tag",
        then: "only sections carrying that tag are returned",
    },
    BddScenario {
        id: "corpus.query.by-flag",
        area: "query",
        given: "parsed sections containing validation flags",
        when: "find_by_flag is called with a flag",
        then: "only sections carrying that flag are returned",
    },
    BddScenario {
        id: "corpus.codegen.seeded-determinism",
        area: "generation",
        given: "a statement count and fixed random seed",
        when: "generate_perl_code_with_seed is called repeatedly with the same seed",
        then: "identical generated code is produced",
    },
    BddScenario {
        id: "corpus.codegen.options-kinds",
        area: "generation",
        given: "codegen options scoped to selected statement kinds",
        when: "generate_perl_code_with_options is called",
        then: "output reflects the selected generation strategy constraints",
    },
    BddScenario {
        id: "corpus.fixtures.edge-cases",
        area: "fixtures",
        given: "the static edge case catalog",
        when: "edge_cases or EdgeCaseGenerator APIs are used",
        then: "fixtures remain deterministic and searchable by id or tag",
    },
    BddScenario {
        id: "corpus.fixtures.specialized-domains",
        area: "fixtures",
        given: "specialized fixture modules for continue/redo, format, glob, and tie",
        when: "domain-specific accessors are called",
        then: "focused fixture collections are available for parser and tooling tests",
    },
    BddScenario {
        id: "corpus.discovery.layers",
        area: "filesystem",
        given: "a workspace with test corpus and fuzz fixtures",
        when: "get_corpus_files or layer-specific helpers are called",
        then: "files are discovered and labeled by corpus layer",
    },
    BddScenario {
        id: "corpus.lint.validation",
        area: "quality",
        given: "corpus sections with ids, tags, and flags",
        when: "lint workflows are run",
        then: "duplicate ids, unknown metadata, and structural violations are reported",
    },
    BddScenario {
        id: "corpus.index.coverage-artifacts",
        area: "quality",
        given: "a corpus directory ready for indexing",
        when: "index workflows are run",
        then: "index and coverage summary artifacts are generated for tooling",
    },
];

/// Return all behavior-driven scenarios for the crate.
pub const fn bdd_scenarios() -> &'static [BddScenario] {
    BDD_SCENARIOS
}

#[cfg(test)]
mod tests {
    use super::{BDD_SCENARIOS, bdd_scenarios};
    use std::collections::HashSet;

    #[test]
    fn scenario_catalog_is_non_empty() {
        assert!(!bdd_scenarios().is_empty());
    }

    #[test]
    fn scenario_ids_are_unique_and_complete() {
        let mut ids = HashSet::new();

        for row in BDD_SCENARIOS {
            assert!(!row.id.is_empty(), "scenario id must not be empty");
            assert!(!row.area.is_empty(), "scenario area must not be empty: {}", row.id);
            assert!(!row.given.is_empty(), "given must not be empty: {}", row.id);
            assert!(!row.when.is_empty(), "when must not be empty: {}", row.id);
            assert!(!row.then.is_empty(), "then must not be empty: {}", row.id);
            assert!(ids.insert(row.id), "duplicate scenario id: {}", row.id);
        }
    }

    #[test]
    fn scenario_areas_cover_core_workflows() {
        let areas: HashSet<_> = BDD_SCENARIOS.iter().map(|row| row.area).collect();

        assert!(areas.contains("parsing"));
        assert!(areas.contains("query"));
        assert!(areas.contains("generation"));
        assert!(areas.contains("fixtures"));
        assert!(areas.contains("filesystem"));
        assert!(areas.contains("quality"));
    }
}
