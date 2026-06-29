use crate::runtime::routing::{IndexAccessMode, route_index_access};
use perl_parser::workspace_index::IndexCoordinator;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

const INDEX_READY_WAIT_MS: u64 = 2_000;
const INDEX_READY_POLL_MS: u64 = 1;

/// Policy determining how a provider handles index-not-ready states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexReadinessPolicy {
    /// Point queries may briefly wait for a ready index, then fall back honestly.
    WaitBriefly,
    /// Background streams should use their current snapshot and never block.
    SnapshotOnly,
    /// Unsafe edits must refuse stale or partial index state.
    FailClosed,
    /// Local-only providers should not consult workspace index readiness.
    LocalOnly,
}

const ALL_INDEX_READINESS_POLICIES: [IndexReadinessPolicy; 4] = [
    IndexReadinessPolicy::WaitBriefly,
    IndexReadinessPolicy::SnapshotOnly,
    IndexReadinessPolicy::FailClosed,
    IndexReadinessPolicy::LocalOnly,
];

/// Result of applying an index readiness policy.
#[derive(Debug)]
pub(crate) enum IndexReadinessOutcome {
    /// Full workspace index access is available.
    Ready,
    /// A wait happened and produced a non-ready but fallback-safe state.
    Waited(&'static str),
    /// The index is not ready and the provider should use local fallback.
    Partial(&'static str),
    /// The provider deliberately stayed on the current snapshot.
    SnapshotOnly(&'static str),
    /// The provider is local-only and did not consult index readiness.
    LocalOnly(&'static str),
    /// An unsafe operation must fail closed.
    Stale(&'static str),
    /// The bounded wait expired while the index was still building.
    TimedOut(&'static str),
}

impl IndexReadinessOutcome {
    /// Returns true when full workspace index access is available.
    pub(crate) fn is_ready(&self) -> bool {
        matches!(self, IndexReadinessOutcome::Ready)
    }

    /// Returns true when a local or partial fallback is safe.
    pub(crate) fn is_fallback_safe(&self) -> bool {
        matches!(
            self,
            IndexReadinessOutcome::Waited(_)
                | IndexReadinessOutcome::Partial(_)
                | IndexReadinessOutcome::SnapshotOnly(_)
                | IndexReadinessOutcome::LocalOnly(_)
                | IndexReadinessOutcome::TimedOut(_)
        )
    }

    /// Returns true when an unsafe operation should be refused.
    pub(crate) fn is_unsafe_rejected(&self) -> bool {
        matches!(self, IndexReadinessOutcome::Stale(_))
    }

    /// Returns a stable reason suitable for traces and user-visible receipts.
    pub(crate) fn reason(&self) -> &'static str {
        match self {
            IndexReadinessOutcome::Ready => "index ready",
            IndexReadinessOutcome::Waited(reason)
            | IndexReadinessOutcome::Partial(reason)
            | IndexReadinessOutcome::SnapshotOnly(reason)
            | IndexReadinessOutcome::LocalOnly(reason)
            | IndexReadinessOutcome::Stale(reason)
            | IndexReadinessOutcome::TimedOut(reason) => reason,
        }
    }
}

/// Apply the provider-specific index readiness policy.
pub(crate) fn check_readiness(
    coordinator: Option<&Arc<IndexCoordinator>>,
    indexing_in_progress: &AtomicBool,
    policy: IndexReadinessPolicy,
) -> IndexReadinessOutcome {
    debug_assert!(ALL_INDEX_READINESS_POLICIES.contains(&policy));
    check_readiness_with_budget(
        coordinator,
        indexing_in_progress,
        policy,
        Duration::from_millis(INDEX_READY_WAIT_MS),
    )
}

fn check_readiness_with_budget(
    coordinator: Option<&Arc<IndexCoordinator>>,
    indexing_in_progress: &AtomicBool,
    policy: IndexReadinessPolicy,
    wait_budget: Duration,
) -> IndexReadinessOutcome {
    match policy {
        IndexReadinessPolicy::LocalOnly => IndexReadinessOutcome::LocalOnly("local-only provider"),
        IndexReadinessPolicy::SnapshotOnly => {
            IndexReadinessOutcome::SnapshotOnly("snapshot-only provider")
        }
        IndexReadinessPolicy::FailClosed => match route_index_access(coordinator) {
            IndexAccessMode::Full(_) => IndexReadinessOutcome::Ready,
            IndexAccessMode::Partial(reason) => IndexReadinessOutcome::Stale(reason),
            IndexAccessMode::None => IndexReadinessOutcome::Stale("no workspace index"),
        },
        IndexReadinessPolicy::WaitBriefly => {
            check_wait_briefly(coordinator, indexing_in_progress, wait_budget)
        }
    }
}

fn check_wait_briefly(
    coordinator: Option<&Arc<IndexCoordinator>>,
    indexing_in_progress: &AtomicBool,
    wait_budget: Duration,
) -> IndexReadinessOutcome {
    let Some(coord) = coordinator else {
        return IndexReadinessOutcome::Partial("no workspace index");
    };

    if !indexing_in_progress.load(Ordering::Acquire) {
        return access_mode_to_readiness(route_index_access(Some(coord)));
    }

    let deadline = Instant::now() + wait_budget;
    let mut waited = false;

    loop {
        match route_index_access(Some(coord)) {
            IndexAccessMode::Full(_) => {
                tracing::debug!("check_readiness: index is Ready");
                return IndexReadinessOutcome::Ready;
            }
            IndexAccessMode::Partial(reason) => {
                if !reason.starts_with("index building") {
                    tracing::debug!(reason, "check_readiness: index degraded, proceeding");
                    return if waited {
                        IndexReadinessOutcome::Waited(reason)
                    } else {
                        IndexReadinessOutcome::Partial(reason)
                    };
                }

                waited = true;
                notify_index_ready_wait_entered();
                if Instant::now() >= deadline {
                    tracing::debug!(
                        reason,
                        "check_readiness: deadline reached, serving partial index"
                    );
                    return IndexReadinessOutcome::TimedOut(reason);
                }
                std::thread::sleep(Duration::from_millis(INDEX_READY_POLL_MS));
            }
            IndexAccessMode::None => return IndexReadinessOutcome::Partial("no workspace index"),
        }
    }
}

fn access_mode_to_readiness(access_mode: IndexAccessMode<'_>) -> IndexReadinessOutcome {
    match access_mode {
        IndexAccessMode::Full(_) => IndexReadinessOutcome::Ready,
        IndexAccessMode::Partial(reason) => IndexReadinessOutcome::Partial(reason),
        IndexAccessMode::None => IndexReadinessOutcome::Partial("no workspace index"),
    }
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
static INDEX_READY_WAIT_ENTERED_OBSERVER: std::sync::Mutex<Option<std::sync::mpsc::Sender<()>>> =
    std::sync::Mutex::new(None);

#[cfg(any(test, feature = "expose_lsp_test_api"))]
pub(crate) fn set_index_ready_wait_entered_observer(sender: std::sync::mpsc::Sender<()>) {
    if let Ok(mut observer) = INDEX_READY_WAIT_ENTERED_OBSERVER.lock() {
        *observer = Some(sender);
    }
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
fn notify_index_ready_wait_entered() {
    let sender =
        INDEX_READY_WAIT_ENTERED_OBSERVER.lock().ok().and_then(|mut observer| observer.take());
    if let Some(sender) = sender {
        let _ = sender.send(());
    }
}

#[cfg(not(any(test, feature = "expose_lsp_test_api")))]
fn notify_index_ready_wait_entered() {}

#[cfg(test)]
mod tests {
    use super::{
        IndexReadinessOutcome, IndexReadinessPolicy, check_readiness, check_readiness_with_budget,
        set_index_ready_wait_entered_observer,
    };
    use anyhow::Result;
    use perl_parser::workspace_index::{DegradationReason, IndexCoordinator};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::Duration;

    #[test]
    fn readiness_contract_waitbriefly_ready_returns_ready() -> Result<()> {
        let coordinator = Arc::new(IndexCoordinator::new());
        coordinator.transition_to_ready(1, 1);
        let indexing = AtomicBool::new(false);

        let outcome =
            check_readiness(Some(&coordinator), &indexing, IndexReadinessPolicy::WaitBriefly);

        assert!(matches!(outcome, IndexReadinessOutcome::Ready));
        assert!(outcome.is_ready());
        assert_eq!(outcome.reason(), "index ready");
        Ok(())
    }

    #[test]
    fn readiness_contract_waitbriefly_building_times_out() -> Result<()> {
        let coordinator = Arc::new(IndexCoordinator::new());
        let indexing = AtomicBool::new(true);

        let outcome = check_readiness_with_budget(
            Some(&coordinator),
            &indexing,
            IndexReadinessPolicy::WaitBriefly,
            Duration::from_millis(2),
        );

        assert!(matches!(outcome, IndexReadinessOutcome::TimedOut(_)));
        assert!(outcome.is_fallback_safe());
        assert!(outcome.reason().starts_with("index building"));
        Ok(())
    }

    #[test]
    fn readiness_contract_waitbriefly_building_can_become_ready() -> Result<()> {
        let coordinator = Arc::new(IndexCoordinator::new());
        let indexing = Arc::new(AtomicBool::new(true));
        let worker_coordinator = Arc::clone(&coordinator);
        let worker_indexing = Arc::clone(&indexing);

        let worker = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(10));
            worker_coordinator.transition_to_ready(1, 1);
            worker_indexing.store(false, Ordering::Release);
        });

        let outcome = check_readiness_with_budget(
            Some(&coordinator),
            indexing.as_ref(),
            IndexReadinessPolicy::WaitBriefly,
            Duration::from_millis(500),
        );

        worker.join().map_err(|_| anyhow::anyhow!("readiness transition thread panicked"))?;
        assert!(matches!(outcome, IndexReadinessOutcome::Ready));
        Ok(())
    }

    #[test]
    fn readiness_contract_failclosed_rejects_building_index() -> Result<()> {
        let coordinator = Arc::new(IndexCoordinator::new());
        let indexing = AtomicBool::new(true);

        let outcome =
            check_readiness(Some(&coordinator), &indexing, IndexReadinessPolicy::FailClosed);

        assert!(matches!(outcome, IndexReadinessOutcome::Stale(_)));
        assert!(outcome.is_unsafe_rejected());
        Ok(())
    }

    #[test]
    fn readiness_contract_snapshot_only_does_not_wait() -> Result<()> {
        let coordinator = Arc::new(IndexCoordinator::new());
        let indexing = AtomicBool::new(true);

        let outcome =
            check_readiness(Some(&coordinator), &indexing, IndexReadinessPolicy::SnapshotOnly);

        assert!(matches!(outcome, IndexReadinessOutcome::SnapshotOnly(_)));
        assert!(indexing.load(Ordering::Acquire));
        assert!(outcome.is_fallback_safe());
        Ok(())
    }

    #[test]
    fn readiness_contract_local_only_ignores_missing_coordinator() -> Result<()> {
        let indexing = AtomicBool::new(true);

        let outcome = check_readiness(None, &indexing, IndexReadinessPolicy::LocalOnly);

        assert!(matches!(outcome, IndexReadinessOutcome::LocalOnly(_)));
        assert!(outcome.is_fallback_safe());
        Ok(())
    }

    #[test]
    fn readiness_contract_outcome_helpers_cover_all_non_ready_states() -> Result<()> {
        let outcomes = [
            (IndexReadinessOutcome::Ready, false, false, "index ready"),
            (IndexReadinessOutcome::Waited("waited"), true, false, "waited"),
            (IndexReadinessOutcome::Partial("partial"), true, false, "partial"),
            (IndexReadinessOutcome::SnapshotOnly("snapshot-only"), true, false, "snapshot-only"),
            (IndexReadinessOutcome::LocalOnly("local-only"), true, false, "local-only"),
            (IndexReadinessOutcome::Stale("stale"), false, true, "stale"),
            (IndexReadinessOutcome::TimedOut("timed out"), true, false, "timed out"),
        ];

        for (outcome, fallback_safe, unsafe_rejected, reason) in outcomes {
            assert_eq!(outcome.is_fallback_safe(), fallback_safe);
            assert_eq!(outcome.is_unsafe_rejected(), unsafe_rejected);
            assert_eq!(outcome.reason(), reason);
        }
        Ok(())
    }

    #[test]
    fn readiness_contract_failclosed_allows_ready_index() -> Result<()> {
        let coordinator = Arc::new(IndexCoordinator::new());
        coordinator.transition_to_ready(1, 1);
        let indexing = AtomicBool::new(false);

        let outcome =
            check_readiness(Some(&coordinator), &indexing, IndexReadinessPolicy::FailClosed);

        assert!(outcome.is_ready());
        assert_eq!(outcome.reason(), "index ready");
        Ok(())
    }

    #[test]
    fn readiness_contract_failclosed_rejects_missing_index() -> Result<()> {
        let indexing = AtomicBool::new(false);

        let outcome = check_readiness(None, &indexing, IndexReadinessPolicy::FailClosed);

        assert!(matches!(outcome, IndexReadinessOutcome::Stale(_)));
        assert!(outcome.is_unsafe_rejected());
        assert_eq!(outcome.reason(), "no workspace index");
        Ok(())
    }

    #[test]
    fn readiness_contract_waitbriefly_missing_index_is_partial() -> Result<()> {
        let indexing = AtomicBool::new(false);

        let outcome = check_readiness(None, &indexing, IndexReadinessPolicy::WaitBriefly);

        assert!(matches!(outcome, IndexReadinessOutcome::Partial(_)));
        assert!(outcome.is_fallback_safe());
        assert_eq!(outcome.reason(), "no workspace index");
        Ok(())
    }

    #[test]
    fn readiness_contract_waitbriefly_degraded_index_is_partial_without_wait() -> Result<()> {
        let coordinator = Arc::new(IndexCoordinator::new());
        coordinator.transition_to_degraded(DegradationReason::ScanTimeout { elapsed_ms: 123 });
        let indexing = AtomicBool::new(true);

        let outcome = check_readiness_with_budget(
            Some(&coordinator),
            &indexing,
            IndexReadinessPolicy::WaitBriefly,
            Duration::from_millis(10),
        );

        assert!(matches!(outcome, IndexReadinessOutcome::Partial(_)));
        assert!(outcome.is_fallback_safe());
        assert!(outcome.reason().contains("scan timeout"));
        Ok(())
    }

    #[test]
    fn readiness_contract_waitbriefly_degraded_after_building_records_wait() -> Result<()> {
        let coordinator = Arc::new(IndexCoordinator::new());
        let indexing = AtomicBool::new(true);
        let (wait_entered_tx, wait_entered_rx) = std::sync::mpsc::channel();
        set_index_ready_wait_entered_observer(wait_entered_tx);
        let worker_coordinator = Arc::clone(&coordinator);

        let worker = std::thread::spawn(move || -> Result<()> {
            wait_entered_rx.recv_timeout(Duration::from_secs(1))?;
            worker_coordinator
                .transition_to_degraded(DegradationReason::ScanTimeout { elapsed_ms: 456 });
            Ok(())
        });

        let outcome = check_readiness_with_budget(
            Some(&coordinator),
            &indexing,
            IndexReadinessPolicy::WaitBriefly,
            Duration::from_secs(1),
        );

        worker.join().map_err(|_| anyhow::anyhow!("readiness observer thread panicked"))??;
        assert!(matches!(outcome, IndexReadinessOutcome::Waited(_)));
        assert!(outcome.is_fallback_safe());
        assert!(outcome.reason().contains("scan timeout"));
        Ok(())
    }
}
