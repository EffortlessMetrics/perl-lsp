//! Index-aware routing for LSP handlers
//!
//! This module provides a unified routing policy for LSP handlers based on
//! the `IndexCoordinator` state. It eliminates ad-hoc state checks scattered
//! across handlers and provides predictable behavior during index building
//! and degraded states.
//!
//! # Design Principles
//!
//! 1. **Single Point of Policy**: All index-related routing decisions flow through
//!    `IndexAccessMode` and the `route_index_access()` helper.
//!
//! 2. **Predictable Degradation**: Handlers return partial results (same-file only)
//!    during Building/Degraded states rather than blocking or returning empty.
//!
//! 3. **Never Panic**: Degraded mode always returns safe fallback behavior.
//!
//! # Index Access Modes
//!
//! - **Full**: Index is Ready, all workspace-wide operations available
//! - **Partial**: Index is Building/Degraded, only same-file or open-doc operations
//! - **None**: No workspace feature, local-only operations
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::runtime::routing::{IndexAccessMode, route_index_access};
//!
//! // In a handler:
//! let mode = route_index_access(self.coordinator());
//! match mode {
//!     IndexAccessMode::Full(coordinator) => {
//!         // Full workspace search
//!     }
//!     IndexAccessMode::Partial(reason) => {
//!         // Same-file or open-doc fallback
//!         eprintln!("Definition: {}, using same-file fallback", reason);
//!     }
//!     IndexAccessMode::None => {
//!         // Local-only operation
//!     }
//! }
//! ```
// This module is part of ongoing refactoring - allow unused code during development
#![allow(dead_code)]
#![allow(unreachable_pub)]

#[cfg(feature = "workspace")]
use perl_parser::workspace_index::{DegradationReason, IndexCoordinator, IndexState};
#[cfg(feature = "workspace")]
use std::sync::Arc;

/// The resolved access mode for index-dependent operations
///
/// This enum represents the outcome of routing policy evaluation:
/// - `Full`: Index is ready for workspace-wide queries
/// - `Partial`: Index is not ready, handlers should use same-file/open-doc fallback
/// - `None`: No workspace feature enabled
#[cfg(feature = "workspace")]
#[derive(Debug)]
pub enum IndexAccessMode<'a> {
    /// Full workspace access available
    Full(&'a Arc<IndexCoordinator>),

    /// Partial access - use same-file or open-doc fallback
    /// Contains a human-readable reason for logging
    Partial(&'static str),

    /// No workspace feature available
    None,
}

/// The resolved access mode for index-dependent operations (no workspace)
#[cfg(not(feature = "workspace"))]
#[derive(Debug)]
pub enum IndexAccessMode {
    /// Partial access - use same-file or open-doc fallback
    Partial(&'static str),

    /// No workspace feature available
    None,
}

#[cfg(feature = "workspace")]
impl IndexAccessMode<'_> {
    /// Returns true if full workspace access is available
    #[inline]
    pub fn is_full(&self) -> bool {
        matches!(self, IndexAccessMode::Full(_))
    }

    /// Returns true if only partial/same-file access is available
    #[inline]
    pub fn is_partial(&self) -> bool {
        matches!(self, IndexAccessMode::Partial(_))
    }

    /// Returns a description suitable for logging
    pub fn description(&self) -> &'static str {
        match self {
            IndexAccessMode::Full(_) => "full workspace access",
            IndexAccessMode::Partial(reason) => reason,
            IndexAccessMode::None => "no workspace feature",
        }
    }
}

#[cfg(not(feature = "workspace"))]
impl IndexAccessMode {
    /// Returns true if full workspace access is available
    #[inline]
    pub fn is_full(&self) -> bool {
        false
    }

    /// Returns true if only partial/same-file access is available
    #[inline]
    pub fn is_partial(&self) -> bool {
        matches!(self, IndexAccessMode::Partial(_))
    }

    /// Returns a description suitable for logging
    pub fn description(&self) -> &'static str {
        match self {
            IndexAccessMode::Partial(reason) => reason,
            IndexAccessMode::None => "no workspace feature",
        }
    }
}

