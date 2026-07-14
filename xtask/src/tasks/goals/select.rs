//! Pure, deterministic work selector (#3624, M3 of the enablement train
//! #3612).
//!
//! `select_next` takes a [`SelectionSnapshot`] built entirely by adapters
//! (`super::snapshot` — the only place `git`/`gh` are shelled or manifests
//! are read from disk) and returns a [`SelectionDecision`]. It performs
//! **no I/O**: no shell, no filesystem, no network, no session-task-board reads.
//! The same snapshot always produces the same decision (byte-stable JSON).
//!
//! Invariant (mirrors CLAUDE.md's truth hierarchy): live GitHub state and
//! the manifest chain outrank conversation and session bookkeeping. This
//! module and its siblings in `xtask/src/tasks/goals/` must never read or
//! write the Claude Code harness's session task-tracking board — that
//! board's own "completed" flag is known not to persist reliably across
//! sessions and must never gate selection. See `mod.rs`'s test suite for
//! the mechanical check of this invariant.

use serde::Serialize;
use std::collections::BTreeSet;

/// Normalized status of a selectable unit of work (a milestone or a
/// lane-routing work item), independent of which program manifest shape
/// produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilestoneStatus {
    Completed,
    InProgress,
    Pending,
    Blocked,
    Deferred,
}

/// Parses a raw manifest status string into the normalized selection
/// status. Unknown strings fail closed to `Blocked` rather than being
/// silently treated as selectable.
pub fn parse_status(raw: &str) -> MilestoneStatus {
    match raw {
        "completed" => MilestoneStatus::Completed,
        "in_progress" | "active" => MilestoneStatus::InProgress,
        "pending" | "ready" | "planned" => MilestoneStatus::Pending,
        "deferred" => MilestoneStatus::Deferred,
        _ => MilestoneStatus::Blocked,
    }
}

