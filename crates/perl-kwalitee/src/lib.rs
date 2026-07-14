//! # perl-kwalitee
//!
//! A **Perl distribution Kwalitee evaluator** for the perl-lsp native stack.
//!
//! "Kwalitee" (borrowed from CPAN's `Module::CPANTS`) is *measurable
//! distribution quality* — objective, checkable indicators about how a
//! distribution is shipped — as distinct from subjective code quality. This
//! crate owns the indicator model, scoring, profiles, JSON receipt schema, and
//! Markdown rendering. The `cargo xtask perl-kwalitee` command wires repository
//! paths, existing xtask gate results, and CI ergonomics into it.
//!
//! ## Design
//!
//! The crate is **pure**: every indicator is evaluated either from the
//! repository filesystem (manifests, first-mile surfaces) or by reading a JSON
//! receipt another tool produced. It never spawns a subprocess or touches the
//! network. Heavier gates that genuinely need to run (release archive
//! validation, the runCritic parity test, `update-status --check`) are executed
//! by the caller and fed in as [`ExternalResult`]s. This keeps evaluation
//! deterministic and unit-testable.
//!
//! ## Example
//!
//! ```no_run
//! use perl_kwalitee::{evaluate, KwaliteeOptions, KwaliteeProfile};
//!
//! let options = KwaliteeOptions::new("/path/to/repo", KwaliteeProfile::Pr);
//! let receipt = evaluate(&options);
//! println!("verdict: {} score: {}", receipt.verdict.label(), receipt.score);
//! println!("{}", receipt.to_markdown());
//! ```

#![warn(missing_docs)]
// Production code stays under the workspace `unwrap_used`/`expect_used` deny;
// test code may use `expect()`/`unwrap()` for brevity. Matches the precedent in
// perl-incremental-parsing and perl-lsp-perltidy, and keeps `cargo clippy
// --all-targets` green without threading `Result`/`must` through every fixture.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod evaluator;
mod evidence;
mod indicator;
mod profile;
mod receipt;
mod score;

pub use evaluator::{EvidencePaths, ExternalResult, KwaliteeOptions, evaluate, is_known_indicator};
pub use indicator::{
    EvidenceRef, IndicatorExplanation, IndicatorStatus, KwaliteeIndicator, explain, indicator_ids,
};
pub use profile::KwaliteeProfile;
pub use receipt::{KwaliteeReceipt, KwaliteeVerdict, RECEIPT_KIND, SCHEMA_VERSION};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_api_is_reachable() {
        // A smoke test that the re-exported surface composes.
        let opts = KwaliteeOptions::new(".", KwaliteeProfile::Pr);
        let receipt = evaluate(&opts);
        assert_eq!(receipt.kind, RECEIPT_KIND);
        assert_eq!(receipt.schema_version, SCHEMA_VERSION);
        assert!(!indicator_ids().is_empty());
        assert!(explain(indicator_ids()[0]).is_some());
        assert!(is_known_indicator("license.declared"));
    }
}