/// Route to appropriate index access mode based on coordinator state
///
/// This is the single policy function that all handlers should use.
/// It evaluates the current index state and returns the appropriate
/// access mode for the handler to use.
///
/// # Arguments
///
/// * `coordinator` - Optional reference to the IndexCoordinator
///
/// # Returns
///
/// `IndexAccessMode` indicating what level of index access is available
///
/// # Policy Rules
///
/// - `Ready` state → `Full` access
/// - `Building` state → `Partial` with "index building" reason
/// - `Degraded` state → `Partial` with degradation reason
/// - No coordinator → `None` (no workspace feature)
#[cfg(feature = "workspace")]
pub fn route_index_access(coordinator: Option<&Arc<IndexCoordinator>>) -> IndexAccessMode<'_> {
    match coordinator {
        Some(coord) => {
            match coord.state() {
                IndexState::Ready { .. } => IndexAccessMode::Full(coord),
                IndexState::Building { indexed_count, total_count, .. } => {
                    // Provide specific reason for building state
                    if total_count == 0 {
                        IndexAccessMode::Partial("index building (scanning workspace)")
                    } else if indexed_count < total_count {
                        IndexAccessMode::Partial("index building (indexing files)")
                    } else {
                        IndexAccessMode::Partial("index building")
                    }
                }
                IndexState::Degraded { reason, .. } => {
                    // Map degradation reason to human-readable message
                    IndexAccessMode::Partial(degradation_reason_str(&reason))
                }
            }
        }
        None => IndexAccessMode::None,
    }
}

/// Route to index access mode (non-workspace version)
#[cfg(not(feature = "workspace"))]
pub(crate) fn route_index_access<T>(_coordinator: Option<&T>) -> IndexAccessMode {
    IndexAccessMode::None
}

/// Convert degradation reason to static string for logging
#[cfg(feature = "workspace")]
fn degradation_reason_str(reason: &DegradationReason) -> &'static str {
    match reason {
        DegradationReason::ParseStorm { .. } => "index degraded (parse storm)",
        DegradationReason::IoError { .. } => "index degraded (IO error)",
        DegradationReason::ScanTimeout { .. } => "index degraded (scan timeout)",
        DegradationReason::ResourceLimit { .. } => "index degraded (resource limit)",
    }
}

/// Helper trait for handlers to implement degraded behavior
///
/// Handlers can implement this trait to provide consistent degraded
/// behavior across the codebase.
#[allow(dead_code)]
pub(crate) trait DegradedBehavior<T> {
    /// The default empty result for this handler
    fn empty_result() -> T;

    /// Description of what's unavailable in degraded mode
    fn unavailable_feature() -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_mode_none_without_workspace() {
        let mode: IndexAccessMode<'static> = IndexAccessMode::None;
        assert!(!mode.is_full());
        assert!(!mode.is_partial());
        assert_eq!(mode.description(), "no workspace feature");
    }

    #[test]
    fn test_access_mode_partial() {
        let mode = IndexAccessMode::Partial("test reason");
        assert!(!mode.is_full());
        assert!(mode.is_partial());
        assert_eq!(mode.description(), "test reason");
    }

    #[cfg(feature = "workspace")]
    mod workspace_tests {
        use super::*;

        #[test]
        fn test_route_building_state() {
            let coordinator = Arc::new(IndexCoordinator::new());
            // Default state is Building
            let mode = route_index_access(Some(&coordinator));
            assert!(mode.is_partial());
            assert!(mode.description().contains("building"));
        }

        #[test]
        fn test_route_ready_state() {
            let coordinator = Arc::new(IndexCoordinator::new());
            coordinator.transition_to_ready(10, 100);

            let mode = route_index_access(Some(&coordinator));
            assert!(mode.is_full());
            assert_eq!(mode.description(), "full workspace access");
        }

        #[test]
        fn test_route_degraded_parse_storm() {
            let coordinator = Arc::new(IndexCoordinator::new());
            coordinator.transition_to_ready(10, 100);

            // Trigger parse storm
            for _ in 0..15 {
                coordinator.notify_change("test.pm");
            }

            let mode = route_index_access(Some(&coordinator));
            assert!(mode.is_partial());
            assert!(mode.description().contains("parse storm"));
        }

        #[test]
        fn test_route_none_coordinator() {
            let mode = route_index_access(None::<&Arc<IndexCoordinator>>);
            assert!(matches!(mode, IndexAccessMode::None));
        }
    }
}