#[derive(Debug, Clone)]
pub struct MilestoneCandidate {
    pub id: String,
    pub title: String,
    pub status: MilestoneStatus,
    pub issue: Option<u64>,
    pub depends_on: Vec<String>,
    pub exit_criteria: String,
    pub lane: Option<String>,
    pub claim_boundary: Option<String>,
    pub ownership: Vec<String>,
    pub required_proof: Vec<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct LiveOpenPr {
    pub number: u64,
    pub title: String,
    #[serde(default)]
    pub body: String,
    pub url: String,
    #[serde(rename = "isDraft", alias = "is_draft", default)]
    pub is_draft: bool,
}

#[derive(Debug, Clone)]
pub struct ProgramCandidate {
    pub id: String,
}

/// Everything `select_next` needs, already resolved and read. Built only
/// by `super::snapshot::build_snapshot` (live) or directly by tests
/// (fixture-equivalent, in-process).
#[derive(Debug, Clone)]
pub struct SelectionSnapshot {
    pub repository: String,
    pub requested_program: Option<String>,
    pub default_program: Option<String>,
    pub known_programs: Vec<ProgramCandidate>,
    /// `None` means program selection could not be resolved unambiguously
    /// (no explicit `--program`, no governed `default_program`, or more
    /// than one candidate default with no priority winner). `select_next`
    /// fails closed on this rather than guessing.
    pub resolved_program: Option<String>,
    pub mode: String,
    pub board: Option<String>,
    /// Program-level display title (e.g. a milestone ledger's own `title`
    /// field), surfaced in `GoalsNextOutput` for human/JSON consumers.
    /// `None` for lane-routing programs, which have no equivalent field.
    pub program_title: Option<String>,
    /// The GitHub tracker issue for the whole program (e.g. #3612 for
    /// `agent_loop_enablement`), distinct from any individual candidate's
    /// own `issue`.
    pub tracker_issue: Option<u64>,
    pub non_goals: Vec<String>,
    pub candidates: Vec<MilestoneCandidate>,
    pub live_open_prs: Vec<LiveOpenPr>,
    /// The actual local git ref (branch name, or short SHA when detached)
    /// this snapshot's evidence was read from — measured by
    /// `super::snapshot::current_git_ref`, never hardcoded. Surfaced in
    /// `WorkPacket::inputs_used` so the JSON receipt does not misattribute
    /// its own evidence when the checkout is not `main` (see #3692).
    pub current_git_ref: String,
    /// `false` when the adapter could not obtain live PR state at all (e.g.
    /// `gh pr list` failed/unauthenticated) — distinct from "queried and
    /// found zero PRs". Checked FIRST by `classify_in_progress` so an
    /// in-progress candidate with genuinely no open PR is never confused
    /// with one where liveness simply couldn't be determined. `--fixture`
    /// callers and the live adapter both always set this explicitly (never
    /// left to a type default), per #3696 item B.
    pub live_prs_available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkPacket {
    pub issue: Option<u64>,
    pub id: String,
    pub reason: String,
    pub mode: String,
    pub ownership: Vec<String>,
    pub non_goals: Vec<String>,
    pub required_proof: Vec<String>,
    pub session_goal: String,
    pub inputs_used: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SelectionBlocker {
    pub kind: String,
    pub detail: String,
    pub pr_number: Option<u64>,
    pub pr_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompletionEvidence {
    pub program: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "decision", content = "data", rename_all = "snake_case")]
pub enum SelectionDecision {
    Selected(WorkPacket),
    Blocked(Vec<SelectionBlocker>),
    Complete(CompletionEvidence),
}

/// A single advisory finding from `goals reconcile` (#3696 item B):
/// evidence that a milestone's self-reported ledger `status` may have
/// drifted from live GitHub reality, or lacks the identity `select_next`
/// needs to verify it. Unlike [`SelectionBlocker`], these never gate
/// `select_next` on their own — `reconcile_in_progress` is a diagnostic
/// report, not a selection decision.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReconciliationFinding {
    pub milestone_id: String,
    pub issue: Option<u64>,
    pub kind: String,
    pub detail: String,
    pub pr_number: Option<u64>,
    pub pr_url: Option<String>,
}

/// Result of classifying a single `InProgress` candidate against live PR
/// state, for Guard A (#3696 item B). `Reconciled` is the only outcome that
/// does NOT itself produce a selection blocker — it falls through to the
/// existing rule below, which already blocks on a healthy single-flight
/// open PR.
#[derive(Debug, Clone, PartialEq, Eq)]
enum InProgressReconciliation {
    /// Exactly one open PR references this candidate's issue.
    Reconciled,
    /// `status == InProgress` but `issue.is_none()` — unmatchable against
    /// any live PR state.
    LacksIdentity,
    /// Has an issue, but zero open PRs reference it (e.g. its PR merged,
    /// not open — the M3/#3647-ledger case this PR fixes). Carries the
    /// issue number so callers never need to re-derive it from
    /// `candidate.issue` with a fallback (it is always `Some` here by
    /// construction — see `classify_in_progress`).
    NoLivePr(u64),
    /// More than one open PR references the same issue — decision 4:
    /// reclassified from `active_work_must_be_dispositioned` (ambiguity is
    /// a reconciliation problem, not a plain single-flight conflict).
    /// Carries the issue number for the same reason as `NoLivePr`.
    MultiplePrs(u64, Vec<u64>),
    /// The adapter could not obtain live PR state at all
    /// (`live_prs_available == false`). Checked FIRST so "unknown" is
    /// never conflated with "confirmed no open PR".
    LiveStateUnavailable,
}

fn classify_in_progress(
    candidate: &MilestoneCandidate,
    open_prs: &[LiveOpenPr],
    repository: &str,
    live_prs_available: bool,
) -> InProgressReconciliation {
    if !live_prs_available {
        return InProgressReconciliation::LiveStateUnavailable;
    }
    let Some(issue) = candidate.issue else {
        return InProgressReconciliation::LacksIdentity;
    };
    let matching: Vec<u64> = open_prs
        .iter()
        .filter(|pr| references_issue(pr, repository, issue))
        .map(|pr| pr.number)
        .collect();
    match matching.len() {
        0 => InProgressReconciliation::NoLivePr(issue),
        1 => InProgressReconciliation::Reconciled,
        _ => InProgressReconciliation::MultiplePrs(issue, matching),
    }
}

/// Pure selection. See module docs for the I/O contract.
///
/// Precedence:
/// 1. Ambiguous program authority (`resolved_program` unset) -> `Blocked`.
/// 2. Guard A (#3696 item B): every `InProgress` MILESTONE candidate
///    (`lane.is_none()` — the same milestone-vs-lane-routing discriminator
///    `load_milestone_candidates`/`load_lane_routing_candidates` already
///    establish: milestone candidates always have `lane: None`, lane work
///    items always carry `lane: Some(_)`) must reconcile against live PR
///    state (exactly one matching open PR) -> otherwise `Blocked` with
///    `in_progress_state_requires_reconciliation`, naming the unreconciled
///    candidate (declaration order). This runs BEFORE rule 3 below so a
///    stale/identity-less in-progress milestone can never be silently
///    skipped in favor of a later Pending sibling. Scoped to milestones
///    ONLY (decision 3, #3696 item B): a lane work item is legitimately
///    identity-less/in-progress by design (the #3634 trust lane), so it
///    must never trip this guard.
/// 3. A live open PR already references any non-terminal (not
///    `Completed`/`Deferred`) candidate in this program -> `Blocked`,
///    naming the PR (never mutated/closed here). Live PR state outranks
///    the candidate's self-reported ledger status.
/// 4. Earliest candidate (declaration order) that is itself `Pending` and
///    whose `depends_on` are all `Completed` -> `Selected`, unless
///    `live_prs_available` is `false` (`live_pr_state_unavailable`) or it
///    has no `issue` (`pending_candidate_missing_issue`) -> `Blocked`,
///    naming it (never skipping to a later sibling that happens to
///    qualify).
/// 5. Every candidate `Completed`/`Deferred` -> `Complete`.
/// 6. Otherwise (e.g. everything in flight/blocked with unmet deps, or no
///    candidates at all) -> `Blocked` rather than guessing.
pub fn select_next(snapshot: &SelectionSnapshot) -> SelectionDecision {
    let Some(program_id) = snapshot.resolved_program.as_deref() else {
        return SelectionDecision::Blocked(vec![SelectionBlocker {
            kind: "ambiguous_program_authority".to_owned(),
            detail: ambiguity_detail(snapshot),
            pr_number: None,
            pr_url: None,
        }]);
    };

    let mut guard_a_blockers = Vec::new();
    for candidate in snapshot
        .candidates
        .iter()
        .filter(|c| c.status == MilestoneStatus::InProgress && c.lane.is_none())
    {
        match classify_in_progress(
            candidate,
            &snapshot.live_open_prs,
            &snapshot.repository,
            snapshot.live_prs_available,
        ) {
            InProgressReconciliation::Reconciled => {}
            InProgressReconciliation::LiveStateUnavailable => {
                guard_a_blockers.push(SelectionBlocker {
                    kind: "in_progress_state_requires_reconciliation".to_owned(),
                    detail: format!(
                        "{}: in_progress but live PR state is unavailable; cannot verify reconciliation (retry with gh access, or pass --fixture)",
                        candidate.id
                    ),
                    pr_number: None,
                    pr_url: None,
                });
            }
            InProgressReconciliation::LacksIdentity => {
                guard_a_blockers.push(SelectionBlocker {
                    kind: "in_progress_state_requires_reconciliation".to_owned(),
                    detail: format!(
                        "{}: in_progress with no issue number; cannot verify live PR state — run `goals reconcile`",
                        candidate.id
                    ),
                    pr_number: None,
                    pr_url: None,
                });
            }
            InProgressReconciliation::NoLivePr(issue) => {
                guard_a_blockers.push(SelectionBlocker {
                    kind: "in_progress_state_requires_reconciliation".to_owned(),
                    detail: format!(
                        "{}: in_progress (#{issue}) but no open PR references it (its PR may have merged); mark completed with merged evidence, reopen, or split — run `goals reconcile`",
                        candidate.id
                    ),
                    pr_number: None,
                    pr_url: None,
                });
            }
            InProgressReconciliation::MultiplePrs(issue, pr_numbers) => {
                for pr_number in pr_numbers {
                    let pr_url = snapshot
                        .live_open_prs
                        .iter()
                        .find(|pr| pr.number == pr_number)
                        .map(|pr| pr.url.clone());
                    guard_a_blockers.push(SelectionBlocker {
                        kind: "in_progress_state_requires_reconciliation".to_owned(),
                        detail: format!(
                            "{}: in_progress (#{issue}) has more than one open PR referencing it (#{pr_number}); disposition the ambiguity before selecting new work",
                            candidate.id
                        ),
                        pr_number: Some(pr_number),
                        pr_url,
                    });
                }
            }
        }
    }
    if !guard_a_blockers.is_empty() {
        return SelectionDecision::Blocked(guard_a_blockers);
    }

    // Match live open PRs against every non-terminal candidate's issue
    // (in-progress, pending, AND blocked), not only in-progress ones.
    // A candidate's ledger `status` is self-reported and can drift from
    // reality (e.g. a PR opened for a "pending" candidate before its
    // status was updated); per CLAUDE.md's truth hierarchy live GitHub PR
    // state outranks the manifest, so scoping this guard to `InProgress`
    // alone would let `select_next` hand out a sibling candidate while
    // real work is already in flight for one the ledger hasn't caught up
    // on yet. Completed/Deferred candidates are excluded: they are done,
    // and a PR merely mentioning their issue (e.g. in a changelog) must
    // not block selection.
    let non_terminal_issues: Vec<u64> = snapshot
        .candidates
        .iter()
        .filter(|c| !matches!(c.status, MilestoneStatus::Completed | MilestoneStatus::Deferred))
        .filter_map(|c| c.issue)
        .collect();
    let blocking_prs: Vec<&LiveOpenPr> = snapshot
        .live_open_prs
        .iter()
        .filter(|pr| {
            non_terminal_issues
                .iter()
                .any(|issue| references_issue(pr, &snapshot.repository, *issue))
        })
        .collect();
    if !blocking_prs.is_empty() {
        return SelectionDecision::Blocked(
            blocking_prs
                .iter()
                .map(|pr| SelectionBlocker {
                    kind: "active_work_must_be_dispositioned".to_owned(),
                    detail: format!(
                        "PR #{} ({:?}) is open for tracked work in program {program_id:?}; disposition it before selecting new work",
                        pr.number, pr.title
                    ),
                    pr_number: Some(pr.number),
                    pr_url: Some(pr.url.clone()),
                })
                .collect(),
        );
    }

    let completed: BTreeSet<&str> = snapshot
        .candidates
        .iter()
        .filter(|c| c.status == MilestoneStatus::Completed)
        .map(|c| c.id.as_str())
        .collect();

    let selected = snapshot.candidates.iter().find(|c| {
        c.status == MilestoneStatus::Pending
            && c.depends_on.iter().all(|dep| completed.contains(dep.as_str()))
    });

    if let Some(candidate) = selected {
        // `live_prs_available == false` is otherwise only consulted by
        // Guard A's per-InProgress-candidate loop above; when there are
        // no InProgress MILESTONE candidates to trip it (e.g. every
        // candidate is Pending, or the only InProgress ones are
        // lane-routing work items Guard A deliberately skips), an
        // unavailable live-PR fetch was invisible to the rest of
        // `select_next` and the active-work guard above sees an
        // artificially empty `live_open_prs`, silently treating "unknown"
        // as "confirmed no PR" and letting selection through. Fail closed
        // here too: a Pending candidate must never be handed out as
        // `Selected` while we cannot verify no PR is already open for it.
        if !snapshot.live_prs_available {
            return SelectionDecision::Blocked(vec![SelectionBlocker {
                kind: "live_pr_state_unavailable".to_owned(),
                detail: format!(
                    "earliest eligible candidate {:?} in {program_id}, but live PR state is unavailable; cannot verify no PR is already open for it — retry with gh access, or pass --fixture",
                    candidate.id
                ),
                pr_number: None,
                pr_url: None,
            }]);
        }

        // The active-work guard above can only correlate a live PR to a
        // candidate BY ISSUE NUMBER. A pending candidate with no `issue`
        // is therefore invisible to that guard — handing it out as
        // "Selected" here would silently defeat the single-flight
        // guarantee (a PR could already be open for exactly this
        // candidate with no way to detect it). Fail closed instead of
        // selecting: block, naming what is missing, rather than
        // guessing it's safe (see #3692).
        if candidate.issue.is_none() {
            return SelectionDecision::Blocked(vec![SelectionBlocker {
                kind: "pending_candidate_missing_issue".to_owned(),
                detail: format!(
                    "earliest eligible candidate {:?} in {program_id} has no issue number; the active-work guard cannot verify no live PR already exists for it — file a tracking issue before it can be selected",
                    candidate.id
                ),
                pr_number: None,
                pr_url: None,
            }]);
        }

        let mut reason = format!(
            "earliest eligible candidate in {program_id}: depends_on {:?} all completed, status pending; exit criteria: {}",
            candidate.depends_on, candidate.exit_criteria
        );
        if let Some(lane) = &candidate.lane {
            reason = format!("{reason} (lane: {lane})");
        }
        let mut non_goals = snapshot.non_goals.clone();
        if let Some(claim_boundary) = &candidate.claim_boundary {
            non_goals.insert(0, claim_boundary.clone());
        }
        return SelectionDecision::Selected(WorkPacket {
            issue: candidate.issue,
            id: candidate.id.clone(),
            reason,
            mode: snapshot.mode.clone(),
            ownership: candidate.ownership.clone(),
            non_goals,
            required_proof: candidate.required_proof.clone(),
            session_goal: format!("{}: {}", candidate.id, candidate.title),
            inputs_used: vec![
                format!("local git checkout: {}", snapshot.current_git_ref),
                "live gh pr list (open, this repository)".to_owned(),
                format!(".perl-lsp/goals/programs/{program_id}.toml"),
            ],
        });
    }

    let all_terminal = !snapshot.candidates.is_empty()
        && snapshot
            .candidates
            .iter()
            .all(|c| matches!(c.status, MilestoneStatus::Completed | MilestoneStatus::Deferred));
    if all_terminal {
        return SelectionDecision::Complete(CompletionEvidence {
            program: program_id.to_owned(),
            detail: format!("all {} candidates completed or deferred", snapshot.candidates.len()),
        });
    }

    SelectionDecision::Blocked(vec![SelectionBlocker {
        kind: "no_eligible_candidate".to_owned(),
        detail: format!(
            "no candidate in {program_id} has satisfied dependencies and pending status ({} candidates total)",
            snapshot.candidates.len()
        ),
        pr_number: None,
        pr_url: None,
    }])
}

pub(crate) fn ambiguity_detail(snapshot: &SelectionSnapshot) -> String {
    format!(
        "requested={:?} default={:?} known={:?}",
        snapshot.requested_program,
        snapshot.default_program,
        snapshot.known_programs.iter().map(|p| p.id.as_str()).collect::<Vec<_>>()
    )
}

fn references_issue(pr: &LiveOpenPr, repository: &str, issue: u64) -> bool {
    contains_issue_reference(&pr.title, repository, issue)
        || contains_issue_reference(&pr.body, repository, issue)
}

/// Recognizes the two GitHub-native ways a PR title/body can reference an
/// issue IN THIS REPOSITORY (see #3692): a bare `#<issue>` token (GitHub's
/// own convention — an unqualified `#N` always resolves within the repo
/// the referencing PR was opened in), or the full URL form
/// `github.com/<repository>/issues/<issue>`, which carries no literal
/// `#<issue>` token at all and would otherwise be a false negative.
fn contains_issue_reference(text: &str, repository: &str, issue: u64) -> bool {
    contains_hash_reference(text, repository, issue) || contains_issue_url(text, repository, issue)
}

/// Raw substring matching on `#<issue>` false-matches in two ways this
/// checks for:
/// - one issue number is a numeric prefix of another (candidate `#12`
///   would match a mention of `#120`; candidate `#3602` would match
///   `#36024`) — requires the character immediately after the match to be
///   end-of-string or a non-digit, so the matched `#<issue>` is not itself
///   a prefix of a longer issue reference.
/// - a same-numbered issue in a DIFFERENT repository, qualified as
///   `owner/repo#<issue>` (e.g. `other-org/other-repo#12` must not
///   false-positive candidate issue `#12` in THIS repo) — requires the
///   token immediately preceding the `#` to be either absent (a bare
///   reference) or to equal this repository's own `owner/repo` name (a
///   self-qualified reference is equivalent to a bare one).
fn contains_hash_reference(text: &str, repository: &str, issue: u64) -> bool {
    let needle = format!("#{issue}");
    text.match_indices(&needle).any(|(idx, _)| {
        let after = idx + needle.len();
        let after_ok = after >= text.len() || !text.as_bytes()[after].is_ascii_digit();
        after_ok && hash_is_scoped_to_this_repo(text, idx, repository)
    })
}

/// `hash_idx` is the byte offset of the `#` in `text`. Returns `true` when
/// the reference at that position is bare (no `owner/repo` prefix
/// immediately touching the `#`, ignoring any trailing wrapper/filler
/// punctuation) or when the touching prefix token is exactly `repository`
/// (case-insensitive, matching GitHub's own case-insensitive repo names).
///
/// Fixed from a review finding on PR #3701: an earlier version used two
/// DIFFERENT character-class checks — one (`touches_identifier`) to
/// decide whether a qualifier token is present at all, another
/// (`rfind`'s boundary set) to find where that token starts — and the two
/// sets disagreed on `)`, `]`, `,`, `:`. Whenever one of those bytes sat
/// immediately before `#` (e.g. `(other-org/other-repo)#12`,
/// `other-org/other-repo,#12`), `touches_identifier` was `false` and the
/// function returned `true` (bare/same-repo) BEFORE ever consulting the
/// `rfind` boundary set — silently false-positiving a qualified
/// different-repo reference as same-repo. This version uses ONE boundary
/// definition throughout: qualifier bytes are `[A-Za-z0-9/_.-]`; `)`,
/// `]`, `,`, and `:` immediately before `#` are transparent trailing
/// filler (skipped, since `(owner/repo)#N`, `owner/repo,#N`, and
/// `owner/repo:#N` all still qualify `owner/repo` despite the punctuation
/// sitting between the token and the `#`); anything else (whitespace, an
/// opening `(`/`[`, or start-of-string) ends the token.
fn hash_is_scoped_to_this_repo(text: &str, hash_idx: usize, repository: &str) -> bool {
    let bytes = text.as_bytes();

    // Skip trailing filler punctuation between a qualifier token and the
    // `#` itself: `(other-org/repo)#12` still names `other-org/repo`.
    let mut end = hash_idx;
    while end > 0 && matches!(bytes[end - 1], b')' | b']' | b',' | b':') {
        end -= 1;
    }

    // Scan backward from `end` for a maximal run of qualifier bytes.
    let mut start = end;
    while start > 0 {
        let b = bytes[start - 1];
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.') {
            start -= 1;
        } else {
            break;
        }
    }

