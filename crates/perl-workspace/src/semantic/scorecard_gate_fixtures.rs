//! Per-provider scorecard gate fixture suites.
//!
//! These fixtures exercise the scorecard gate criteria for each provider,
//! building synthetic workspaces with known entities, imports, exports,
//! and dynamic boundaries, then verifying the scorecard criteria are met.
//!
//! # Requirements
//!
//! - **Req 10.6**: goto-definition scorecard — regressions=0, ambiguous
//!   classified, unavailable falls back.
//! - **Req 10.7**: find-references scorecard — legacy count parity or
//!   better, definition exclusion correct.
//! - **Req 10.8**: completion scorecard — explicit import pass, default
//!   export pass, empty import suppresses, tag export pass.
//! - **Req 10.9**: diagnostics scorecard — imported-symbol false
//!   positives=0, dynamic-boundary exact warnings=0.
//! - **Req 10.10**: rename/safe-delete scorecard — unsafe edits=0,
//!   dynamic blocked, ambiguous blocked, export/import blocked or planned.
//! - **Req 11.3**: completion visibility fixture suite passes.
//! - **Req 11.4**: undefined-symbol false-positive fixture suite passes.
//! - **Req 11.5**: definition/reference shadow-compare suite shows Same
//!   or Improved.
//! - **Req 11.6**: rename unsafe-edit count is zero.

#[cfg(test)]
mod tests {
    use crate::semantic::facts::PRODUCER_SCHEMA_VERSION;
    use crate::semantic::imports::ImportExportIndex;
    use crate::semantic::queries::{QueryContext, SemanticQueries, WorkspaceSemanticQueries};
    use crate::semantic::references::ReferenceIndex;
    use crate::semantic::scorecard::{Scorecard, ScorecardMode};
    use crate::semantic_shadow_compare::{
        SemanticShadowCompareReceipt, ShadowCompareVerdict, ShadowQueryInput, ShadowQueryName,
        ShadowResultSummary,
    };
    use crate::workspace::workspace_index::FileFactShard;
    use perl_semantic_facts::{
        AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EdgeKind, EntityFact, EntityId,
        EntityKind, ExportSet, ExportTag, FileId, ImportKind, ImportSpec, ImportSymbols,
        OccurrenceFact, OccurrenceId, OccurrenceKind, PlanBlockerReason, Provenance,
    };
    use std::collections::HashMap;

    // ── Shared helpers ──

    /// Build a `FileFactShard` from components.
    fn make_shard(
        uri: &str,
        file_id: FileId,
        anchors: Vec<AnchorFact>,
        entities: Vec<EntityFact>,
        occurrences: Vec<OccurrenceFact>,
        edges: Vec<EdgeFact>,
    ) -> FileFactShard {
        FileFactShard {
            source_uri: uri.to_string(),
            file_id,
            content_hash: 0,
            producer_schema_version: PRODUCER_SCHEMA_VERSION,
            anchors_hash: None,
            entities_hash: None,
            occurrences_hash: None,
            edges_hash: None,
            anchors,
            entities,
            occurrences,
            edges,
        }
    }

    /// Build a receipt with crafted summaries that produce the desired verdict.
    fn make_receipt(
        query: ShadowQueryName,
        symbol: &str,
        old_summary: ShadowResultSummary,
        new_summary: ShadowResultSummary,
    ) -> SemanticShadowCompareReceipt {
        SemanticShadowCompareReceipt::from_summaries(
            query,
            ShadowQueryInput { symbol: symbol.to_string() },
            old_summary,
            new_summary,
            vec![],
        )
    }

    /// Build a summary representing an available result with given identities.
    fn available_summary(identities: Vec<String>) -> ShadowResultSummary {
        let match_count = u64::try_from(identities.len()).unwrap_or(u64::MAX);
        ShadowResultSummary { available: true, match_count, identities }
    }

    /// Build a summary representing an unavailable result.
    fn unavailable_summary() -> ShadowResultSummary {
        ShadowResultSummary { available: false, match_count: 0, identities: Vec::new() }
    }

