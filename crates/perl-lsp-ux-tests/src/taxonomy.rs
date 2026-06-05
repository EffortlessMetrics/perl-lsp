//! Shared UX readiness enum taxonomy.
//!
//! Defined once, consumed by both the in-process `UxRunRecorder`
//! and the xtask `UxRegressionReceiptEmitter`.

use serde::{Deserialize, Serialize};

/// Classification of why a UX scenario failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UxFailureClass {
    ProviderRegression,
    ServerCrash,
    Timeout,
    TestRace,
    Infra,
    MatrixDrift,
    BaselineDrift,
    NewTestBug,
    Unknown,
}

/// Semantic routing hint for failure investigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UxRoute {
    CiInvestigation,
    FixtureUpdate,
    TestFix,
    ProviderFix,
    Triage,
    BaselineUpdate,
    CrashFix,
    TimeoutTriage,
}

/// CI execution tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UxCiTier {
    Pr,
    Nightly,
    Release,
}

/// Outcome of a UX scenario execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UxScenarioResult {
    Pass,
    Fail,
    Quarantined,
    Skipped,
}

/// State of a metric value in the scorecard.
///
/// `InsufficientData` represents missing or below-threshold receipt data —
/// it is a metric state, not a scenario execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MetricState<T> {
    Measured { value: T, sample_count: usize },
    InsufficientData { reason: String },
}

/// Component subsystem that a scenario exercises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UxComponent {
    Completion,
    Diagnostics,
    ModuleResolution,
    WorkspaceSymbols,
    Rename,
    SafeDelete,
    Hover,
    GotoDefinition,
    SignatureHelp,
    CodeLens,
    FoldingRange,
    SemanticTokens,
    CodeActions,
    Infra,
    AiCompletion,
}

/// Map a failure class to its semantic route.
pub fn route_for_failure_class(class: UxFailureClass) -> UxRoute {
    match class {
        UxFailureClass::ProviderRegression => UxRoute::ProviderFix,
        UxFailureClass::ServerCrash => UxRoute::CrashFix,
        UxFailureClass::Timeout => UxRoute::TimeoutTriage,
        UxFailureClass::Infra => UxRoute::CiInvestigation,
        UxFailureClass::MatrixDrift => UxRoute::FixtureUpdate,
        UxFailureClass::BaselineDrift => UxRoute::BaselineUpdate,
        UxFailureClass::TestRace | UxFailureClass::NewTestBug => UxRoute::TestFix,
        UxFailureClass::Unknown => UxRoute::Triage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Exhaustive route mapping table test ──────────────────────────────

    #[test]
    fn route_for_failure_class_exhaustive_mapping() -> Result<(), Box<dyn std::error::Error>> {
        let table: &[(UxFailureClass, UxRoute)] = &[
            (UxFailureClass::ProviderRegression, UxRoute::ProviderFix),
            (UxFailureClass::ServerCrash, UxRoute::CrashFix),
            (UxFailureClass::Timeout, UxRoute::TimeoutTriage),
            (UxFailureClass::Infra, UxRoute::CiInvestigation),
            (UxFailureClass::MatrixDrift, UxRoute::FixtureUpdate),
            (UxFailureClass::BaselineDrift, UxRoute::BaselineUpdate),
            (UxFailureClass::TestRace, UxRoute::TestFix),
            (UxFailureClass::NewTestBug, UxRoute::TestFix),
            (UxFailureClass::Unknown, UxRoute::Triage),
        ];

        for &(class, expected_route) in table {
            let actual = route_for_failure_class(class);
            assert_eq!(
                actual, expected_route,
                "route_for_failure_class({class:?}) = {actual:?}, expected {expected_route:?}"
            );
        }

        Ok(())
    }

    // ── Serde round-trip helpers ─────────────────────────────────────────

    /// Serialize to JSON then deserialize back, asserting equality.
    fn serde_round_trip<T>(value: &T) -> Result<(), Box<dyn std::error::Error>>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
    {
        let json = serde_json::to_string(value)?;
        let back: T = serde_json::from_str(&json)?;
        assert_eq!(&back, value, "round-trip failed for {value:?}: serialized as {json}");
        Ok(())
    }

    // ── UxFailureClass serde round-trip ──────────────────────────────────

    #[test]
    fn serde_round_trip_ux_failure_class() -> Result<(), Box<dyn std::error::Error>> {
        let variants = [
            UxFailureClass::ProviderRegression,
            UxFailureClass::ServerCrash,
            UxFailureClass::Timeout,
            UxFailureClass::TestRace,
            UxFailureClass::Infra,
            UxFailureClass::MatrixDrift,
            UxFailureClass::BaselineDrift,
            UxFailureClass::NewTestBug,
            UxFailureClass::Unknown,
        ];
        for variant in &variants {
            serde_round_trip(variant)?;
        }
        Ok(())
    }

    // ── UxRoute serde round-trip ─────────────────────────────────────────

    #[test]
    fn serde_round_trip_ux_route() -> Result<(), Box<dyn std::error::Error>> {
        let variants = [
            UxRoute::CiInvestigation,
            UxRoute::FixtureUpdate,
            UxRoute::TestFix,
            UxRoute::ProviderFix,
            UxRoute::Triage,
            UxRoute::BaselineUpdate,
            UxRoute::CrashFix,
            UxRoute::TimeoutTriage,
        ];
        for variant in &variants {
            serde_round_trip(variant)?;
        }
        Ok(())
    }

    // ── UxCiTier serde round-trip ────────────────────────────────────────

    #[test]
    fn serde_round_trip_ux_ci_tier() -> Result<(), Box<dyn std::error::Error>> {
        let variants = [UxCiTier::Pr, UxCiTier::Nightly, UxCiTier::Release];
        for variant in &variants {
            serde_round_trip(variant)?;
        }
        Ok(())
    }

    // ── UxScenarioResult serde round-trip ────────────────────────────────

    #[test]
    fn serde_round_trip_ux_scenario_result() -> Result<(), Box<dyn std::error::Error>> {
        let variants = [
            UxScenarioResult::Pass,
            UxScenarioResult::Fail,
            UxScenarioResult::Quarantined,
            UxScenarioResult::Skipped,
        ];
        for variant in &variants {
            serde_round_trip(variant)?;
        }
        Ok(())
    }

    // ── UxComponent serde round-trip ─────────────────────────────────────

    #[test]
    fn serde_round_trip_ux_component() -> Result<(), Box<dyn std::error::Error>> {
        let variants = [
            UxComponent::Completion,
            UxComponent::Diagnostics,
            UxComponent::ModuleResolution,
            UxComponent::WorkspaceSymbols,
            UxComponent::Rename,
            UxComponent::SafeDelete,
            UxComponent::Hover,
            UxComponent::GotoDefinition,
            UxComponent::SignatureHelp,
            UxComponent::CodeLens,
            UxComponent::FoldingRange,
            UxComponent::SemanticTokens,
            UxComponent::Infra,
            UxComponent::AiCompletion,
        ];
        for variant in &variants {
            serde_round_trip(variant)?;
        }
        Ok(())
    }
}
