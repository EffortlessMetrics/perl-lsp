//! Lexical reference extraction from PIR-lowered HirFile.
//!
//! This module defines the lexical fact extractor, which walks a per-body PIR lowering
//! and collects all lexical reads and writes, grouped by body boundary to preserve scope isolation.
//!
//! The extractor is used by PR2 (#2634) for shadow comparison and later by providers for
//! navigation and reference detection. PR1 defines the core API only; no provider changes.

use crate::hir::{BodyOwnerKind, HirBodyId, HirFile};
use crate::pir::lower::lower_single_body;
use crate::pir::model::{LexicalName, PirOperation, PirSourceAnchor};

/// Current schema version for lexical extractor receipts.
pub const LEXICAL_EXTRACTOR_RECEIPT_VERSION: u32 = 1;

/// A single lexical variable binding fact extracted from PIR.
///
/// Each fact represents one read or write of a lexical (`my`/`state`) variable,
/// anchored to its source range and discriminated by body identity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LexicalBindingFact {
    /// The lexical variable (sigil + name).
    pub name: LexicalName,
    /// Whether this fact records a read or write.
    pub role: LexicalRole,
    /// Source anchor for this reference (always anchored in PR1).
    pub source_anchor: PirSourceAnchor,
    /// Body index where this binding occurs (0-based).
    pub body_idx: usize,
    /// Body owner kind (what construct owns this body).
    pub body_owner: BodyOwnerKind,
}

/// Role of a lexical binding fact (read or write).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LexicalRole {
    /// Read access to a lexical variable.
    Read,
    /// Write access (declaration or assignment) to a lexical variable.
    Write,
}

/// Extraction results for a single body.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BodyExtractionResult {
    /// Index of this body (0-based, matches position in HirFile::bodies).
    pub body_idx: usize,
    /// Owner of this body (ProgramRoot or Subroutine).
    pub owner: BodyOwnerKind,
    /// All lexical binding facts in this body, in lowering order.
    pub facts: Vec<LexicalBindingFact>,
    /// Count of nodes that had source anchors.
    pub anchored_node_count: usize,
    /// Total count of all nodes processed (anchored + non-anchored).
    pub total_node_count: usize,
}

/// Receipt from the lexical extractor, summarizing all bindings extracted from a file.
///
/// This is distinct from [`PirReceipt`](crate::pir::PirReceipt), which models lowering stats.
/// `LexicalExtractorReceipt` is a compiler-substrate proof surface: it records what the
/// extractor found, not what the lowerer did.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LexicalExtractorReceipt {
    /// Schema version (currently 1).
    pub schema_version: u32,
    /// Per-body extraction results, in order.
    pub bodies: Vec<BodyExtractionResult>,
    /// Total count of lexical reads across all bodies.
    pub total_read_count: usize,
    /// Total count of lexical writes across all bodies.
    pub total_write_count: usize,
    /// Count of operations skipped (Modify, StashModify).
    pub skipped_node_count: usize,
    /// Whether provider behavior changed (always false for PR1).
    pub provider_behavior_changed: bool,
}

/// Extract lexical binding facts from a HirFile.
///
/// Lowers each body independently and collects all `LexicalRead` and `LexicalWrite`
/// operations, preserving body boundaries so scope isolation is unambiguous.
///
/// # Arguments
///
/// * `file` - A HIR file with populated `.bodies` (from the second pass of HIR lowering)
///
/// # Returns
///
/// A receipt containing all facts grouped by body, plus summary counts.
///
/// # Invariants
///
/// - All emitted facts have `source_anchor.is_anchored() == true`
/// - `Modify`/`StashModify` operations are skipped (not counted as facts, but tracked in `skipped_node_count`)
/// - `StashRead`/`StashWrite` are ignored (not extracted in PR1)
/// - `provider_behavior_changed` is always `false`
/// - `total_read_count + total_write_count == sum(bodies[].facts.len())`
#[must_use]
pub fn extract_lexical_facts(file: &HirFile) -> LexicalExtractorReceipt {
    let mut bodies = Vec::new();
    let mut total_read_count = 0usize;
    let mut total_write_count = 0usize;
    let mut skipped_node_count = 0usize;

    for (body_idx, body) in file.bodies.iter().enumerate() {
        let owner = body.owner.clone();
        let mut facts = Vec::new();
        let mut anchored_node_count = 0usize;
        let mut total_node_count = 0usize;

        // Lower this body in isolation — fresh lowerer per body preserves scope boundaries.
        let pir_nodes = lower_single_body(body, HirBodyId(body_idx as u32), file);

        for pir_node in pir_nodes {
            total_node_count += 1;

            match &pir_node.operation {
                PirOperation::LexicalRead { name } => {
                    if pir_node.source_anchor.is_anchored() {
                        anchored_node_count += 1;
                        facts.push(LexicalBindingFact {
                            name: name.clone(),
                            role: LexicalRole::Read,
                            source_anchor: pir_node.source_anchor.clone(),
                            body_idx,
                            body_owner: owner.clone(),
                        });
                        total_read_count += 1;
                    }
                }
                PirOperation::LexicalWrite { name } => {
                    if pir_node.source_anchor.is_anchored() {
                        anchored_node_count += 1;
                        facts.push(LexicalBindingFact {
                            name: name.clone(),
                            role: LexicalRole::Write,
                            source_anchor: pir_node.source_anchor.clone(),
                            body_idx,
                            body_owner: owner.clone(),
                        });
                        total_write_count += 1;
                    }
                }
                PirOperation::Modify { .. } | PirOperation::StashModify { .. } => {
                    // Modify and StashModify are explicitly skipped: they are neither Read nor
                    // Write in the lexical-fact model (they represent compound RMW operations).
                    // Track them so the receipt surface is honest about what was filtered.
                    skipped_node_count += 1;
                }
                // StashRead, StashWrite, Call, MethodCall, Assign, DynamicBoundary, etc.
                // are outside PR1 scope — silently ignored (not facts, not skipped).
                _ => {}
            }
        }

        bodies.push(BodyExtractionResult {
            body_idx,
            owner,
            facts,
            anchored_node_count,
            total_node_count,
        });
    }

    LexicalExtractorReceipt {
        schema_version: LEXICAL_EXTRACTOR_RECEIPT_VERSION,
        bodies,
        total_read_count,
        total_write_count,
        skipped_node_count,
        provider_behavior_changed: false,
    }
}
