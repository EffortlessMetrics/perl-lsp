//! Coverage tests for `crates/perl-corpus/src/inventory.rs`.
//!
//! # What is covered
//!
//! - `generator_families`: returns a non-empty, sorted, deterministic list.
//! - `inventory_from_sections`:
//!   - Empty input returns a zero-count inventory with default/empty fields.
//!   - Sections with `IdSource::Explicit` increment `ids.explicit` and
//!     record `id_source_breakdown["explicit"]`.
//!   - Sections with `IdSource::Generated` increment `ids.generated` and
//!     record `id_source_breakdown["generated"]`.
//!   - Missing `explicit_id` on an `Explicit` section increments
//!     `ids.missing_explicit_ids` (edge case: explicit source but no id field).
//!   - Duplicate effective IDs populate `ids.duplicate_effective_ids`.
//!   - Duplicate explicit IDs populate `ids.duplicate_explicit_ids`.
//!   - Known tags accumulate into `tags.known` (BTreeSet order).
//!   - Unknown tags accumulate into `tags.unknown` (BTreeSet order).
//!   - Flags map increments correctly across sections.
//!   - Markers: `expected-error` flag, `wip` flag, `todo` flag, `parser-sensitive` flag.
//!   - `schema_version` is always 2.
//!   - `generators` always matches `generator_families()`.
//! - `build_inventory_from_paths`: non-existent root produces an empty inventory
//!   (no corpus files -> no sections; gold root absent -> no fixture coverage).
//! - `populate_fixture_coverage` indirectly via `build_inventory_from_paths`:
//!   - Gold root does not exist -> `expectations_available = false`.
//!   - Gold root with a fixture that has `expected.json` -> `expectations_available = true`.
//!   - Gold root with a fixture that has a concept file -> `concept_mapping_available = true`.
//!   - `fixtures_without_expectations` populated when some fixtures lack expected.json.
//!   - `fixtures_without_concepts` populated when some fixtures lack concept files.
//!
//! # What is NOT covered (and why)
//!
//! - `build_inventory` (the workspace-discovery variant): depends on the real
//!   workspace layout and `PERL_CORPUS_ROOT`; tested indirectly through
//!   `build_inventory_from_paths`.
//! - `parse_file` I/O errors inside `build_inventory_from_paths`: would require
//!   crafting an unreadable file, which is platform-specific and fragile.

mod inventory {
    use perl_corpus::files::CorpusPaths;
    use perl_corpus::inventory::{
        build_inventory_from_paths, generator_families, inventory_from_sections,
    };
    use perl_corpus::metadata::{IdSource, Section};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    fn temp_dir(suffix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let pid = std::process::id();
        path.push(format!("perl_corpus_inv_{}_{}_{}", suffix, pid, nanos));
        path
    }

