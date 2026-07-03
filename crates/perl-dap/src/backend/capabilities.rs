//! Backend capability description and DAP capability negotiation.
//!
//! DAP capabilities advertised to the editor must be the **intersection** of
//! what the feature catalog supports and what the selected backend can actually
//! do (decision D6). A ptkdb peer in `mirror` mode, for example, must not cause
//! `perl-dap` to advertise control commands the peer never offered.

use serde::{Deserialize, Serialize};

/// Who owns stepping/control in an external-peer session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ControlMode {
    /// The external UI (ptkdb) owns control; the IDE mirrors state. Friendliest
    /// first integration and the only fully-exercised mode today.
    #[default]
    Mirror,
    /// Both the IDE and the external UI may send control commands.
    Cooperative,
    /// The IDE owns control; the external tool is mostly a UI/data provider.
    DapControlled,
}

/// What a backend can do, after any negotiation with its engine/peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugBackendCapabilities {
    /// Can set source-line breakpoints.
    pub source_breakpoints: bool,
    /// Supports conditional breakpoints.
    pub conditional_breakpoints: bool,
    /// Supports hit-count conditions.
    pub hit_conditions: bool,
    /// Supports logpoints.
    pub logpoints: bool,
    /// Supports function/subroutine breakpoints.
    pub function_breakpoints: bool,
    /// Supports data/watchpoints.
    pub data_breakpoints: bool,
    /// Supports expression evaluation.
    pub evaluate: bool,
    /// Can list variables.
    pub variables: bool,
    /// Can list scopes.
    pub scopes: bool,
    /// Can produce stack traces.
    pub stack_trace: bool,
    /// Supports resuming execution (DAP `continue`).
    ///
    /// Distinct from [`Self::stepping`]: a peer can be able to resume a stopped
    /// program without supporting single-step, so the two are negotiated
    /// separately (mirror-mode honesty).
    pub continue_execution: bool,
    /// Supports stepping (next/stepIn/stepOut).
    pub stepping: bool,
    /// Supports pause.
    pub pause: bool,
    /// Supports setting variable values.
    pub set_variable: bool,
    /// The control mode this backend/session operates under.
    pub control_mode: ControlMode,
}

impl DebugBackendCapabilities {
    /// Everything on (used by mocks / a fully-capable native engine).
    #[must_use]
    pub fn full() -> Self {
        Self {
            source_breakpoints: true,
            conditional_breakpoints: true,
            hit_conditions: true,
            logpoints: true,
            function_breakpoints: true,
            data_breakpoints: true,
            evaluate: true,
            variables: true,
            scopes: true,
            stack_trace: true,
            continue_execution: true,
            stepping: true,
            pause: true,
            set_variable: true,
            control_mode: ControlMode::DapControlled,
        }
    }

    /// Nothing on. Starting point before a peer negotiates.
    #[must_use]
    pub fn none() -> Self {
        Self {
            source_breakpoints: false,
            conditional_breakpoints: false,
            hit_conditions: false,
            logpoints: false,
            function_breakpoints: false,
            data_breakpoints: false,
            evaluate: false,
            variables: false,
            scopes: false,
            stack_trace: false,
            continue_execution: false,
            stepping: false,
            pause: false,
            set_variable: false,
            control_mode: ControlMode::Mirror,
        }
    }

    /// The realistic capability floor for a `Devel::ptkdb` peer at protocol v1.
    ///
    /// ptkdb documents conditional breakpoints, sub breakpoints, expression
    /// evaluation, stack and variable inspection — but not DAP-style logpoints,
    /// hit conditions, or data breakpoints. This is a *documentation* default;
    /// the live value is whatever the peer negotiates in its `peer/hello`.
    #[must_use]
    pub fn ptkdb_v1_defaults() -> Self {
        Self {
            source_breakpoints: true,
            conditional_breakpoints: true,
            hit_conditions: false,
            logpoints: false,
            function_breakpoints: true,
            data_breakpoints: false,
            evaluate: true,
            variables: true,
            scopes: true,
            stack_trace: true,
            continue_execution: true,
            stepping: true,
            pause: true,
            set_variable: false,
            control_mode: ControlMode::Mirror,
        }
    }
}

/// The catalog-side (feature.toml-derived) view of which DAP features are
/// compiled/advertised. Passed in explicitly so negotiation is unit-testable
/// without the build-time catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogDapFlags {
    /// `dap.core` — core requests (evaluate-for-hovers, function bps, set var).
    pub core: bool,
    /// `dap.breakpoints.basic` — conditional breakpoints, breakpoint locations.
    pub breakpoints_basic: bool,
    /// `dap.breakpoints.hit_condition`.
    pub hit_condition: bool,
    /// `dap.breakpoints.logpoints`.
    pub logpoints: bool,
    /// `dap.watchpoints` — data breakpoints.
    pub watchpoints: bool,
    /// `dap.breakpoints.function`.
    pub function_breakpoints: bool,
}

