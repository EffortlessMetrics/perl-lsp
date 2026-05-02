use std::collections::HashSet;

use crate::tasks::ci_scope::ScopeOutput;

use super::{GateDefinition, GatePlanningRole, GateTier};

#[derive(Debug, Clone)]
pub(super) struct GatePlan {
    pub(super) tier: GateTier,
    pub(super) base: String,
    pub(super) scope: Option<ScopeOutput>,
    pub(super) scope_ok: bool,
    pub(super) fallback_used: bool,
    pub(super) fallback_reason: Option<String>,
    pub(super) package_args: Vec<String>,
    pub(super) selected: Vec<PlannedGate>,
    pub(super) skipped: Vec<SkippedGate>,
}

#[derive(Debug, Clone)]
pub(super) struct PlannedGate {
    pub(super) gate: GateDefinition,
    pub(super) role: GatePlanningRole,
    pub(super) reason: String,
}

#[derive(Debug, Clone)]
pub(super) struct SkippedGate {
    pub(super) name: String,
    pub(super) role: Option<GatePlanningRole>,
    pub(super) reason: String,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PackageTargetIndex {
    pub(super) lib_packages: HashSet<String>,
}

impl PackageTargetIndex {
    pub(super) fn has_lib(&self, package: &str) -> bool {
        self.lib_packages.contains(package)
    }
}
