use crate::runtime::routing::{IndexAccessMode, route_index_access};
use perl_parser::workspace_index::IndexCoordinator;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

const INDEX_READY_WAIT_MS: u64 = 2_000;
const INDEX_READY_POLL_MS: u64 = 1;

/// LSP-level milestones used to measure when startup indexing becomes useful.
#[allow(dead_code)] // Provider readiness hooks land in the follow-up workload slice.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ReadinessMilestone {
    /// Workspace indexing thread began.
    WorkspaceStart,
    /// The active document is ready for provider requests.
    ActiveDocumentReady,
    /// Direct imports of the active document are ready.
    DirectDependencySetReady,
    /// The full workspace index is ready.
    WholeWorkspaceReady,
}

impl ReadinessMilestone {
    fn field_name(self) -> &'static str {
        match self {
            Self::WorkspaceStart => "workspace_start_us",
            Self::ActiveDocumentReady => "active_document_ready_us",
            Self::DirectDependencySetReady => "direct_dependency_set_ready_us",
            Self::WholeWorkspaceReady => "whole_workspace_ready_us",
        }
    }
}

/// Provider classes whose first correct answer is useful during startup.
#[allow(dead_code)] // Provider readiness hooks land in the follow-up workload slice.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ReadinessAnswerKind {
    /// Completion became correct for the active document.
    Completion,
    /// Hover became correct for the active document.
    Hover,
    /// Definition became correct for the active document.
    Definition,
    /// References became correct for the active document.
    References,
    /// Diagnostics became correct for the active document.
    Diagnostics,
}