    fn make_section(
        id: &str,
        id_source: IdSource,
        explicit_id: Option<&str>,
        tags: &[&str],
        flags: &[&str],
    ) -> Section {
        Section {
            id: id.to_string(),
            id_source,
            explicit_id: explicit_id.map(str::to_string),
            generated_id: if id_source == IdSource::Generated {
                Some(id.to_string())
            } else {
                None
            },
            title: "Title".to_string(),
            file: "f.txt".to_string(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            perl: None,
            flags: flags.iter().map(|f| f.to_string()).collect(),
            body: "my $x = 1;".to_string(),
            expected: None,
            line: Some(1),
        }
    }

    fn explicit(id: &str) -> Section {
        make_section(id, IdSource::Explicit, Some(id), &[], &[])
    }

    fn generated(id: &str) -> Section {
        make_section(id, IdSource::Generated, None, &[], &[])
    }

    // -------------------------------------------------------------------------
    // generator_families
    // -------------------------------------------------------------------------

    #[test]
    fn generator_families_non_empty_and_deterministic() {
        let a = generator_families();
        let b = generator_families();
        assert!(!a.is_empty());
        assert_eq!(a, b, "generator_families must be deterministic");
    }

    #[test]
    fn generator_families_contains_known_generators() {
        let families = generator_families();
        let expected_subset = [
            "regex",
            "heredoc",
            "qw",
            "whitespace",
            "control_flow",
            "format_statements",
            "glob",
            "tie",
            "io",
            "builtins",
        ];
        for name in &expected_subset {
            assert!(
                families.iter().any(|f| f == name),
                "expected generator '{name}' not found in {families:?}"
            );
        }
    }

    // -------------------------------------------------------------------------
    // inventory_from_sections - empty input
    // -------------------------------------------------------------------------

    #[test]
    fn inventory_from_sections_empty_returns_zeroed_counts() {
        let inv = inventory_from_sections(0, &[]);
        assert_eq!(inv.schema_version, 2);
        assert_eq!(inv.files, 0);
        assert_eq!(inv.sections, 0);
        assert_eq!(inv.cases, 0);
        assert_eq!(inv.ids.explicit, 0);
        assert_eq!(inv.ids.generated, 0);
        assert_eq!(inv.ids.missing_explicit_ids, 0);
        assert!(inv.ids.generated_ids.is_empty());
        assert!(inv.ids.duplicate_effective_ids.is_empty());
        assert!(inv.ids.duplicate_explicit_ids.is_empty());
        assert!(inv.tags.known.is_empty());
        assert!(inv.tags.unknown.is_empty());
        assert!(inv.flags.is_empty());
        assert_eq!(inv.markers.expected_error, 0);
        assert_eq!(inv.markers.wip, 0);
        assert_eq!(inv.markers.parser_sensitive, 0);
        assert!(!inv.concept_mapping_available);
        assert!(!inv.expectations_available);
    }

    // -------------------------------------------------------------------------
    // inventory_from_sections - id source breakdown
    // -------------------------------------------------------------------------

    #[test]
    fn inventory_from_sections_counts_explicit_ids() {
        let sections = [explicit("a.case"), explicit("b.case")];
        let inv = inventory_from_sections(1, &sections);
        assert_eq!(inv.ids.explicit, 2);
        assert_eq!(inv.ids.generated, 0);
        assert_eq!(inv.ids.id_source_breakdown.get("explicit"), Some(&2));
        assert_eq!(inv.ids.id_source_breakdown.get("generated"), None);
    }

    #[test]
    fn inventory_from_sections_counts_generated_ids() {
        let sections = [generated("gen.001"), generated("gen.002")];
        let inv = inventory_from_sections(1, &sections);
        assert_eq!(inv.ids.explicit, 0);
        assert_eq!(inv.ids.generated, 2);
        assert_eq!(inv.ids.missing_explicit_ids, 2, "every generated section is missing explicit");
        assert_eq!(inv.ids.id_source_breakdown.get("generated"), Some(&2));
        // generated IDs are collected in a BTreeSet, so order is deterministic
        assert_eq!(inv.ids.generated_ids.len(), 2);
    }

    #[test]
    fn inventory_from_sections_mixed_sources() {
        let sections = [explicit("a.case"), generated("gen.001"), explicit("b.case")];
        let inv = inventory_from_sections(2, &sections);
        assert_eq!(inv.ids.explicit, 2);
        assert_eq!(inv.ids.generated, 1);
        assert_eq!(inv.ids.missing_explicit_ids, 1);
    }

    // -------------------------------------------------------------------------
    // inventory_from_sections - explicit section with no explicit_id field
    // -------------------------------------------------------------------------

    #[test]
    fn inventory_from_sections_explicit_source_without_id_field_counts_as_missing() {
        // Explicit source but explicit_id = None - the unusual edge case
        let section = make_section(
            "some.id",
            IdSource::Explicit,
            None, // explicit_id is None despite Explicit source
            &[],
            &[],
        );
        let inv = inventory_from_sections(1, &[section]);
        assert_eq!(inv.ids.explicit, 1);
        assert_eq!(inv.ids.missing_explicit_ids, 1, "no explicit_id field -> missing count");
    }

    // -------------------------------------------------------------------------
    // inventory_from_sections - duplicate detection
    // -------------------------------------------------------------------------

    #[test]
    fn inventory_from_sections_detects_duplicate_effective_ids() {
        let sections = [explicit("dup.case"), explicit("dup.case"), explicit("unique.case")];
        let inv = inventory_from_sections(1, &sections);
        assert_eq!(inv.ids.duplicate_effective_ids, vec!["dup.case"]);
        assert!(inv.ids.duplicate_explicit_ids.contains(&"dup.case".to_string()));
    }

    #[test]
    fn inventory_from_sections_no_duplicates_when_unique() {
        let sections = [explicit("a.case"), explicit("b.case"), explicit("c.case")];
        let inv = inventory_from_sections(1, &sections);
        assert!(inv.ids.duplicate_effective_ids.is_empty());
        assert!(inv.ids.duplicate_explicit_ids.is_empty());
    }

    // -------------------------------------------------------------------------
    // inventory_from_sections - tags
    // -------------------------------------------------------------------------

    #[test]
    fn inventory_from_sections_classifies_known_and_unknown_tags() {
        let sections = [
            make_section(
                "a.case",
                IdSource::Explicit,
                Some("a.case"),
                &["regex", "my-unknown-tag"],
                &[],
            ),
            make_section(
                "b.case",
                IdSource::Explicit,
                Some("b.case"),
                &["heredoc", "another-unknown"],
                &[],
            ),
        ];
        let inv = inventory_from_sections(1, &sections);
        // regex and heredoc are in KNOWN_TAGS
        assert!(inv.tags.known.contains(&"regex".to_string()));
        assert!(inv.tags.known.contains(&"heredoc".to_string()));
        // unknown tags go to the unknown bucket
        assert!(inv.tags.unknown.contains(&"my-unknown-tag".to_string()));
        assert!(inv.tags.unknown.contains(&"another-unknown".to_string()));
    }

    #[test]
    fn inventory_from_sections_tags_are_deduplicated_across_sections() {
        // Same tag appearing in multiple sections should appear only once in `known`
        let sections = [
            make_section("a.case", IdSource::Explicit, Some("a.case"), &["regex"], &[]),
            make_section("b.case", IdSource::Explicit, Some("b.case"), &["regex"], &[]),
        ];
        let inv = inventory_from_sections(1, &sections);
        let regex_count = inv.tags.known.iter().filter(|t| t.as_str() == "regex").count();
        assert_eq!(regex_count, 1, "duplicate tag should appear once in known list");
    }

    // -------------------------------------------------------------------------
    // inventory_from_sections - flags map
    // -------------------------------------------------------------------------

    #[test]
    fn inventory_from_sections_flags_count_per_flag() {
        let sections = [
            make_section(
                "a.case",
                IdSource::Explicit,
                Some("a.case"),
                &[],
                &["parser-sensitive", "slow"],
            ),
            make_section("b.case", IdSource::Explicit, Some("b.case"), &[], &["slow"]),
        ];
        let inv = inventory_from_sections(1, &sections);
        assert_eq!(inv.flags.get("parser-sensitive"), Some(&1));
        assert_eq!(inv.flags.get("slow"), Some(&2));
    }

    // -------------------------------------------------------------------------
    // inventory_from_sections - markers
    // -------------------------------------------------------------------------

    #[test]
    fn inventory_from_sections_marker_expected_error() {
        let sections = [
            make_section("a.case", IdSource::Explicit, Some("a.case"), &[], &["expected-error"]),
            make_section("b.case", IdSource::Explicit, Some("b.case"), &[], &[]),
        ];
        let inv = inventory_from_sections(1, &sections);
        assert_eq!(inv.markers.expected_error, 1);
    }

    #[test]
    fn inventory_from_sections_marker_wip_flag() {
        let sections = [
            make_section("a.case", IdSource::Explicit, Some("a.case"), &[], &["wip"]),
            make_section("b.case", IdSource::Explicit, Some("b.case"), &[], &["todo"]),
            make_section("c.case", IdSource::Explicit, Some("c.case"), &[], &[]),
        ];
        let inv = inventory_from_sections(1, &sections);
        // wip and todo both count toward the wip marker
        assert_eq!(inv.markers.wip, 2);
    }

    #[test]
    fn inventory_from_sections_marker_parser_sensitive() {
        let sections = [
            make_section("a.case", IdSource::Explicit, Some("a.case"), &[], &["parser-sensitive"]),
            make_section("b.case", IdSource::Explicit, Some("b.case"), &[], &["parser-sensitive"]),
            make_section("c.case", IdSource::Explicit, Some("c.case"), &[], &[]),
        ];
        let inv = inventory_from_sections(1, &sections);
        assert_eq!(inv.markers.parser_sensitive, 2);
    }

    // -------------------------------------------------------------------------
    // inventory_from_sections - generators list matches generator_families()
    // -------------------------------------------------------------------------

    #[test]
    fn inventory_from_sections_generators_matches_generator_families() {
        let inv = inventory_from_sections(0, &[]);
        assert_eq!(inv.generators, generator_families());
    }

    // -------------------------------------------------------------------------
    // inventory_from_sections - schema_version is always 2
    // -------------------------------------------------------------------------

    #[test]
    fn inventory_from_sections_schema_version_is_2() {
        let inv = inventory_from_sections(99, &[]);
        assert_eq!(inv.schema_version, 2);
    }

    // -------------------------------------------------------------------------
    // build_inventory_from_paths - nonexistent corpus root
    // -------------------------------------------------------------------------

    #[test]
    fn build_inventory_from_paths_nonexistent_root() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("nonexistent");
        // root does NOT exist, so there are no corpus files
        let paths = CorpusPaths::from_root(root.clone());
        let inv = build_inventory_from_paths(&paths)?;
        assert_eq!(inv.files, 0);
        assert_eq!(inv.sections, 0);
        // gold root (root/test_corpus/gold) also doesn't exist -> no fixture info
        assert!(!inv.expectations_available);
        assert!(!inv.concept_mapping_available);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // build_inventory_from_paths - gold root absent
    // -------------------------------------------------------------------------

    #[test]
    fn build_inventory_from_paths_gold_root_absent_expectations_not_available()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("no_gold");
        fs::create_dir_all(&root)?;
        // test_corpus exists but no gold sub-directory
        let test_corpus = root.join("test_corpus");
        fs::create_dir_all(&test_corpus)?;

        let paths = CorpusPaths::from_root(root.clone());
        let inv = build_inventory_from_paths(&paths)?;
        assert!(!inv.expectations_available, "gold root absent -> expectations_available=false");
        assert!(
            !inv.concept_mapping_available,
            "gold root absent -> concept_mapping_available=false"
        );

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // build_inventory_from_paths - gold root with expectation fixtures
    // -------------------------------------------------------------------------

    #[test]
    fn build_inventory_from_paths_gold_fixtures_with_expected_json()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("gold_with_expected");
        let gold_dir = root.join("test_corpus").join("gold");
        let fixture_dir = gold_dir.join("my-fixture");
        fs::create_dir_all(&fixture_dir)?;

        // A valid fixture.pl + expected.json
        fs::write(fixture_dir.join("fixture.pl"), "my $x = 1;\n")?;
        fs::write(fixture_dir.join("expected.json"), r#"{"diagnostics":[]}"#)?;

        let paths = CorpusPaths::from_root(root.clone());
        let inv = build_inventory_from_paths(&paths)?;
        assert!(inv.expectations_available, "expected.json present -> expectations_available=true");
        assert!(
            inv.fixtures_without_expectations.is_empty(),
            "all fixtures have expected.json so list should be empty"
        );

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn build_inventory_from_paths_gold_fixture_without_expected_json_listed()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("gold_missing_expected");
        let gold_dir = root.join("test_corpus").join("gold");

        // Two fixtures: one has expected.json, one does not
        let with_expected = gold_dir.join("has-expected");
        fs::create_dir_all(&with_expected)?;
        fs::write(with_expected.join("fixture.pl"), "1;\n")?;
        fs::write(with_expected.join("expected.json"), r#"{"diagnostics":[]}"#)?;

        let without_expected = gold_dir.join("no-expected");
        fs::create_dir_all(&without_expected)?;
        fs::write(without_expected.join("fixture.pl"), "1;\n")?;

        let paths = CorpusPaths::from_root(root.clone());
        let inv = build_inventory_from_paths(&paths)?;
        assert!(inv.expectations_available, "at least one fixture has expected.json");
        assert_eq!(
            inv.fixtures_without_expectations,
            vec!["no-expected".to_string()],
            "fixture without expected.json should be listed"
        );

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // build_inventory_from_paths - gold root with concept fixtures
    // -------------------------------------------------------------------------

    #[test]
    fn build_inventory_from_paths_gold_fixture_with_concepts_json()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("gold_with_concepts");
        let gold_dir = root.join("test_corpus").join("gold");
        let fixture_dir = gold_dir.join("my-concept-fixture");
        fs::create_dir_all(&fixture_dir)?;

        fs::write(fixture_dir.join("fixture.pl"), "my $y = 2;\n")?;
        // Use one of the recognized concept file names
        fs::write(fixture_dir.join("concepts.json"), r#"[]"#)?;

        let paths = CorpusPaths::from_root(root.clone());
        let inv = build_inventory_from_paths(&paths)?;
        assert!(
            inv.concept_mapping_available,
            "concepts.json present -> concept_mapping_available=true"
        );

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn build_inventory_from_paths_gold_fixture_with_expected_concepts_toml()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("gold_with_concepts_toml");
        let gold_dir = root.join("test_corpus").join("gold");
        let fixture_dir = gold_dir.join("toml-concept-fixture");
        fs::create_dir_all(&fixture_dir)?;

        fs::write(fixture_dir.join("fixture.pl"), "my $z = 3;\n")?;
        // expected_concepts.json is also a recognized name
        fs::write(fixture_dir.join("expected_concepts.json"), r#"[]"#)?;

        let paths = CorpusPaths::from_root(root.clone());
        let inv = build_inventory_from_paths(&paths)?;
        assert!(inv.concept_mapping_available);

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn build_inventory_from_paths_fixture_without_concepts_listed()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("gold_missing_concepts");
        let gold_dir = root.join("test_corpus").join("gold");

        // Fixture with a concepts file
        let with_concepts = gold_dir.join("has-concepts");
        fs::create_dir_all(&with_concepts)?;
        fs::write(with_concepts.join("fixture.pl"), "1;\n")?;
        fs::write(with_concepts.join("concepts.json"), r#"[]"#)?;

        // Fixture without any concepts file
        let without_concepts = gold_dir.join("no-concepts");
        fs::create_dir_all(&without_concepts)?;
        fs::write(without_concepts.join("fixture.pl"), "1;\n")?;

        let paths = CorpusPaths::from_root(root.clone());
        let inv = build_inventory_from_paths(&paths)?;
        assert!(inv.concept_mapping_available);
        assert_eq!(inv.fixtures_without_concepts, vec!["no-concepts".to_string()],);

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // build_inventory_from_paths - non-directory entries in gold root are skipped
    // -------------------------------------------------------------------------

    #[test]
    fn build_inventory_from_paths_gold_root_skips_non_dir_entries()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("gold_non_dir");
        let gold_dir = root.join("test_corpus").join("gold");
        fs::create_dir_all(&gold_dir)?;

        // Plain files in gold root (not directories with fixture.pl)
        fs::write(gold_dir.join("readme.txt"), "not a fixture\n")?;
        fs::write(gold_dir.join("config.json"), r#"{}"#)?;

        // A directory without fixture.pl - should also be skipped
        let empty_subdir = gold_dir.join("empty-fixture");
        fs::create_dir_all(&empty_subdir)?;
        // No fixture.pl inside

        let paths = CorpusPaths::from_root(root.clone());
        let inv = build_inventory_from_paths(&paths)?;
        // No valid fixture directories found
        assert!(!inv.expectations_available);
        assert!(!inv.concept_mapping_available);

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // inventory_from_sections - file count is passed through
    // -------------------------------------------------------------------------

    #[test]
    fn inventory_from_sections_passes_through_file_count() {
        let inv = inventory_from_sections(42, &[]);
        assert_eq!(inv.files, 42);
    }

    // -------------------------------------------------------------------------
    // inventory_from_sections - sections and cases counts match
    // -------------------------------------------------------------------------

    #[test]
    fn inventory_from_sections_sections_and_cases_match() {
        let sections: Vec<Section> = (0..5_u32).map(|i| explicit(&format!("case.{i}"))).collect();
        let inv = inventory_from_sections(1, &sections);
        assert_eq!(inv.sections, 5);
        assert_eq!(inv.cases, 5);
    }

    // -------------------------------------------------------------------------
    // inventory_from_sections - concept and expectation fields default to false/empty
    // -------------------------------------------------------------------------

    #[test]
    fn inventory_from_sections_concept_and_expectation_fields_default() {
        let inv = inventory_from_sections(0, &[]);
        assert!(!inv.concept_mapping_available);
        assert!(!inv.expectations_available);
        assert!(inv.fixtures_without_concepts.is_empty());
        assert!(inv.fixtures_without_expectations.is_empty());
    }
}