    if start == end {
        // No qualifier token immediately precedes `#` (after skipping any
        // trailing filler punctuation): a bare reference.
        return true;
    }
    text[start..end].eq_ignore_ascii_case(repository)
}

/// Recognizes `https://github.com/<repository>/issues/<issue>` (any
/// scheme/case) with no literal `#<issue>` token — the fallback reference
/// form GitHub itself renders when auto-linking a full issue URL.
fn contains_issue_url(text: &str, repository: &str, issue: u64) -> bool {
    let lower_text = text.to_lowercase();
    let needle = format!("github.com/{}/issues/{issue}", repository.to_lowercase());
    lower_text.match_indices(&needle).any(|(idx, _)| {
        let after = idx + needle.len();
        after >= lower_text.len() || !lower_text.as_bytes()[after].is_ascii_digit()
    })
}

/// `cargo xtask goals reconcile` (#3696 item B). Pure comparator: given the
/// already-normalized candidates plus separately-fetched OPEN and MERGED
/// live PRs, reports drift between a milestone's self-reported ledger
/// `status` and live GitHub reality. Never mutates anything and never
/// gates `select_next` directly — Guard A above is select_next's own,
/// stricter, selection-time check; this is the broader advisory report.
///
/// Kinds emitted:
/// - `merged_pr_but_still_in_progress`: `InProgress` candidate, issue set,
///   no open PR references it, but a MERGED PR does — the ledger's status
///   is stale (mark completed with merged evidence, or split).
/// - `in_progress_without_identity`: `InProgress` candidate with no issue.
/// - `pending_without_identity` (decision 2, SOFT — deliberately NOT part
///   of `manifest::validate_milestone_ledger`'s hard violations, so
///   `check-active-goal-manifest` is never red-CI'd by this): `Pending`
///   candidate with no issue.
/// - `live_state_unavailable`: `InProgress` candidate with an issue, but
///   `live_prs_available` is `false` (the OPEN-PR fetch failed/was
///   unauthenticated). Emitted INSTEAD of ever evaluating
///   `merged_pr_but_still_in_progress` for that candidate — see the
///   `live_prs_available` parameter doc below for why.
///
/// Scoped to MILESTONE candidates only (`lane.is_none()`, mirroring Guard
/// A's scoping in `select_next` and decision 3, #3696 item B): a lane work
/// item is legitimately identity-less by design (the #3634 trust lane), so
/// it must never generate an identity-drift finding here either.
pub fn reconcile_in_progress(
    candidates: &[MilestoneCandidate],
    open_prs: &[LiveOpenPr],
    merged_prs: &[LiveOpenPr],
    repository: &str,
    // `false` when the OPEN-PR fetch (`load_live_prs`) failed/was
    // unavailable — distinct from "queried and found zero open PRs".
    // Without this, an asymmetric `gh` failure (the plain `gh pr list`
    // call errors while the separate `gh pr list --state merged --search`
    // call used for `merged_prs` succeeds — plausible, since the search
    // endpoint has different rate limits) would make `has_open` below
    // falsely read as "confirmed no open PR" when it is really "unknown",
    // producing a false-positive `merged_pr_but_still_in_progress` finding
    // for a candidate whose PR is in fact still open.
    live_prs_available: bool,
) -> Vec<ReconciliationFinding> {
    let mut findings = Vec::new();
    for candidate in candidates.iter().filter(|c| c.lane.is_none()) {
        match candidate.status {
            MilestoneStatus::InProgress => match candidate.issue {
                None => findings.push(ReconciliationFinding {
                    milestone_id: candidate.id.clone(),
                    issue: None,
                    kind: "in_progress_without_identity".to_owned(),
                    detail: format!("{}: in_progress with no issue number", candidate.id),
                    pr_number: None,
                    pr_url: None,
                }),
                Some(issue) if !live_prs_available => {
                    findings.push(ReconciliationFinding {
                        milestone_id: candidate.id.clone(),
                        issue: Some(issue),
                        kind: "live_state_unavailable".to_owned(),
                        detail: format!(
                            "{}: in_progress (#{issue}) but live open-PR state is unavailable; cannot verify whether its PR is still open or has merged — retry with gh access, or pass --fixture",
                            candidate.id
                        ),
                        pr_number: None,
                        pr_url: None,
                    });
                }
                Some(issue) => {
                    let has_open =
                        open_prs.iter().any(|pr| references_issue(pr, repository, issue));
                    if !has_open
                        && let Some(merged) =
                            merged_prs.iter().find(|pr| references_issue(pr, repository, issue))
                    {
                        findings.push(ReconciliationFinding {
                            milestone_id: candidate.id.clone(),
                            issue: Some(issue),
                            kind: "merged_pr_but_still_in_progress".to_owned(),
                            detail: format!(
                                "{}: in_progress (#{issue}) but PR #{} ({:?}) referencing it is already merged; mark completed with merged evidence or split the remaining work",
                                candidate.id, merged.number, merged.title
                            ),
                            pr_number: Some(merged.number),
                            pr_url: Some(merged.url.clone()),
                        });
                    }
                }
            },
            MilestoneStatus::Pending if candidate.issue.is_none() => {
                findings.push(ReconciliationFinding {
                    milestone_id: candidate.id.clone(),
                    issue: None,
                    kind: "pending_without_identity".to_owned(),
                    detail: format!("{}: pending with no issue number", candidate.id),
                    pr_number: None,
                    pr_url: None,
                });
            }
            _ => {}
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, status: MilestoneStatus, depends_on: &[&str]) -> MilestoneCandidate {
        MilestoneCandidate {
            id: id.to_owned(),
            title: format!("title-{id}"),
            status,
            issue: None,
            depends_on: depends_on.iter().map(|s| (*s).to_owned()).collect(),
            exit_criteria: "exit".to_owned(),
            lane: None,
            claim_boundary: None,
            ownership: Vec::new(),
            required_proof: Vec::new(),
        }
    }

    fn candidate_with_issue(
        id: &str,
        status: MilestoneStatus,
        depends_on: &[&str],
        issue: u64,
    ) -> MilestoneCandidate {
        MilestoneCandidate { issue: Some(issue), ..candidate(id, status, depends_on) }
    }

    fn base_snapshot(candidates: Vec<MilestoneCandidate>) -> SelectionSnapshot {
        SelectionSnapshot {
            repository: "EffortlessMetrics/perl-lsp-swarm".to_owned(),
            requested_program: None,
            default_program: Some("agent_loop_enablement".to_owned()),
            known_programs: vec![ProgramCandidate { id: "agent_loop_enablement".to_owned() }],
            resolved_program: Some("agent_loop_enablement".to_owned()),
            mode: "maintainer".to_owned(),
            board: None,
            program_title: None,
            tracker_issue: None,
            non_goals: vec!["no product change".to_owned()],
            candidates,
            live_open_prs: Vec::new(),
            current_git_ref: "main".to_owned(),
            live_prs_available: true,
        }
    }

    #[test]
    fn selection_precedence_picks_earliest_eligible_by_depends_on() {
        // Corrected per #3696 item B: this test originally hard-coded M3
        // `InProgress` with NO issue and NO live PR, and asserted
        // `Selected(M4)` -- that is exactly THE BUG Guard A fixes (a
        // stale/identity-less in_progress milestone silently falling
        // through to a later Pending sibling instead of blocking for
        // reconciliation; see
        // `in_progress_without_live_pr_blocks_reconciliation_not_selection_of_next`
        // below for the dedicated regression test). M3 is made `Completed`
        // here so Guard A has nothing to trip on, isolating this test's
        // actual purpose: depends_on-ordering among Pending siblings.
        let snapshot = base_snapshot(vec![
            candidate("M2", MilestoneStatus::Completed, &[]),
            candidate("M3", MilestoneStatus::Completed, &["M2"]),
            candidate_with_issue("M4", MilestoneStatus::Pending, &["M2"], 9994),
            candidate_with_issue("M5", MilestoneStatus::Pending, &["M2"], 9995),
        ]);

        let decision = select_next(&snapshot);
        match decision {
            SelectionDecision::Selected(packet) => assert_eq!(packet.id, "M4"),
            other => panic!("expected Selected(M4), got {other:?}"),
        }
    }

    #[test]
    fn selection_respects_unsatisfied_depends_on() {
        let snapshot = base_snapshot(vec![
            candidate_with_issue("M2", MilestoneStatus::Pending, &[], 9992),
            candidate_with_issue("M3", MilestoneStatus::Pending, &["M2"], 9993),
        ]);

        let decision = select_next(&snapshot);
        match decision {
            SelectionDecision::Selected(packet) => assert_eq!(packet.id, "M2"),
            other => panic!("expected Selected(M2), got {other:?}"),
        }
    }

    #[test]
    fn pending_candidate_with_no_issue_is_blocked_not_selected() {
        // Regression for #3692 defect 2: the active-work guard can only
        // correlate a live PR to a candidate BY ISSUE NUMBER, so a pending
        // candidate with no issue must never be silently handed out as
        // "Selected" — that would defeat the single-flight guarantee for
        // exactly the candidate shape most likely to have a PR opened
        // for it before the ledger is updated with an issue number. This
        // mirrors the live `agent_loop_enablement.toml` ledger's M4-M7
        // shape (pending, no issue) so the fix does not require touching
        // production ledger data.
        let snapshot = base_snapshot(vec![
            candidate("M2", MilestoneStatus::Completed, &[]),
            candidate("M4", MilestoneStatus::Pending, &["M2"]),
        ]);

        let decision = select_next(&snapshot);

        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers.len(), 1);
                assert_eq!(blockers[0].kind, "pending_candidate_missing_issue");
                assert!(blockers[0].detail.contains("M4"));
                assert!(blockers[0].pr_number.is_none());
            }
            other => panic!("expected Blocked(pending_candidate_missing_issue), got {other:?}"),
        }
    }

    #[test]
    fn ambiguous_program_authority_blocks() {
        let mut snapshot = base_snapshot(vec![candidate("M2", MilestoneStatus::Pending, &[])]);
        snapshot.resolved_program = None;
        snapshot.default_program = None;
        snapshot.known_programs =
            vec![ProgramCandidate { id: "a".to_owned() }, ProgramCandidate { id: "b".to_owned() }];

        let decision = select_next(&snapshot);
        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers.len(), 1);
                assert_eq!(blockers[0].kind, "ambiguous_program_authority");
                assert!(blockers[0].pr_number.is_none());
            }
            other => panic!("expected Blocked(ambiguous_program_authority), got {other:?}"),
        }
    }

    #[test]
    fn active_same_lane_work_conflict_blocks_and_identifies_pr_without_mutation() {
        let mut m3 = candidate("M3", MilestoneStatus::InProgress, &["M2"]);
        m3.issue = Some(3624);
        let mut snapshot =
            base_snapshot(vec![candidate("M2", MilestoneStatus::Completed, &[]), m3]);
        snapshot.live_open_prs = vec![LiveOpenPr {
            number: 4242,
            title: "feat(goals): M3 selector (#3624)".to_owned(),
            body: "Part of #3612".to_owned(),
            url: "https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/4242".to_owned(),
            is_draft: true,
        }];
        let before = snapshot.clone();

        let decision = select_next(&snapshot);

        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers.len(), 1);
                assert_eq!(blockers[0].kind, "active_work_must_be_dispositioned");
                assert_eq!(blockers[0].pr_number, Some(4242));
                assert!(blockers[0].pr_url.as_deref().unwrap().contains("4242"));
            }
            other => panic!("expected Blocked(active_work_must_be_dispositioned), got {other:?}"),
        }
        // Read-only: select_next must not mutate the snapshot it was given
        // (no PR close/mutate call exists in this module at all).
        assert_eq!(before.candidates.len(), snapshot.candidates.len());
        assert_eq!(before.live_open_prs.len(), snapshot.live_open_prs.len());
    }

    #[test]
    fn pending_candidate_with_live_pr_blocks_selection_even_though_not_in_progress() {
        // Live GitHub state outranks the manifest's self-reported status
        // (CLAUDE.md truth hierarchy): a PR already open for a "pending"
        // candidate (ledger not yet updated to "in_progress") must still
        // block, not be silently skipped in favor of a different pending
        // sibling.
        let mut m3 = candidate("M3", MilestoneStatus::Pending, &["M2"]);
        m3.issue = Some(3624);
        let mut snapshot = base_snapshot(vec![
            candidate("M2", MilestoneStatus::Completed, &[]),
            m3,
            candidate("M4", MilestoneStatus::Pending, &["M2"]),
        ]);
        snapshot.live_open_prs = vec![LiveOpenPr {
            number: 4242,
            title: "feat(goals): M3 selector (#3624)".to_owned(),
            body: String::new(),
            url: "https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/4242".to_owned(),
            is_draft: true,
        }];

        let decision = select_next(&snapshot);

        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers[0].kind, "active_work_must_be_dispositioned");
                assert_eq!(blockers[0].pr_number, Some(4242));
            }
            other => panic!("expected Blocked(active_work_must_be_dispositioned), got {other:?}"),
        }
    }

    #[test]
    fn completed_candidate_referenced_by_a_pr_does_not_block_selection() {
        // A PR that merely mentions a *completed* candidate's issue (e.g. a
        // changelog entry, a "see also") must not block selection of new
        // work — only non-terminal candidates participate in the guard.
        let mut m2 = candidate("M2", MilestoneStatus::Completed, &[]);
        m2.issue = Some(3614);
        let mut snapshot = base_snapshot(vec![
            m2,
            candidate_with_issue("M3", MilestoneStatus::Pending, &["M2"], 9993),
        ]);
        snapshot.live_open_prs = vec![LiveOpenPr {
            number: 5000,
            title: "docs: mention M2 (#3614) in the changelog".to_owned(),
            body: String::new(),
            url: "u".to_owned(),
            is_draft: false,
        }];

        let decision = select_next(&snapshot);

        match decision {
            SelectionDecision::Selected(packet) => assert_eq!(packet.id, "M3"),
            other => panic!("expected Selected(M3), got {other:?}"),
        }
    }

    #[test]
    fn references_issue_does_not_prefix_match_a_longer_issue_number() {
        // Candidate issue #12 must not be considered referenced by a PR
        // that only mentions #120 or #1234 — `references_issue` requires a
        // non-digit boundary after the match.
        let mut m2 = candidate("M2", MilestoneStatus::InProgress, &[]);
        m2.issue = Some(12);
        let mut snapshot = base_snapshot(vec![m2]);
        snapshot.live_open_prs = vec![LiveOpenPr {
            number: 1,
            title: "unrelated work (#120)".to_owned(),
            body: "see also #1234".to_owned(),
            url: "u".to_owned(),
            is_draft: false,
        }];

        let decision = select_next(&snapshot);

        // M2 is `InProgress` with no genuinely-matching PR, so Guard A
        // (#3696 item B) now classifies it as `NoLivePr` and blocks with
        // `in_progress_state_requires_reconciliation` before rule 3 (the
        // active-work-conflict check) or rule 4 ever run — the
        // prefix-matching guard this test exists for is still exercised:
        // if `#120`/`#1234` falsely matched `#12`, this would instead
        // reconcile as `Reconciled` and fall through to a different
        // outcome entirely.
        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers[0].kind, "in_progress_state_requires_reconciliation");
                assert_eq!(blockers[0].pr_number, None);
            }
            other => {
                panic!("expected Blocked(in_progress_state_requires_reconciliation), got {other:?}")
            }
        }
    }

    #[test]
    fn cross_repo_hash_reference_does_not_false_positive_the_guard() {
        // Regression for #3692 defect 3: a PR that mentions a
        // *different* repository's same-numbered issue
        // (`other-org/other-repo#12`) must not be mistaken for a
        // reference to THIS repository's candidate issue #12 — plain
        // substring matching on "#12" would false-positive here.
        let mut m2 = candidate("M2", MilestoneStatus::InProgress, &[]);
        m2.issue = Some(12);
        let mut snapshot = base_snapshot(vec![m2]);
        snapshot.live_open_prs = vec![LiveOpenPr {
            number: 1,
            title: "unrelated cross-repo mention".to_owned(),
            body: "see other-org/other-repo#12 for background".to_owned(),
            url: "u".to_owned(),
            is_draft: false,
        }];

        let decision = select_next(&snapshot);

        // A different repo's #12 must not be treated as referencing THIS
        // candidate; Guard A still blocks (M2 is genuinely in_progress
        // with no matching PR), but as `in_progress_state_requires_reconciliation`,
        // never `active_work_must_be_dispositioned`.
        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(
                    blockers[0].kind, "in_progress_state_requires_reconciliation",
                    "a different repo's #12 must not trip active_work_must_be_dispositioned"
                );
            }
            other => {
                panic!("expected Blocked(in_progress_state_requires_reconciliation), got {other:?}")
            }
        }
    }

    #[test]
    fn cross_repo_hash_reference_with_adjacent_punctuation_does_not_false_positive() {
        // Regression for the factory-droid P1 finding on PR #3701: the
        // original `touches_identifier` byte-class check and the
        // separate `rfind` boundary-char set disagreed on `)`, `]`,
        // `,`, and `:` — whenever one of those bytes sat immediately
        // before `#` with no space (a qualified cross-repo reference
        // wrapped in punctuation), the function returned `true` (bare/
        // same-repo) before ever consulting the boundary set, false-
        // positiving the guard. Every PR body below names a DIFFERENT
        // repo's issue #12 with no space before the `#`.
        let mut m2 = candidate("M2", MilestoneStatus::InProgress, &[]);
        m2.issue = Some(12);
        let mut snapshot = base_snapshot(vec![m2]);
        snapshot.live_open_prs = vec![
            LiveOpenPr {
                number: 1,
                title: "t".to_owned(),
                body: "see (other-org/other-repo)#12".to_owned(),
                url: "u".to_owned(),
                is_draft: false,
            },
            LiveOpenPr {
                number: 2,
                title: "t".to_owned(),
                body: "[other-org/other-repo]#12".to_owned(),
                url: "u".to_owned(),
                is_draft: false,
            },
            LiveOpenPr {
                number: 3,
                title: "t".to_owned(),
                body: "compare with other-org/other-repo,#12".to_owned(),
                url: "u".to_owned(),
                is_draft: false,
            },
            LiveOpenPr {
                number: 4,
                title: "t".to_owned(),
                body: "see other-org/other-repo:#12 above".to_owned(),
                url: "u".to_owned(),
                is_draft: false,
            },
        ];

        let decision = select_next(&snapshot);

        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(
                    blockers[0].kind, "in_progress_state_requires_reconciliation",
                    "punctuation-wrapped cross-repo #12 mentions must not trip active_work_must_be_dispositioned"
                );
            }
            other => {
                panic!("expected Blocked(in_progress_state_requires_reconciliation), got {other:?}")
            }
        }
    }

    #[test]
    fn self_repo_qualified_hash_reference_still_matches() {
        // A same-repo qualified reference (`owner/repo#N`) is equivalent
        // to a bare `#N` and must still block — only a DIFFERENT repo's
        // qualified reference should be excluded.
        let mut m3 = candidate("M3", MilestoneStatus::InProgress, &["M2"]);
        m3.issue = Some(3624);
        let mut snapshot =
            base_snapshot(vec![candidate("M2", MilestoneStatus::Completed, &[]), m3]);
        snapshot.live_open_prs = vec![LiveOpenPr {
            number: 4242,
            title: "feat(goals): M3 selector".to_owned(),
            body: "See EffortlessMetrics/perl-lsp-swarm#3624 for the epic".to_owned(),
            url: "https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/4242".to_owned(),
            is_draft: true,
        }];

        let decision = select_next(&snapshot);

        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers[0].kind, "active_work_must_be_dispositioned");
                assert_eq!(blockers[0].pr_number, Some(4242));
            }
            other => panic!("expected Blocked(active_work_must_be_dispositioned), got {other:?}"),
        }
    }

    #[test]
    fn full_issue_url_with_no_hash_token_still_matches() {
        // Regression for #3692 defect 3: a PR body that references the
        // candidate's issue only via the full GitHub URL (no literal
        // `#3624` token anywhere) must still trip the active-work guard.
        let mut m3 = candidate("M3", MilestoneStatus::InProgress, &["M2"]);
        m3.issue = Some(3624);
        let mut snapshot =
            base_snapshot(vec![candidate("M2", MilestoneStatus::Completed, &[]), m3]);
        snapshot.live_open_prs = vec![LiveOpenPr {
            number: 4242,
            title: "feat(goals): M3 selector".to_owned(),
            body: "See https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3624 for context"
                .to_owned(),
            url: "https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/4242".to_owned(),
            is_draft: true,
        }];

        let decision = select_next(&snapshot);

        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers[0].kind, "active_work_must_be_dispositioned");
                assert_eq!(blockers[0].pr_number, Some(4242));
            }
            other => panic!("expected Blocked(active_work_must_be_dispositioned), got {other:?}"),
        }
    }

    #[test]
    fn inputs_used_reflects_the_actual_current_git_ref_not_a_hardcoded_literal() {
        // Regression for #3692 defect 6: `inputs_used` must be populated
        // from the snapshot's measured `current_git_ref`, not a
        // hardcoded "origin/main" literal — so a selection made from a
        // feature-branch checkout is honestly attributed.
        let mut snapshot =
            base_snapshot(vec![candidate_with_issue("M4", MilestoneStatus::Pending, &[], 9994)]);
        snapshot.current_git_ref = "feature/some-other-branch".to_owned();

        let decision = select_next(&snapshot);

        match decision {
            SelectionDecision::Selected(packet) => {
                assert!(
                    packet.inputs_used.iter().any(|i| i.contains("feature/some-other-branch")),
                    "expected inputs_used to name the measured current_git_ref, got {:?}",
                    packet.inputs_used
                );
                assert!(
                    !packet.inputs_used.iter().any(|i| i == "origin/main"),
                    "inputs_used must not contain the old hardcoded literal"
                );
            }
            other => panic!("expected Selected(M4), got {other:?}"),
        }
    }

    #[test]
    fn empty_candidates_is_graceful_not_a_panic() {
        let snapshot = base_snapshot(Vec::new());
        let decision = select_next(&snapshot);
        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers[0].kind, "no_eligible_candidate");
            }
            other => panic!("expected Blocked(no_eligible_candidate), got {other:?}"),
        }
    }

    #[test]
    fn all_completed_reports_complete() {
        let snapshot = base_snapshot(vec![
            candidate("M2", MilestoneStatus::Completed, &[]),
            candidate("M3", MilestoneStatus::Deferred, &["M2"]),
        ]);
        let decision = select_next(&snapshot);
        match decision {
            SelectionDecision::Complete(evidence) => {
                assert_eq!(evidence.program, "agent_loop_enablement");
            }
            other => panic!("expected Complete, got {other:?}"),
        }
    }

    #[test]
    fn deterministic_byte_stable_output_for_fixed_snapshot() {
        let snapshot = base_snapshot(vec![
            candidate("M2", MilestoneStatus::Completed, &[]),
            candidate("M3", MilestoneStatus::Pending, &["M2"]),
        ]);

        let first = serde_json::to_string(&select_next(&snapshot))
            .unwrap_or_else(|e| panic!("serialize failed: {e}"));
        let second = serde_json::to_string(&select_next(&snapshot))
            .unwrap_or_else(|e| panic!("serialize failed: {e}"));
        assert_eq!(first, second);
    }

    // --- #3696 item B: Guard A (in-progress reconciliation) + `goals
    // reconcile`. ---

    #[test]
    fn in_progress_without_live_pr_blocks_reconciliation_not_selection_of_next() {
        // THE regression this PR exists to fix: M3 in_progress (#3624) with
        // no open PR referencing it (its PR merged, not open — see
        // `agent_loop_enablement.toml`) must not silently fall through to
        // selecting a later Pending sibling (M4). The ledger's self-reported
        // status has drifted from live reality and needs `goals reconcile`
        // or a ledger update, not a new work assignment.
        let mut m3 = candidate("M3", MilestoneStatus::InProgress, &["M2"]);
        m3.issue = Some(3624);
        let snapshot = base_snapshot(vec![
            candidate("M2", MilestoneStatus::Completed, &[]),
            m3,
            candidate("M4", MilestoneStatus::Pending, &["M2"]),
        ]);

        let decision = select_next(&snapshot);
        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers[0].kind, "in_progress_state_requires_reconciliation");
                assert_eq!(blockers[0].pr_number, None);
            }
            other => {
                panic!("expected Blocked(in_progress_state_requires_reconciliation), got {other:?}")
            }
        }
    }

    #[test]
    fn in_progress_candidate_blocks_when_live_pr_state_is_unavailable() {
        // `live_prs_available == false` (adapter could not reach `gh`) must
        // be checked BEFORE "no matching PR" — unknown state is never
        // treated as equivalent to "confirmed no PR".
        let mut m2 = candidate("M2", MilestoneStatus::InProgress, &[]);
        m2.issue = Some(1234);
        let mut snapshot = base_snapshot(vec![m2]);
        snapshot.live_prs_available = false;

        let decision = select_next(&snapshot);
        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers[0].kind, "in_progress_state_requires_reconciliation");
                assert_eq!(blockers[0].pr_number, None);
            }
            other => {
                panic!("expected Blocked(in_progress_state_requires_reconciliation), got {other:?}")
            }
        }
    }

    #[test]
    fn pending_selection_blocks_when_live_pr_state_is_unavailable_and_no_in_progress_candidates() {
        // coderabbit (select.rs:327) / chatgpt-codex (snapshot.rs:228):
        // `live_prs_available` was previously only consulted by Guard A's
        // per-InProgress-MILESTONE-candidate loop above. When there are NO
        // such candidates (e.g. every candidate is Pending, or the only
        // InProgress ones are lane-routing work items Guard A
        // deliberately skips), an unavailable live-PR fetch was invisible
        // to the rest of `select_next`: rule 3's active-work guard sees an
        // artificially empty `live_open_prs` and finds nothing to block
        // on, so a Pending candidate with an issue could still be handed
        // out as `Selected` even though we cannot verify no PR is already
        // open for it -- a real regression from the prior hard-error
        // behavior on a failed `gh pr list`. This must now block instead.
        let snapshot = {
            let mut s = base_snapshot(vec![candidate_with_issue(
                "M4",
                MilestoneStatus::Pending,
                &[],
                9994,
            )]);
            s.live_prs_available = false;
            s
        };

        let decision = select_next(&snapshot);
        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers.len(), 1);
                assert_eq!(blockers[0].kind, "live_pr_state_unavailable");
                assert!(blockers[0].detail.contains("M4"));
                assert!(blockers[0].pr_number.is_none());
            }
            other => panic!("expected Blocked(live_pr_state_unavailable), got {other:?}"),
        }
    }

    #[test]
    fn multiple_open_prs_referencing_the_same_in_progress_candidate_reclassify_to_reconciliation() {
        // Decision 4 (#3696 item B): ambiguity from more than one matching
        // open PR reclassifies from `active_work_must_be_dispositioned` to
        // `in_progress_state_requires_reconciliation` — ambiguity is a
        // reconciliation problem, not a plain single-flight conflict.
        let mut m3 = candidate("M3", MilestoneStatus::InProgress, &["M2"]);
        m3.issue = Some(3624);
        let mut snapshot =
            base_snapshot(vec![candidate("M2", MilestoneStatus::Completed, &[]), m3]);
        snapshot.live_open_prs = vec![
            LiveOpenPr {
                number: 100,
                title: "feat: M3 attempt 1 (#3624)".to_owned(),
                body: String::new(),
                url: "u1".to_owned(),
                is_draft: true,
            },
            LiveOpenPr {
                number: 200,
                title: "feat: M3 attempt 2 (#3624)".to_owned(),
                body: String::new(),
                url: "u2".to_owned(),
                is_draft: true,
            },
        ];

        let decision = select_next(&snapshot);
        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers.len(), 2);
                for blocker in &blockers {
                    assert_eq!(blocker.kind, "in_progress_state_requires_reconciliation");
                }
                let pr_numbers: Vec<Option<u64>> = blockers.iter().map(|b| b.pr_number).collect();
                assert_eq!(pr_numbers, vec![Some(100), Some(200)]);
            }
            other => panic!(
                "expected Blocked(in_progress_state_requires_reconciliation) x2, got {other:?}"
            ),
        }
    }

    /// A lane-routing work item candidate — `lane: Some(_)`, the same
    /// discriminator `load_lane_routing_candidates` sets (milestone
    /// candidates always have `lane: None` via `candidate` above).
    fn lane_candidate(id: &str, status: MilestoneStatus, lane: &str) -> MilestoneCandidate {
        let mut c = candidate(id, status, &[]);
        c.lane = Some(lane.to_owned());
        c
    }

    #[test]
    fn lane_work_item_in_progress_without_issue_does_not_trip_guard_a() {
        // Guard A is MILESTONE-only (decision 3, #3696 item B): a
        // lane-routing work item (`lane: Some(_)`) that is in_progress
        // with no issue is legitimately identity-less by design (the
        // #3634 trust lane) and must never trip
        // `in_progress_state_requires_reconciliation`. With no Pending
        // sibling at all, this must fall all the way through to
        // `no_eligible_candidate` -- exactly the real
        // `real_perl_editor_trust` lane's current live shape (one
        // in_progress work item, no issue, no Pending sibling).
        let in_flight = lane_candidate("wi-1", MilestoneStatus::InProgress, "reliability");
        let snapshot = base_snapshot(vec![in_flight]);

        let decision = select_next(&snapshot);
        match decision {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers.len(), 1);
                assert_eq!(blockers[0].kind, "no_eligible_candidate");
            }
            other => panic!("expected Blocked(no_eligible_candidate), got {other:?}"),
        }
    }

    #[test]
    fn lane_work_item_in_progress_without_issue_still_allows_a_pending_sibling_to_be_selected() {
        // Same guard, but with a Pending sibling present: Guard A must not
        // hard-block on the identity-less in_progress lane item, so the
        // Pending sibling (which carries its own issue, satisfying the
        // unrelated `pending_candidate_missing_issue` gate from #3701) is
        // still selectable. This is the scenario the reviewer flagged:
        // "it only looks harmless today because that lane has zero
        // Pending siblings -- add one and Guard A wrongly hard-blocks it."
        let in_flight = lane_candidate("wi-1", MilestoneStatus::InProgress, "reliability");
        let mut pending = lane_candidate("wi-2", MilestoneStatus::Pending, "reliability");
        pending.issue = Some(4242);
        let snapshot = base_snapshot(vec![in_flight, pending]);

        let decision = select_next(&snapshot);
        match decision {
            SelectionDecision::Selected(packet) => assert_eq!(packet.id, "wi-2"),
            other => panic!("expected Selected(wi-2), got {other:?}"),
        }
    }

    fn reconciliation_candidate(
        id: &str,
        status: MilestoneStatus,
        issue: Option<u64>,
    ) -> MilestoneCandidate {
        let mut c = candidate(id, status, &[]);
        c.issue = issue;
        c
    }

    #[test]
    fn reconcile_in_progress_flags_a_merged_pr_for_an_in_progress_candidate() {
        let candidates =
            vec![reconciliation_candidate("M3", MilestoneStatus::InProgress, Some(3624))];
        let merged = vec![LiveOpenPr {
            number: 4242,
            title: "feat(goals): M3 selector (#3624)".to_owned(),
            body: String::new(),
            url: "https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/4242".to_owned(),
            is_draft: false,
        }];

        let findings = reconcile_in_progress(
            &candidates,
            &[],
            &merged,
            "EffortlessMetrics/perl-lsp-swarm",
            true,
        );

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].milestone_id, "M3");
        assert_eq!(findings[0].kind, "merged_pr_but_still_in_progress");
        assert_eq!(findings[0].pr_number, Some(4242));
    }

    #[test]
    fn reconcile_in_progress_does_not_flag_a_healthy_single_flight_in_progress_candidate() {
        let candidates =
            vec![reconciliation_candidate("M3", MilestoneStatus::InProgress, Some(3624))];
        let open = vec![LiveOpenPr {
            number: 4242,
            title: "feat(goals): M3 selector (#3624)".to_owned(),
            body: String::new(),
            url: "u".to_owned(),
            is_draft: true,
        }];

        let findings = reconcile_in_progress(
            &candidates,
            &open,
            &[],
            "EffortlessMetrics/perl-lsp-swarm",
            true,
        );
        assert!(findings.is_empty(), "expected no findings, got {findings:?}");
    }

    #[test]
    fn reconcile_in_progress_flags_in_progress_and_pending_without_identity() {
        let candidates = vec![
            reconciliation_candidate("M3", MilestoneStatus::InProgress, None),
            reconciliation_candidate("M4", MilestoneStatus::Pending, None),
            reconciliation_candidate("M5", MilestoneStatus::Pending, Some(5001)),
        ];

        let findings =
            reconcile_in_progress(&candidates, &[], &[], "EffortlessMetrics/perl-lsp-swarm", true);

        assert!(
            findings
                .iter()
                .any(|f| f.milestone_id == "M3" && f.kind == "in_progress_without_identity")
        );
        assert!(
            findings.iter().any(|f| f.milestone_id == "M4" && f.kind == "pending_without_identity")
        );
        assert!(!findings.iter().any(|f| f.milestone_id == "M5"));
    }

    #[test]
    fn reconcile_in_progress_never_flags_a_lane_work_item() {
        // `reconcile_in_progress` mirrors Guard A's milestone-only scoping
        // (`lane.is_none()`): a lane work item (`lane: Some(_)`) that is
        // in_progress or pending with no issue is expected to lack one by
        // design and must never generate an identity-drift finding here.
        let mut lane_in_progress =
            reconciliation_candidate("wi-1", MilestoneStatus::InProgress, None);
        lane_in_progress.lane = Some("reliability".to_owned());
        let mut lane_pending = reconciliation_candidate("wi-2", MilestoneStatus::Pending, None);
        lane_pending.lane = Some("reliability".to_owned());

        let findings = reconcile_in_progress(
            &[lane_in_progress, lane_pending],
            &[],
            &[],
            "EffortlessMetrics/perl-lsp-swarm",
            true,
        );

        assert!(findings.is_empty(), "expected no findings for lane work items, got {findings:?}");
    }

    #[test]
    fn reconcile_in_progress_never_falsifies_merged_pr_but_still_in_progress_when_live_state_unavailable()
     {
        // factory-droid P2 finding on this PR: `reconcile_in_progress`
        // previously had no way to distinguish "the open-PR fetch found
        // zero matches" from "the open-PR fetch failed/was unavailable".
        // Reproduces the exact asymmetric-gh-failure scenario: the plain
        // `gh pr list` call (open PRs) failed (`live_prs_available =
        // false`, `open_prs` empty), while the separate `gh pr list
        // --state merged --search` call succeeded and found a PR that
        // references this candidate's issue AND is in fact still open in
        // reality (simulated here by a merged-PR fixture entry whose
        // number would otherwise trip `merged_pr_but_still_in_progress`).
        // With `live_prs_available: false`, this must now emit
        // `live_state_unavailable` instead of the false-positive
        // `merged_pr_but_still_in_progress`.
        let candidates =
            vec![reconciliation_candidate("M3", MilestoneStatus::InProgress, Some(3624))];
        let merged = vec![LiveOpenPr {
            number: 4242,
            title: "feat(goals): M3 selector (#3624)".to_owned(),
            body: String::new(),
            url: "https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/4242".to_owned(),
            is_draft: false,
        }];

        let findings = reconcile_in_progress(
            &candidates,
            &[],
            &merged,
            "EffortlessMetrics/perl-lsp-swarm",
            false,
        );

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].milestone_id, "M3");
        assert_eq!(
            findings[0].kind, "live_state_unavailable",
            "must never emit merged_pr_but_still_in_progress when live_prs_available is false, got {findings:?}"
        );
        assert!(findings[0].pr_number.is_none(), "must not name a PR it cannot actually confirm");
    }
}