impl ReadinessAnswerKind {
    fn field_name(self) -> &'static str {
        match self {
            Self::Completion => "completion",
            Self::Hover => "hover",
            Self::Definition => "definition",
            Self::References => "references",
            Self::Diagnostics => "diagnostics",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FirstCorrectAnswerReceipt {
    elapsed_us: u64,
    expected_result_class: String,
    answering_tier: String,
    freshness: String,
    fallback_reason: Option<String>,
}

/// LSP-owned receipt for startup usefulness, independent of index lifecycle state.
///
/// Timestamps are recorded relative to `workspace_start`.  The receipt accepts
/// explicit instants so deterministic synthetic workspaces can test transitions
/// without sleeps.  Provider hooks can add first-correct-answer evidence as the
/// readiness workload is connected in a later slice.
#[derive(Debug, Default)]
pub(crate) struct WorkspaceReadinessReceipt {
    workspace_start: Option<Instant>,
    milestones: BTreeMap<&'static str, u64>,
    first_correct_answers: BTreeMap<&'static str, FirstCorrectAnswerReceipt>,
    peak_queued_work: usize,
    memory_high_water_bytes: Option<u64>,
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    test_observer_id: Option<u64>,
}

impl WorkspaceReadinessReceipt {
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub(crate) fn set_test_observer_id(&mut self, observer_id: u64) {
        self.test_observer_id = Some(observer_id);
    }

    /// Record the start of a workspace indexing run once.
    pub(crate) fn record_workspace_start(&mut self, at: Instant) {
        if self.workspace_start.is_none() {
            self.workspace_start = Some(at);
            self.milestones.insert(ReadinessMilestone::WorkspaceStart.field_name(), 0);
        }
    }

    /// Record the first observation of a readiness milestone.
    pub(crate) fn record_milestone(&mut self, milestone: ReadinessMilestone, at: Instant) {
        let Some(start) = self.workspace_start else {
            return;
        };
        self.milestones
            .entry(milestone.field_name())
            .or_insert_with(|| duration_us(at.saturating_duration_since(start)));
    }

    /// Record the first correct provider answer for a workload row.
    #[allow(dead_code)] // Provider readiness hooks land in the follow-up workload slice.
    pub(crate) fn record_first_correct_answer(
        &mut self,
        kind: ReadinessAnswerKind,
        at: Instant,
        expected_result_class: &str,
        answering_tier: &str,
        freshness: &str,
        fallback_reason: Option<&str>,
    ) {
        let Some(start) = self.workspace_start else {
            return;
        };
        self.first_correct_answers.entry(kind.field_name()).or_insert_with(|| {
            FirstCorrectAnswerReceipt {
                elapsed_us: duration_us(at.saturating_duration_since(start)),
                expected_result_class: expected_result_class.to_string(),
                answering_tier: answering_tier.to_string(),
                freshness: freshness.to_string(),
                fallback_reason: fallback_reason.map(str::to_string),
            }
        });
    }

    /// Record the largest observed amount of queued startup work.
    pub(crate) fn record_peak_queued_work(&mut self, queued_work: usize) {
        self.peak_queued_work = self.peak_queued_work.max(queued_work);
    }

    /// Record a memory high-water mark when the host can provide one.
    #[allow(dead_code)] // Host memory sampling lands with the readiness runner.
    pub(crate) fn record_memory_high_water(&mut self, bytes: u64) {
        self.memory_high_water_bytes = Some(match self.memory_high_water_bytes {
            Some(previous) => previous.max(bytes),
            None => bytes,
        });
    }

    /// Return the path-free structured readiness receipt.
    pub(crate) fn summary_json(&self) -> Value {
        let milestone = |name: &'static str| self.milestones.get(name).copied();
        let answers = self
            .first_correct_answers
            .iter()
            .map(|(kind, receipt)| {
                (
                    (*kind).to_string(),
                    json!({
                        "elapsed_us": receipt.elapsed_us,
                        "expected_result_class": receipt.expected_result_class,
                        "answering_tier": receipt.answering_tier,
                        "freshness": receipt.freshness,
                        "fallback_reason": receipt.fallback_reason,
                    }),
                )
            })
            .collect::<serde_json::Map<_, _>>();

        json!({
            "workspace_start_us": milestone("workspace_start_us"),
            "active_document_ready_us": milestone("active_document_ready_us"),
            "direct_dependency_set_ready_us": milestone("direct_dependency_set_ready_us"),
            "whole_workspace_ready_us": milestone("whole_workspace_ready_us"),
            "first_correct_answers": answers,
            "peak_queued_work": self.peak_queued_work,
            "memory_high_water_bytes": self.memory_high_water_bytes,
        })
    }

    /// Emit the current readiness receipt without exposing host paths.
    pub(crate) fn log(&self) {
        let receipt = self.summary_json();
        tracing::info!(
            target: "perl_lsp::workspace_readiness",
            receipt = %receipt,
            "Workspace readiness receipt"
        );
        #[cfg(any(test, feature = "expose_lsp_test_api"))]
        notify_workspace_readiness_receipt(receipt, self.test_observer_id);
        #[cfg(not(any(test, feature = "expose_lsp_test_api")))]
        notify_workspace_readiness_receipt(receipt, None);
    }
}

fn duration_us(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

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
static WORKSPACE_READINESS_RECEIPT_OBSERVERS: std::sync::Mutex<
    Vec<(u64, std::sync::mpsc::Sender<Value>)>,
> = std::sync::Mutex::new(Vec::new());

#[cfg(any(test, feature = "expose_lsp_test_api"))]
static NEXT_WORKSPACE_READINESS_RECEIPT_OBSERVER_ID: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

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

#[cfg(any(test, feature = "expose_lsp_test_api"))]
/// Removes a test-only readiness receipt observer when dropped.
pub(crate) struct WorkspaceReadinessReceiptObserverGuard {
    id: u64,
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
impl WorkspaceReadinessReceiptObserverGuard {
    pub(crate) fn id(&self) -> u64 {
        self.id
    }
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
impl Drop for WorkspaceReadinessReceiptObserverGuard {
    fn drop(&mut self) {
        if let Ok(mut observers) = WORKSPACE_READINESS_RECEIPT_OBSERVERS.lock() {
            observers.retain(|(id, _)| *id != self.id);
        }
    }
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
pub(crate) fn set_workspace_readiness_receipt_observer(
    sender: std::sync::mpsc::Sender<Value>,
) -> WorkspaceReadinessReceiptObserverGuard {
    let id = NEXT_WORKSPACE_READINESS_RECEIPT_OBSERVER_ID
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut observers) = WORKSPACE_READINESS_RECEIPT_OBSERVERS.lock() {
        observers.push((id, sender));
    }
    WorkspaceReadinessReceiptObserverGuard { id }
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
fn notify_workspace_readiness_receipt(receipt: Value, observer_id: Option<u64>) {
    let senders = WORKSPACE_READINESS_RECEIPT_OBSERVERS
        .lock()
        .ok()
        .map(|observers| {
            observers
                .iter()
                .filter(|(id, _)| Some(*id) == observer_id)
                .map(|(_, sender)| sender.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for sender in senders {
        let _ = sender.send(receipt.clone());
    }
}

#[cfg(not(any(test, feature = "expose_lsp_test_api")))]
fn notify_workspace_readiness_receipt(_receipt: Value, _observer_id: Option<u64>) {}

#[cfg(test)]
mod tests {
    use super::{
        IndexReadinessOutcome, IndexReadinessPolicy, ReadinessAnswerKind, ReadinessMilestone,
        WorkspaceReadinessReceipt, check_readiness, check_readiness_with_budget,
        set_index_ready_wait_entered_observer,
    };
    use anyhow::Result;
    use perl_parser::workspace_index::{DegradationReason, IndexCoordinator};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::{Duration, Instant};

    #[test]
    fn readiness_receipt_records_deterministic_first_useful_answer() -> Result<()> {
        let start = Instant::now();
        let mut receipt = WorkspaceReadinessReceipt::default();
        receipt.record_workspace_start(start);
        receipt.record_milestone(
            ReadinessMilestone::ActiveDocumentReady,
            start + Duration::from_micros(10),
        );
        receipt.record_first_correct_answer(
            ReadinessAnswerKind::Completion,
            start + Duration::from_micros(25),
            "non_empty_exact",
            "semantic_source_backed",
            "current_generation",
            Some("partial_index"),
        );
        receipt.record_first_correct_answer(
            ReadinessAnswerKind::Completion,
            start + Duration::from_micros(50),
            "must_not_replace_first",
            "wrong_tier",
            "stale",
            None,
        );
        receipt.record_peak_queued_work(4);
        receipt.record_peak_queued_work(2);
        receipt.record_memory_high_water(128);
        receipt.record_memory_high_water(64);

        let summary = receipt.summary_json();
        assert_eq!(summary["workspace_start_us"], 0);
        assert_eq!(summary["active_document_ready_us"], 10);
        assert_eq!(summary["first_correct_answers"]["completion"]["elapsed_us"], 25);
        assert_eq!(
            summary["first_correct_answers"]["completion"]["expected_result_class"],
            "non_empty_exact"
        );
        assert_eq!(
            summary["first_correct_answers"]["completion"]["fallback_reason"],
            "partial_index"
        );
        assert_eq!(summary["peak_queued_work"], 4);
        assert_eq!(summary["memory_high_water_bytes"], 128);
        Ok(())
    }

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
