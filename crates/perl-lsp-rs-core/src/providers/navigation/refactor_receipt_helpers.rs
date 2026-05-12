//! Shared receipt formatting helpers for refactor provider proofs.

use perl_semantic_facts::{PlanBlocker, PlanBlockerReason};

pub(crate) fn blocker_reason_list(blockers: &[PlanBlocker]) -> String {
    if blockers.is_empty() {
        return "none".to_string();
    }

    blockers
        .iter()
        .map(|blocker| blocker_reason_label(blocker.reason))
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn blocker_ux_list(blockers: &[PlanBlocker]) -> String {
    if blockers.is_empty() {
        return "none".to_string();
    }

    blockers.iter().map(|blocker| blocker.description.as_str()).collect::<Vec<_>>().join(" | ")
}

pub(crate) fn blocker_reason_label(reason: PlanBlockerReason) -> &'static str {
    match reason {
        PlanBlockerReason::DynamicBoundary => "dynamic_boundary",
        PlanBlockerReason::AmbiguousReference => "ambiguous_reference",
        PlanBlockerReason::CrossModuleExport => "cross_module_export",
        PlanBlockerReason::ImportedSymbol => "imported_symbol",
        PlanBlockerReason::ExportedSymbol => "exported_symbol",
        PlanBlockerReason::ReferencesExist => "references_exist",
        PlanBlockerReason::GeneratedMember => "generated_member",
        PlanBlockerReason::StaleFact => "stale_fact",
        PlanBlockerReason::UnclassifiedOccurrence => "unclassified_occurrence",
        _ => "unknown",
    }
}