impl CatalogDapFlags {
    /// Resolve the catalog flags from the build-time feature catalog.
    #[must_use]
    pub fn from_catalog() -> Self {
        use crate::feature_catalog::has_feature;
        Self {
            core: has_feature("dap.core"),
            breakpoints_basic: has_feature("dap.breakpoints.basic"),
            hit_condition: has_feature("dap.breakpoints.hit_condition"),
            logpoints: has_feature("dap.breakpoints.logpoints"),
            watchpoints: has_feature("dap.watchpoints"),
            function_breakpoints: has_feature("dap.breakpoints.function"),
        }
    }
}

/// The negotiated DAP capability flags: catalog ∩ backend.
///
/// Field names mirror the DAP `capabilities` payload keys the frontend emits in
/// `process.rs` so the mapping is obvious.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NegotiatedDapCapabilities {
    /// `supportsConditionalBreakpoints`.
    pub supports_conditional_breakpoints: bool,
    /// `supportsHitConditionalBreakpoints`.
    pub supports_hit_conditional_breakpoints: bool,
    /// `supportsLogPoints`.
    pub supports_log_points: bool,
    /// `supportsFunctionBreakpoints`.
    pub supports_function_breakpoints: bool,
    /// `supportsDataBreakpoints`.
    pub supports_data_breakpoints: bool,
    /// `supportsEvaluateForHovers`.
    pub supports_evaluate_for_hovers: bool,
    /// `supportsSetVariable`.
    pub supports_set_variable: bool,
}

/// Intersect catalog-advertised DAP features with a backend's capabilities.
///
/// A feature is advertised only if *both* the catalog compiled it in *and* the
/// selected backend can actually do it. This is the honest-negotiation core of
/// decision D6.
#[must_use]
pub fn intersect_dap_capabilities(
    catalog: &CatalogDapFlags,
    backend: &DebugBackendCapabilities,
) -> NegotiatedDapCapabilities {
    NegotiatedDapCapabilities {
        supports_conditional_breakpoints: catalog.breakpoints_basic
            && backend.conditional_breakpoints,
        supports_hit_conditional_breakpoints: catalog.hit_condition && backend.hit_conditions,
        supports_log_points: catalog.logpoints && backend.logpoints,
        supports_function_breakpoints: catalog.function_breakpoints && backend.function_breakpoints,
        supports_data_breakpoints: catalog.watchpoints && backend.data_breakpoints,
        supports_evaluate_for_hovers: catalog.core && backend.evaluate,
        supports_set_variable: catalog.core && backend.set_variable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_catalog() -> CatalogDapFlags {
        CatalogDapFlags {
            core: true,
            breakpoints_basic: true,
            hit_condition: true,
            logpoints: true,
            watchpoints: true,
            function_breakpoints: true,
        }
    }

    #[test]
    fn full_backend_with_full_catalog_advertises_everything() {
        let n = intersect_dap_capabilities(&all_catalog(), &DebugBackendCapabilities::full());
        assert!(n.supports_conditional_breakpoints);
        assert!(n.supports_hit_conditional_breakpoints);
        assert!(n.supports_log_points);
        assert!(n.supports_function_breakpoints);
        assert!(n.supports_data_breakpoints);
        assert!(n.supports_evaluate_for_hovers);
        assert!(n.supports_set_variable);
    }

    #[test]
    fn ptkdb_mirror_does_not_advertise_unsupported_features() {
        // Catalog compiles everything, but a ptkdb v1 peer cannot do logpoints,
        // hit conditions, data breakpoints, or set-variable. Negotiation must
        // NOT advertise them (the mirror-mode honesty guarantee).
        let n = intersect_dap_capabilities(
            &all_catalog(),
            &DebugBackendCapabilities::ptkdb_v1_defaults(),
        );
        assert!(n.supports_conditional_breakpoints, "ptkdb supports conditions");
        assert!(n.supports_function_breakpoints, "ptkdb supports sub breakpoints");
        assert!(n.supports_evaluate_for_hovers, "ptkdb supports evaluate");
        assert!(!n.supports_log_points, "ptkdb v1 has no logpoints");
        assert!(!n.supports_hit_conditional_breakpoints, "ptkdb v1 has no hit conditions");
        assert!(!n.supports_data_breakpoints, "ptkdb v1 has no data breakpoints");
        assert!(!n.supports_set_variable, "ptkdb v1 does not set variables");
    }

    #[test]
    fn catalog_gate_wins_when_feature_not_compiled() {
        // Backend can evaluate, but catalog didn't compile dap.core.
        let catalog = CatalogDapFlags { core: false, ..all_catalog() };
        let n = intersect_dap_capabilities(&catalog, &DebugBackendCapabilities::full());
        assert!(!n.supports_evaluate_for_hovers);
        assert!(!n.supports_set_variable);
        // A non-core feature is unaffected.
        assert!(n.supports_conditional_breakpoints);
    }

    #[test]
    fn control_mode_defaults_to_mirror() {
        assert_eq!(ControlMode::default(), ControlMode::Mirror);
        assert_eq!(DebugBackendCapabilities::none().control_mode, ControlMode::Mirror);
    }
}
