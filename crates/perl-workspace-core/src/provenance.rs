//! Provenance and confidence — the honesty layer.
//!
//! Perl demands more traceability than most languages: a fact might come from
//! exact AST, from parser recovery, from distribution metadata, or from a
//! heuristic. Every fact records **where it came from** ([`EvidenceSource`]) and
//! **how sure we are** ([`Confidence`]), so a consumer can decide whether to act
//! on it (see PLSP-ADR-0006).

use serde::{Deserialize, Serialize};

/// How much to trust a fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    /// Derived from exact, unambiguous evidence (e.g. a parsed `package`).
    High,
    /// Derived from partial or recovered evidence.
    Medium,
    /// Heuristic or best-effort; may be wrong.
    Low,
}

/// The kind of evidence a fact was derived from.
///
/// Ordered roughly from most to least authoritative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    /// Exact AST node from a clean parse.
    ExactAst,
    /// A node salvaged by parser error recovery.
    ParserRecovery,
    /// High-level IR / semantic projection.
    Hir,
    /// A compile-time effect (`use strict`, `use feature`, …).
    CompileEffect,
    /// Distribution metadata (`META.json`, `cpanfile`, …).
    DistMetadata,
    /// POD documentation.
    Pod,
    /// TAP output from a test run.
    TapOutput,
    /// Observed at runtime.
    RuntimeObserved,
    /// An external conformance oracle (perldoc, perlcritic parity, …).
    ExternalConformance,
    /// A heuristic or string scan.
    Heuristic,
}

/// The tool that produced a fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Producer {
    /// Producer name (e.g. `"perl-workspace-core"`).
    pub name: String,
    /// Producer version (crate version).
    pub version: String,
    /// The fact-schema version the producer emits.
    pub schema_version: u32,
}

impl Producer {
    /// The current `perl-workspace-core` producer identity.
    #[must_use]
    pub fn workspace_core() -> Self {
        Self {
            name: "perl-workspace-core".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version: crate::SCHEMA_VERSION,
        }
    }
}

/// A provenance record: producer + evidence source + confidence.
///
/// Facts reference a `Provenance` rather than embedding it, so many facts from
/// the same producer/source share one record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// The tool that produced the fact.
    pub producer: Producer,
    /// What kind of evidence backs the fact.
    pub source: EvidenceSource,
    /// How confident the producer is.
    pub confidence: Confidence,
}

impl Provenance {
    /// Provenance for a fact derived from a clean parse by this crate.
    #[must_use]
    pub fn exact_ast(confidence: Confidence) -> Self {
        Self { producer: Producer::workspace_core(), source: EvidenceSource::ExactAst, confidence }
    }

    /// Provenance for a fact salvaged from parser recovery by this crate.
    #[must_use]
    pub fn parser_recovery() -> Self {
        Self {
            producer: Producer::workspace_core(),
            source: EvidenceSource::ParserRecovery,
            confidence: Confidence::Medium,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_serializes_lowercase() {
        let json = serde_json::to_string(&Confidence::High).unwrap();
        assert_eq!(json, "\"high\"");
    }

    #[test]
    fn evidence_source_serializes_snake_case() {
        let json = serde_json::to_string(&EvidenceSource::ExactAst).unwrap();
        assert_eq!(json, "\"exact_ast\"");
        let json = serde_json::to_string(&EvidenceSource::DistMetadata).unwrap();
        assert_eq!(json, "\"dist_metadata\"");
    }

    #[test]
    fn workspace_core_producer_carries_crate_version() {
        let p = Producer::workspace_core();
        assert_eq!(p.name, "perl-workspace-core");
        assert_eq!(p.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(p.schema_version, crate::SCHEMA_VERSION);
    }

    #[test]
    fn exact_ast_provenance_records_source() {
        let p = Provenance::exact_ast(Confidence::High);
        assert_eq!(p.source, EvidenceSource::ExactAst);
        assert_eq!(p.confidence, Confidence::High);
    }
}