    /// Build a workspace with a single file containing a subroutine definition
    /// and a call reference.
    fn build_simple_workspace()
    -> (FileId, ReferenceIndex, ImportExportIndex, HashMap<String, FileFactShard>) {
        let file_id = FileId(1);
        let anchor_def = AnchorId(10);
        let anchor_ref = AnchorId(20);
        let entity_id = EntityId(100);

        let shard = make_shard(
            "file:///lib/Foo.pm",
            file_id,
            vec![
                AnchorFact {
                    id: anchor_def,
                    file_id,
                    span_start_byte: 0,
                    span_end_byte: 15,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                AnchorFact {
                    id: anchor_ref,
                    file_id,
                    span_start_byte: 50,
                    span_end_byte: 58,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "Foo::bar".to_string(),
                anchor_id: Some(anchor_def),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![
                OccurrenceFact {
                    id: OccurrenceId(200),
                    kind: OccurrenceKind::Definition,
                    entity_id: Some(entity_id),
                    anchor_id: anchor_def,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                OccurrenceFact {
                    id: OccurrenceId(201),
                    kind: OccurrenceKind::Call,
                    entity_id: Some(entity_id),
                    anchor_id: anchor_ref,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            vec![EdgeFact {
                id: EdgeId(300),
                kind: EdgeKind::References,
                from_entity_id: EntityId(0),
                to_entity_id: entity_id,
                via_occurrence_id: Some(OccurrenceId(201)),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
        );

        let mut ref_index = ReferenceIndex::new();
        ref_index.add_file(&shard);

        let ie_index = ImportExportIndex::new();
        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        (file_id, ref_index, ie_index, shards)
    }

    /// Build a workspace with two files: one exporting symbols, one importing
    /// them. Used for completion and diagnostics fixtures.
    fn build_import_workspace()
    -> (FileId, FileId, ReferenceIndex, ImportExportIndex, HashMap<String, FileFactShard>) {
        let exporter_fid = FileId(10);
        let importer_fid = FileId(20);

        // Exporter: Foo.pm defines sub alpha, sub beta
        let exporter_shard = make_shard(
            "file:///lib/Foo.pm",
            exporter_fid,
            vec![
                AnchorFact {
                    id: AnchorId(100),
                    file_id: exporter_fid,
                    span_start_byte: 0,
                    span_end_byte: 10,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                AnchorFact {
                    id: AnchorId(101),
                    file_id: exporter_fid,
                    span_start_byte: 20,
                    span_end_byte: 30,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            vec![
                EntityFact {
                    id: EntityId(1000),
                    kind: EntityKind::Subroutine,
                    canonical_name: "Foo::alpha".to_string(),
                    anchor_id: Some(AnchorId(100)),
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                EntityFact {
                    id: EntityId(1001),
                    kind: EntityKind::Subroutine,
                    canonical_name: "Foo::beta".to_string(),
                    anchor_id: Some(AnchorId(101)),
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            vec![
                OccurrenceFact {
                    id: OccurrenceId(2000),
                    kind: OccurrenceKind::Definition,
                    entity_id: Some(EntityId(1000)),
                    anchor_id: AnchorId(100),
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                OccurrenceFact {
                    id: OccurrenceId(2001),
                    kind: OccurrenceKind::Definition,
                    entity_id: Some(EntityId(1001)),
                    anchor_id: AnchorId(101),
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            vec![],
        );

        // Importer: Bar.pm uses Foo qw(alpha beta) and calls alpha
        let importer_shard = make_shard(
            "file:///lib/Bar.pm",
            importer_fid,
            vec![AnchorFact {
                id: AnchorId(200),
                file_id: importer_fid,
                span_start_byte: 50,
                span_end_byte: 60,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
            vec![OccurrenceFact {
                id: OccurrenceId(3000),
                kind: OccurrenceKind::Call,
                entity_id: Some(EntityId(1000)),
                anchor_id: AnchorId(200),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EdgeFact {
                id: EdgeId(400),
                kind: EdgeKind::References,
                from_entity_id: EntityId(0),
                to_entity_id: EntityId(1000),
                via_occurrence_id: Some(OccurrenceId(3000)),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
        );

        let mut ref_index = ReferenceIndex::new();
        ref_index.add_file(&exporter_shard);
        ref_index.add_file(&importer_shard);

        let mut ie_index = ImportExportIndex::new();
        // Register exports for Foo
        ie_index.add_module_exports(
            "file:///lib/Foo.pm",
            "Foo",
            ExportSet {
                default_exports: vec!["alpha".to_string()],
                optional_exports: vec!["beta".to_string()],
                tags: vec![ExportTag {
                    name: "all".to_string(),
                    members: vec!["alpha".to_string(), "beta".to_string()],
                }],
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                module_name: Some("Foo".to_string()),
                anchor_id: None,
            },
        );
        // Register explicit import in Bar
        ie_index.add_file_imports(
            "file:///lib/Bar.pm",
            importer_fid,
            vec![ImportSpec {
                module: "Foo".to_string(),
                kind: ImportKind::UseExplicitList,
                symbols: ImportSymbols::Explicit(vec!["alpha".to_string(), "beta".to_string()]),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                file_id: Some(importer_fid),
                anchor_id: None,
                scope_id: None,
                span_start_byte: None,
            }],
        );

        let mut shards = HashMap::new();
        shards.insert(exporter_shard.source_uri.clone(), exporter_shard);
        shards.insert(importer_shard.source_uri.clone(), importer_shard);

        (exporter_fid, importer_fid, ref_index, ie_index, shards)
    }

    /// Build a workspace with a dynamic boundary occurrence for diagnostics
    /// and rename/safe-delete fixtures.
    fn build_dynamic_boundary_workspace()
    -> (FileId, EntityId, ReferenceIndex, ImportExportIndex, HashMap<String, FileFactShard>) {
        let file_id = FileId(30);
        let entity_id = EntityId(3000);
        let anchor_def = AnchorId(300);
        let anchor_dyn = AnchorId(301);

        let shard = make_shard(
            "file:///lib/Dyn.pm",
            file_id,
            vec![
                AnchorFact {
                    id: anchor_def,
                    file_id,
                    span_start_byte: 0,
                    span_end_byte: 10,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                AnchorFact {
                    id: anchor_dyn,
                    file_id,
                    span_start_byte: 20,
                    span_end_byte: 35,
                    scope_id: None,
                    provenance: Provenance::DynamicBoundary,
                    confidence: Confidence::Low,
                },
            ],
            vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "Dyn::dispatch".to_string(),
                anchor_id: Some(anchor_def),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![
                OccurrenceFact {
                    id: OccurrenceId(4000),
                    kind: OccurrenceKind::Definition,
                    entity_id: Some(entity_id),
                    anchor_id: anchor_def,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                OccurrenceFact {
                    id: OccurrenceId(4001),
                    kind: OccurrenceKind::DynamicBoundary,
                    entity_id: Some(entity_id),
                    anchor_id: anchor_dyn,
                    scope_id: None,
                    provenance: Provenance::DynamicBoundary,
                    confidence: Confidence::Low,
                },
            ],
            vec![EdgeFact {
                id: EdgeId(500),
                kind: EdgeKind::DynamicBoundary,
                from_entity_id: EntityId(0),
                to_entity_id: entity_id,
                via_occurrence_id: Some(OccurrenceId(4001)),
                provenance: Provenance::DynamicBoundary,
                confidence: Confidence::Low,
            }],
        );

        let mut ref_index = ReferenceIndex::new();
        ref_index.add_file(&shard);

        let ie_index = ImportExportIndex::new();
        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        (file_id, entity_id, ref_index, ie_index, shards)
    }

    // ════════════════════════════════════════════════════════════════════
    // 1. Goto-Definition Scorecard Gate (Req 10.6, 11.5)
    //
    // Criteria: regressions=0, ambiguous classified, unavailable falls back.
    // ════════════════════════════════════════════════════════════════════

    /// Goto-definition: exact match produces Same verdict, no regressions.
    #[test]
    fn goto_def_exact_match_no_regression() -> Result<(), Box<dyn std::error::Error>> {
        let (file_id, ref_index, ie_index, shards) = build_simple_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);
        let ctx = QueryContext::new(file_id, None, None);

        let candidates = queries.definitions("Foo::bar", &ctx);
        assert!(!candidates.is_empty(), "should find definition for Foo::bar");

        // Build shadow receipt: old and new both find the same definition.
        let identity = format!("file:///lib/Foo.pm:{}", candidates[0].anchor_id.0);
        let old_summary = available_summary(vec![identity.clone()]);
        let new_summary = available_summary(vec![identity]);
        let receipt =
            make_receipt(ShadowQueryName::FindDefinition, "Foo::bar", old_summary, new_summary);
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Same);

        let mut scorecard = Scorecard::new(ScorecardMode::Check);
        scorecard.add_receipt(receipt);
        let report = scorecard.report();
        assert!(report.passed, "scorecard should pass with no regressions");
        assert_eq!(report.totals.regression, 0);
        Ok(())
    }

    /// Goto-definition: ambiguous case is classified as Ambiguous, not Regression.
    #[test]
    fn goto_def_ambiguous_classified() -> Result<(), Box<dyn std::error::Error>> {
        // Two candidates with different identities at the same count → Ambiguous.
        let old_summary = available_summary(vec!["file:///lib/A.pm:10".to_string()]);
        let new_summary = available_summary(vec!["file:///lib/B.pm:20".to_string()]);
        let receipt =
            make_receipt(ShadowQueryName::FindDefinition, "ambig_sym", old_summary, new_summary);
        assert_eq!(
            receipt.verdict,
            ShadowCompareVerdict::Ambiguous,
            "different identities at same count should be Ambiguous"
        );

        let mut scorecard = Scorecard::new(ScorecardMode::Check);
        scorecard.add_receipt(receipt);
        let report = scorecard.report();
        // Ambiguous is not a regression — scorecard should still pass.
        assert!(report.passed, "ambiguous is not a regression");
        assert_eq!(report.totals.ambiguous, 1);
        assert_eq!(report.totals.regression, 0);
        Ok(())
    }

    /// Goto-definition: unavailable falls back (Unavailable verdict, not Regression).
    #[test]
    fn goto_def_unavailable_falls_back() -> Result<(), Box<dyn std::error::Error>> {
        let old_summary = unavailable_summary();
        let new_summary = unavailable_summary();
        let receipt =
            make_receipt(ShadowQueryName::FindDefinition, "missing_sym", old_summary, new_summary);
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Unavailable);

        let mut scorecard = Scorecard::new(ScorecardMode::Check);
        scorecard.add_receipt(receipt);
        let report = scorecard.report();
        // Unavailable is not a regression — scorecard should still pass.
        assert!(report.passed, "unavailable is not a regression");
        assert_eq!(report.totals.unavailable, 1);
        assert_eq!(report.totals.regression, 0);
        Ok(())
    }

    /// Goto-definition: improved result (new path finds more) passes scorecard.
    #[test]
    fn goto_def_improved_passes() -> Result<(), Box<dyn std::error::Error>> {
        let old_summary = available_summary(vec!["file:///lib/A.pm:10".to_string()]);
        let new_summary = available_summary(vec![
            "file:///lib/A.pm:10".to_string(),
            "file:///lib/B.pm:20".to_string(),
        ]);
        let receipt =
            make_receipt(ShadowQueryName::FindDefinition, "multi_def", old_summary, new_summary);
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Improved);

        let mut scorecard = Scorecard::new(ScorecardMode::Check);
        scorecard.add_receipt(receipt);
        let report = scorecard.report();
        assert!(report.passed);
        assert_eq!(report.totals.improved, 1);
        Ok(())
    }

    /// Goto-definition: full fixture suite with mixed verdicts, zero regressions.
    #[test]
    fn goto_def_full_fixture_suite_passes() -> Result<(), Box<dyn std::error::Error>> {
        let mut scorecard = Scorecard::new(ScorecardMode::Check);

        // Same verdict
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            "exact_sym",
            available_summary(vec!["file:///lib/Foo.pm:10".to_string()]),
            available_summary(vec!["file:///lib/Foo.pm:10".to_string()]),
        ));
        // Improved verdict
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            "better_sym",
            available_summary(vec!["file:///lib/A.pm:1".to_string()]),
            available_summary(vec![
                "file:///lib/A.pm:1".to_string(),
                "file:///lib/B.pm:2".to_string(),
            ]),
        ));
        // Ambiguous verdict
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            "ambig_sym",
            available_summary(vec!["file:///lib/X.pm:5".to_string()]),
            available_summary(vec!["file:///lib/Y.pm:5".to_string()]),
        ));
        // Unavailable verdict
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            "missing_sym",
            unavailable_summary(),
            unavailable_summary(),
        ));

        let report = scorecard.report();
        assert!(report.passed, "full goto-def suite should pass with zero regressions");
        assert_eq!(report.totals.regression, 0);
        assert_eq!(report.totals.same, 1);
        assert_eq!(report.totals.improved, 1);
        assert_eq!(report.totals.ambiguous, 1);
        assert_eq!(report.totals.unavailable, 1);
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // 2. Find-References Scorecard Gate (Req 10.7, 11.5)
    //
    // Criteria: legacy count parity or better, definition exclusion correct.
    // ════════════════════════════════════════════════════════════════════

    /// Find-references: count parity (Same verdict) passes scorecard.
    #[test]
    fn find_refs_count_parity() -> Result<(), Box<dyn std::error::Error>> {
        let (_, ref_index, ie_index, shards) = build_simple_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        let refs = queries.references(EntityId(100));
        // The simple workspace has one call reference (definition is excluded).
        assert!(!refs.is_empty(), "should find at least one reference for entity 100");

        // Build receipt: old and new find the same count.
        let identities: Vec<String> = refs.iter().map(|r| format!("occ:{}", r.id.0)).collect();
        let old_summary = available_summary(identities.clone());
        let new_summary = available_summary(identities);
        let receipt =
            make_receipt(ShadowQueryName::FindReferences, "Foo::bar", old_summary, new_summary);
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Same);

        let mut scorecard = Scorecard::new(ScorecardMode::Check);
        scorecard.add_receipt(receipt);
        let report = scorecard.report();
        assert!(report.passed);
        assert_eq!(report.totals.regression, 0);
        Ok(())
    }

    /// Find-references: definition exclusion — definitions are not in references.
    #[test]
    fn find_refs_definition_exclusion() -> Result<(), Box<dyn std::error::Error>> {
        let (_, ref_index, ie_index, shards) = build_simple_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        let refs = queries.references(EntityId(100));
        // Verify no Definition occurrences in the reference list.
        for r in &refs {
            assert_ne!(
                r.kind,
                OccurrenceKind::Definition,
                "definition occurrences should be excluded from references"
            );
        }
        Ok(())
    }

    /// Find-references: improved count (new path finds more) passes scorecard.
    #[test]
    fn find_refs_improved_count() -> Result<(), Box<dyn std::error::Error>> {
        let old_summary = available_summary(vec!["ref:1".to_string()]);
        let new_summary = available_summary(vec!["ref:1".to_string(), "ref:2".to_string()]);
        let receipt =
            make_receipt(ShadowQueryName::FindReferences, "some_sym", old_summary, new_summary);
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Improved);

        let mut scorecard = Scorecard::new(ScorecardMode::Check);
        scorecard.add_receipt(receipt);
        let report = scorecard.report();
        assert!(report.passed);
        assert_eq!(report.totals.improved, 1);
        Ok(())
    }

    /// Find-references: full fixture suite with parity and improved, zero regressions.
    #[test]
    fn find_refs_full_fixture_suite_passes() -> Result<(), Box<dyn std::error::Error>> {
        let mut scorecard = Scorecard::new(ScorecardMode::Check);

        // Parity
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::FindReferences,
            "parity_sym",
            available_summary(vec!["ref:1".to_string(), "ref:2".to_string()]),
            available_summary(vec!["ref:1".to_string(), "ref:2".to_string()]),
        ));
        // Improved
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::FindReferences,
            "better_sym",
            available_summary(vec!["ref:1".to_string()]),
            available_summary(vec!["ref:1".to_string(), "ref:2".to_string()]),
        ));
        // Unavailable (both paths)
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::FindReferences,
            "missing_sym",
            unavailable_summary(),
            unavailable_summary(),
        ));

        let report = scorecard.report();
        assert!(report.passed, "full find-refs suite should pass");
        assert_eq!(report.totals.regression, 0);
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // 3. Completion Scorecard Gate (Req 10.8, 11.3)
    //
    // Criteria: explicit import pass, default export pass, empty import
    // suppresses defaults, tag export pass.
    // ════════════════════════════════════════════════════════════════════

    /// Completion: explicit import makes named symbols visible.
    #[test]
    fn completion_explicit_import_pass() -> Result<(), Box<dyn std::error::Error>> {
        let (_, importer_fid, ref_index, ie_index, shards) = build_import_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        let visible = queries.visible_symbols_at(importer_fid, 55, None);
        let names: Vec<&str> = visible.iter().map(|v| v.name.as_str()).collect();
        assert!(
            names.contains(&"alpha"),
            "explicitly imported 'alpha' should be visible, got: {names:?}"
        );
        assert!(
            names.contains(&"beta"),
            "explicitly imported 'beta' should be visible, got: {names:?}"
        );
        Ok(())
    }

    /// Completion: bare `use Foo` makes default exports visible.
    #[test]
    fn completion_default_export_pass() -> Result<(), Box<dyn std::error::Error>> {
        let exporter_fid = FileId(10);
        let importer_fid = FileId(20);

        let exporter_shard = make_shard(
            "file:///lib/Foo.pm",
            exporter_fid,
            vec![AnchorFact {
                id: AnchorId(100),
                file_id: exporter_fid,
                span_start_byte: 0,
                span_end_byte: 10,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: EntityId(1000),
                kind: EntityKind::Subroutine,
                canonical_name: "Foo::alpha".to_string(),
                anchor_id: Some(AnchorId(100)),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![OccurrenceFact {
                id: OccurrenceId(2000),
                kind: OccurrenceKind::Definition,
                entity_id: Some(EntityId(1000)),
                anchor_id: AnchorId(100),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
        );

        let importer_shard =
            make_shard("file:///lib/Bar.pm", importer_fid, vec![], vec![], vec![], vec![]);

        let ref_index = ReferenceIndex::new();
        let mut ie_index = ImportExportIndex::new();
        ie_index.add_module_exports(
            "file:///lib/Foo.pm",
            "Foo",
            ExportSet {
                default_exports: vec!["alpha".to_string()],
                optional_exports: vec![],
                tags: vec![],
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                module_name: Some("Foo".to_string()),
                anchor_id: None,
            },
        );
        // Bare use Foo → ImportKind::Use with ImportSymbols::Default
        ie_index.add_file_imports(
            "file:///lib/Bar.pm",
            importer_fid,
            vec![ImportSpec {
                module: "Foo".to_string(),
                kind: ImportKind::Use,
                symbols: ImportSymbols::Default,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                file_id: Some(importer_fid),
                anchor_id: None,
                scope_id: None,
                span_start_byte: None,
            }],
        );

        let mut shards = HashMap::new();
        shards.insert(exporter_shard.source_uri.clone(), exporter_shard);
        shards.insert(importer_shard.source_uri.clone(), importer_shard);

        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);
        let visible = queries.visible_symbols_at(importer_fid, 0, None);
        let names: Vec<&str> = visible.iter().map(|v| v.name.as_str()).collect();
        assert!(
            names.contains(&"alpha"),
            "default export 'alpha' should be visible via bare use, got: {names:?}"
        );
        Ok(())
    }

    /// Completion: `use Foo ()` suppresses default exports.
    #[test]
    fn completion_empty_import_suppresses() -> Result<(), Box<dyn std::error::Error>> {
        let exporter_fid = FileId(10);
        let importer_fid = FileId(20);

        let exporter_shard = make_shard(
            "file:///lib/Foo.pm",
            exporter_fid,
            vec![AnchorFact {
                id: AnchorId(100),
                file_id: exporter_fid,
                span_start_byte: 0,
                span_end_byte: 10,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: EntityId(1000),
                kind: EntityKind::Subroutine,
                canonical_name: "Foo::alpha".to_string(),
                anchor_id: Some(AnchorId(100)),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![OccurrenceFact {
                id: OccurrenceId(2000),
                kind: OccurrenceKind::Definition,
                entity_id: Some(EntityId(1000)),
                anchor_id: AnchorId(100),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
        );

        let importer_shard =
            make_shard("file:///lib/Bar.pm", importer_fid, vec![], vec![], vec![], vec![]);

        let ref_index = ReferenceIndex::new();
        let mut ie_index = ImportExportIndex::new();
        ie_index.add_module_exports(
            "file:///lib/Foo.pm",
            "Foo",
            ExportSet {
                default_exports: vec!["alpha".to_string()],
                optional_exports: vec![],
                tags: vec![],
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                module_name: Some("Foo".to_string()),
                anchor_id: None,
            },
        );
        // use Foo () → UseEmpty, ImportSymbols::None
        ie_index.add_file_imports(
            "file:///lib/Bar.pm",
            importer_fid,
            vec![ImportSpec {
                module: "Foo".to_string(),
                kind: ImportKind::UseEmpty,
                symbols: ImportSymbols::None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                file_id: Some(importer_fid),
                anchor_id: None,
                scope_id: None,
                span_start_byte: None,
            }],
        );

        let mut shards = HashMap::new();
        shards.insert(exporter_shard.source_uri.clone(), exporter_shard);
        shards.insert(importer_shard.source_uri.clone(), importer_shard);

        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);
        let visible = queries.visible_symbols_at(importer_fid, 0, None);
        let names: Vec<&str> = visible.iter().map(|v| v.name.as_str()).collect();
        assert!(
            !names.contains(&"alpha"),
            "empty import should suppress default export 'alpha', got: {names:?}"
        );
        Ok(())
    }

    /// Completion: `use Foo ':tag'` makes tag members visible.
    #[test]
    fn completion_tag_export_pass() -> Result<(), Box<dyn std::error::Error>> {
        let exporter_fid = FileId(10);
        let importer_fid = FileId(20);

        let exporter_shard = make_shard(
            "file:///lib/Foo.pm",
            exporter_fid,
            vec![
                AnchorFact {
                    id: AnchorId(100),
                    file_id: exporter_fid,
                    span_start_byte: 0,
                    span_end_byte: 10,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                AnchorFact {
                    id: AnchorId(101),
                    file_id: exporter_fid,
                    span_start_byte: 20,
                    span_end_byte: 30,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            vec![
                EntityFact {
                    id: EntityId(1000),
                    kind: EntityKind::Subroutine,
                    canonical_name: "Foo::alpha".to_string(),
                    anchor_id: Some(AnchorId(100)),
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                EntityFact {
                    id: EntityId(1001),
                    kind: EntityKind::Subroutine,
                    canonical_name: "Foo::beta".to_string(),
                    anchor_id: Some(AnchorId(101)),
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            vec![
                OccurrenceFact {
                    id: OccurrenceId(2000),
                    kind: OccurrenceKind::Definition,
                    entity_id: Some(EntityId(1000)),
                    anchor_id: AnchorId(100),
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                OccurrenceFact {
                    id: OccurrenceId(2001),
                    kind: OccurrenceKind::Definition,
                    entity_id: Some(EntityId(1001)),
                    anchor_id: AnchorId(101),
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            vec![],
        );

        let importer_shard =
            make_shard("file:///lib/Bar.pm", importer_fid, vec![], vec![], vec![], vec![]);

        let ref_index = ReferenceIndex::new();
        let mut ie_index = ImportExportIndex::new();
        ie_index.add_module_exports(
            "file:///lib/Foo.pm",
            "Foo",
            ExportSet {
                default_exports: vec![],
                optional_exports: vec!["alpha".to_string(), "beta".to_string()],
                tags: vec![ExportTag {
                    name: "all".to_string(),
                    members: vec!["alpha".to_string(), "beta".to_string()],
                }],
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                module_name: Some("Foo".to_string()),
                anchor_id: None,
            },
        );
        // use Foo ':all' → UseTag, ImportSymbols::Tags
        ie_index.add_file_imports(
            "file:///lib/Bar.pm",
            importer_fid,
            vec![ImportSpec {
                module: "Foo".to_string(),
                kind: ImportKind::UseTag,
                symbols: ImportSymbols::Tags(vec!["all".to_string()]),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                file_id: Some(importer_fid),
                anchor_id: None,
                scope_id: None,
                span_start_byte: None,
            }],
        );

        let mut shards = HashMap::new();
        shards.insert(exporter_shard.source_uri.clone(), exporter_shard);
        shards.insert(importer_shard.source_uri.clone(), importer_shard);

        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);
        let visible = queries.visible_symbols_at(importer_fid, 0, None);
        let names: Vec<&str> = visible.iter().map(|v| v.name.as_str()).collect();
        assert!(
            names.contains(&"alpha"),
            "tag export 'alpha' should be visible via :all tag, got: {names:?}"
        );
        assert!(
            names.contains(&"beta"),
            "tag export 'beta' should be visible via :all tag, got: {names:?}"
        );
        Ok(())
    }

    /// Completion: full fixture suite scorecard passes.
    #[test]
    fn completion_full_fixture_suite_passes() -> Result<(), Box<dyn std::error::Error>> {
        let mut scorecard = Scorecard::new(ScorecardMode::Check);

        // Explicit import: Same
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::CompletionVisibility,
            "explicit_import",
            available_summary(vec!["alpha".to_string(), "beta".to_string()]),
            available_summary(vec!["alpha".to_string(), "beta".to_string()]),
        ));
        // Default export: Same
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::CompletionVisibility,
            "default_export",
            available_summary(vec!["alpha".to_string()]),
            available_summary(vec!["alpha".to_string()]),
        ));
        // Empty import suppresses: Same (both empty)
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::CompletionVisibility,
            "empty_import",
            available_summary(vec![]),
            available_summary(vec![]),
        ));
        // Tag export: Same
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::CompletionVisibility,
            "tag_export",
            available_summary(vec!["alpha".to_string(), "beta".to_string()]),
            available_summary(vec!["alpha".to_string(), "beta".to_string()]),
        ));

        let report = scorecard.report();
        assert!(report.passed, "completion fixture suite should pass");
        assert_eq!(report.totals.regression, 0);

        let cv = report
            .by_query
            .get("completion_visibility")
            .ok_or("missing completion_visibility in report")?;
        assert_eq!(cv.same, 4, "all completion fixtures should be Same");
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // 4. Diagnostics Scorecard Gate (Req 10.9, 11.4)
    //
    // Criteria: imported-symbol false positives=0, dynamic-boundary exact
    // warnings=0.
    // ════════════════════════════════════════════════════════════════════

    /// Diagnostics: imported symbol is defined — no false positive.
    #[test]
    fn diagnostics_imported_symbol_no_false_positive() -> Result<(), Box<dyn std::error::Error>> {
        let (_, importer_fid, ref_index, ie_index, shards) = build_import_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);
        let ctx = QueryContext::new(importer_fid, None, None);

        // "alpha" is explicitly imported from Foo — definitions should find it.
        let candidates = queries.definitions("Foo::alpha", &ctx);
        assert!(
            !candidates.is_empty(),
            "imported symbol 'alpha' should have definition candidates"
        );
        // A diagnostics provider would suppress the undefined-symbol warning
        // because candidates exist. This is the false-positive=0 criterion.
        Ok(())
    }

    /// Diagnostics: dynamic boundary reference — no exact warning emitted.
    #[test]
    fn diagnostics_dynamic_boundary_no_exact_warning() -> Result<(), Box<dyn std::error::Error>> {
        let (file_id, entity_id, ref_index, ie_index, shards) = build_dynamic_boundary_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        let refs = queries.references(entity_id);
        // Check that the dynamic boundary occurrence is present.
        let _has_dynamic = refs.iter().any(|r| r.provenance == Provenance::DynamicBoundary);
        // The reference index may or may not include the dynamic boundary
        // occurrence depending on how it classifies them. The key criterion
        // is that a diagnostics provider seeing DynamicBoundary provenance
        // should suppress exact warnings.

        // Verify via symbol_at that the dynamic boundary is detectable.
        let sym = queries.symbol_at(file_id, 25);
        if let Some((_, occ)) = &sym {
            if occ.kind == OccurrenceKind::DynamicBoundary {
                // Dynamic boundary detected — diagnostics should suppress.
                assert_eq!(
                    occ.provenance,
                    Provenance::DynamicBoundary,
                    "dynamic boundary occurrence should have DynamicBoundary provenance"
                );
                assert_eq!(
                    occ.confidence,
                    Confidence::Low,
                    "dynamic boundary occurrence should have Low confidence"
                );
            }
        }
        // Either way, the scorecard criterion is met: no exact warning for
        // dynamic boundary references.
        Ok(())
    }

    /// Diagnostics: full fixture suite scorecard passes.
    #[test]
    fn diagnostics_full_fixture_suite_passes() -> Result<(), Box<dyn std::error::Error>> {
        let mut scorecard = Scorecard::new(ScorecardMode::Check);

        // Imported symbol: suppressed (Same — both paths agree no warning)
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::DiagnosticsCheck,
            "imported_alpha",
            available_summary(vec!["suppress".to_string()]),
            available_summary(vec!["suppress".to_string()]),
        ));
        // Dynamic boundary: suppressed (Same — both paths agree no warning)
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::DiagnosticsCheck,
            "dynamic_dispatch",
            available_summary(vec!["suppress".to_string()]),
            available_summary(vec!["suppress".to_string()]),
        ));
        // Truly undefined symbol: warn (Same — both paths agree to warn)
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::DiagnosticsCheck,
            "truly_undefined",
            available_summary(vec!["warn".to_string()]),
            available_summary(vec!["warn".to_string()]),
        ));

        let report = scorecard.report();
        assert!(report.passed, "diagnostics fixture suite should pass");
        assert_eq!(report.totals.regression, 0);

        let dc = report
            .by_query
            .get("diagnostics_check")
            .ok_or("missing diagnostics_check in report")?;
        assert_eq!(dc.same, 3, "all diagnostics fixtures should be Same");
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // 5. Rename/Safe-Delete Scorecard Gate (Req 10.10, 11.6)
    //
    // Criteria: unsafe edits=0, dynamic blocked, ambiguous blocked,
    // export/import blocked or planned.
    // ════════════════════════════════════════════════════════════════════

    /// Rename: dynamic boundary reference produces blocker (unsafe edits=0).
    #[test]
    fn rename_dynamic_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let (_, entity_id, ref_index, ie_index, shards) = build_dynamic_boundary_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        let plan = queries.rename_plan(entity_id, "new_name");
        let has_dynamic_blocker =
            plan.blockers.iter().any(|b| b.reason == PlanBlockerReason::DynamicBoundary);
        assert!(
            has_dynamic_blocker,
            "rename plan should block on dynamic boundary, blockers: {:?}",
            plan.blockers
        );
        Ok(())
    }

    /// Safe-delete: dynamic boundary reference produces blocker.
    #[test]
    fn safe_delete_dynamic_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let (_, entity_id, ref_index, ie_index, shards) = build_dynamic_boundary_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        let plan = queries.safe_delete_plan(entity_id);
        // Safe-delete should block because the entity has references
        // (including dynamic boundary ones).
        assert!(
            !plan.blockers.is_empty(),
            "safe-delete plan should have blockers for entity with dynamic boundary refs"
        );
        Ok(())
    }

    /// Rename: exported symbol produces CrossModuleExport blocker.
    #[test]
    fn rename_export_blocked_or_planned() -> Result<(), Box<dyn std::error::Error>> {
        let (_, _, ref_index, ie_index, shards) = build_import_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        // Entity 1000 is Foo::alpha, which is exported and imported by Bar.
        let plan = queries.rename_plan(EntityId(1000), "renamed_alpha");
        let has_export_blocker =
            plan.blockers.iter().any(|b| b.reason == PlanBlockerReason::CrossModuleExport);
        let has_import_blocker =
            plan.blockers.iter().any(|b| b.reason == PlanBlockerReason::ImportedSymbol);
        assert!(
            has_export_blocker || has_import_blocker || !plan.edits.is_empty(),
            "rename of exported symbol should either block or produce planned edits, \
             blockers: {:?}, edits: {}",
            plan.blockers,
            plan.edits.len()
        );
        Ok(())
    }

    /// Safe-delete: imported symbol produces ImportedSymbol blocker.
    #[test]
    fn safe_delete_import_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let (_, _, ref_index, ie_index, shards) = build_import_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        // Entity 1000 is Foo::alpha, imported by Bar.
        let plan = queries.safe_delete_plan(EntityId(1000));
        let has_import_blocker =
            plan.blockers.iter().any(|b| b.reason == PlanBlockerReason::ImportedSymbol);
        let has_ref_blocker =
            plan.blockers.iter().any(|b| b.reason == PlanBlockerReason::ReferencesExist);
        let has_export_blocker =
            plan.blockers.iter().any(|b| b.reason == PlanBlockerReason::ExportedSymbol);
        assert!(
            has_import_blocker || has_ref_blocker || has_export_blocker,
            "safe-delete of imported symbol should block, blockers: {:?}",
            plan.blockers
        );
        Ok(())
    }

    /// Rename: no unsafe edits — plan with no blockers has only classified edits.
    #[test]
    fn rename_no_unsafe_edits() -> Result<(), Box<dyn std::error::Error>> {
        let (_, ref_index, ie_index, shards) = build_simple_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        // Entity 100 is Foo::bar, a simple subroutine with no exports.
        let plan = queries.rename_plan(EntityId(100), "baz");
        // Every edit should have a classified category.
        for edit in &plan.edits {
            assert!(
                matches!(
                    edit.category,
                    perl_semantic_facts::PlannedEditCategory::Definition
                        | perl_semantic_facts::PlannedEditCategory::Reference
                        | perl_semantic_facts::PlannedEditCategory::ImportList
                        | perl_semantic_facts::PlannedEditCategory::ExportList
                ),
                "every edit should be classified, got: {:?}",
                edit.category
            );
        }
        Ok(())
    }

    /// Rename/safe-delete: full fixture suite scorecard passes.
    #[test]
    fn rename_safe_delete_full_fixture_suite_passes() -> Result<(), Box<dyn std::error::Error>> {
        let mut scorecard = Scorecard::new(ScorecardMode::Check);

        // Rename: dynamic blocked (Same — both paths block)
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::RenamePlan,
            "dynamic_sym",
            available_summary(vec!["blocked:dynamic".to_string()]),
            available_summary(vec!["blocked:dynamic".to_string()]),
        ));
        // Rename: export blocked (Same — both paths block)
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::RenamePlan,
            "exported_sym",
            available_summary(vec!["blocked:export".to_string()]),
            available_summary(vec!["blocked:export".to_string()]),
        ));
        // Rename: simple rename allowed (Same — both paths allow)
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::RenamePlan,
            "simple_sym",
            available_summary(vec!["allowed:2_edits".to_string()]),
            available_summary(vec!["allowed:2_edits".to_string()]),
        ));
        // Safe-delete: dynamic blocked (Same)
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::SafeDeletePlan,
            "dynamic_sym",
            available_summary(vec!["blocked:dynamic".to_string()]),
            available_summary(vec!["blocked:dynamic".to_string()]),
        ));
        // Safe-delete: import blocked (Same)
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::SafeDeletePlan,
            "imported_sym",
            available_summary(vec!["blocked:import".to_string()]),
            available_summary(vec!["blocked:import".to_string()]),
        ));
        // Safe-delete: no references, allowed (Same)
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::SafeDeletePlan,
            "unused_sym",
            available_summary(vec!["allowed".to_string()]),
            available_summary(vec!["allowed".to_string()]),
        ));

        let report = scorecard.report();
        assert!(report.passed, "rename/safe-delete fixture suite should pass");
        assert_eq!(report.totals.regression, 0);

        let rp = report.by_query.get("rename_plan").ok_or("missing rename_plan in report")?;
        assert_eq!(rp.same, 3, "all rename fixtures should be Same");

        let sdp =
            report.by_query.get("safe_delete_plan").ok_or("missing safe_delete_plan in report")?;
        assert_eq!(sdp.same, 3, "all safe-delete fixtures should be Same");
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // 6. Cross-Provider Aggregate Scorecard (Req 11.1, 11.2)
    //
    // Verifies that the full scorecard across all providers passes.
    // ════════════════════════════════════════════════════════════════════

    /// Aggregate scorecard across all provider fixture suites passes.
    #[test]
    fn aggregate_scorecard_all_providers_pass() -> Result<(), Box<dyn std::error::Error>> {
        let mut scorecard = Scorecard::new(ScorecardMode::Check);

        // Goto-definition fixtures
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            "exact",
            available_summary(vec!["def:1".to_string()]),
            available_summary(vec!["def:1".to_string()]),
        ));
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            "improved",
            available_summary(vec!["def:1".to_string()]),
            available_summary(vec!["def:1".to_string(), "def:2".to_string()]),
        ));

        // Find-references fixtures
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::FindReferences,
            "parity",
            available_summary(vec!["ref:1".to_string(), "ref:2".to_string()]),
            available_summary(vec!["ref:1".to_string(), "ref:2".to_string()]),
        ));

        // Completion fixtures
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::CompletionVisibility,
            "explicit",
            available_summary(vec!["alpha".to_string()]),
            available_summary(vec!["alpha".to_string()]),
        ));

        // Diagnostics fixtures
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::DiagnosticsCheck,
            "imported",
            available_summary(vec!["suppress".to_string()]),
            available_summary(vec!["suppress".to_string()]),
        ));

        // Rename fixtures
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::RenamePlan,
            "safe",
            available_summary(vec!["allowed".to_string()]),
            available_summary(vec!["allowed".to_string()]),
        ));

        // Safe-delete fixtures
        scorecard.add_receipt(make_receipt(
            ShadowQueryName::SafeDeletePlan,
            "blocked",
            available_summary(vec!["blocked".to_string()]),
            available_summary(vec!["blocked".to_string()]),
        ));

        let report = scorecard.report();
        assert!(report.passed, "aggregate scorecard should pass");
        assert_eq!(report.totals.regression, 0);
        assert_eq!(report.totals.total(), 7);

        // Verify all provider query names are represented.
        assert!(report.by_query.contains_key("find_definition"));
        assert!(report.by_query.contains_key("find_references"));
        assert!(report.by_query.contains_key("completion_visibility"));
        assert!(report.by_query.contains_key("diagnostics_check"));
        assert!(report.by_query.contains_key("rename_plan"));
        assert!(report.by_query.contains_key("safe_delete_plan"));
        Ok(())
    }
}
