# Reviewer Network Archaeology
## How Reviewers Shifted From Human-Heavy To Mixed To AI-Reviewed-AI

The reviewer story in this repository is not just "more bots over time." It is a
network evolution. Different eras use different reviewer mixes, and the mix
tracks the workflow shape of the era.

This note is limited to what the GitHub PR archive actually shows: reviewer
identities, review counts, comment counts, and the workflow labels attached to a
few representative PRs.

---

## 1. The Early Network Is Human-Led But Already Multi-Agent

PR `#153`,
`Sync master improvements: Agent refactoring and customization features`,
is the clearest early signal that review was already distributed across humans
and machines.

Verified archive facts:

- `35` reviews
- `100` comments
- review effort marked `4/5`
- labels such as `review:stage:sweep-initial`, `review:stage:sweep-final`,
  `gate:hygiene`, `gate:matrix`, `gate:fuzz (clean)`, `gate:security (clean)`,
  `gate:policy (clear)`, and `merge-ready`

Reviewer identities on that PR included:

- `EffortlessSteven`
- `copilot-pull-request-reviewer`
- `gemini-code-assist`
- `bito-code-review`
- `chatgpt-codex-connector`
- `codiumai-pr-agent-free`
- `coderabbitai`

That is already a reviewer network, not a single reviewer. The maintainer is
present, but bot reviewers are normal enough that the PR comment thread reads
like a shared review lane.

PR `#160` shows the same early pattern with a smaller surface:

- `13` reviews
- `57` comments
- labels including `review:stage:intake`, `gate:docs (clean)`,
  `gate:perf (ok)`, `gate:policy (blocked)`, `gate:policy (clear)`,
  `integrative-review`, `arch:aligned`, and `schema:aligned`

Its review identities were:

- `copilot-pull-request-reviewer`
- `chatgpt-codex-connector`
- `coderabbitai`

So the early network is not "human-only." It is human-led, but already mixed.

---

## 2. Q3 Makes AI-Reviewing-AI Normal

PR `#209`,
`feat(dap): Phase 1 DAP support - Bridge to Perl::LanguageServer (#207)`,
is the cleanest evidence that AI-reviewing-AI became a normal operating mode.

Verified archive facts:

- `6` reviews
- `29` comments
- labels including `review:stage:intake`, `merge-ready`,
  `gate:docs (clean)`, `gate:perf (ok)`, `gate:tests (pass)`,
  `gate:security (clean)`, `gate:policy (clear)`, `state:in-progress`,
  `state:ready`, `ready-to-merge`, and `flow:integrative`

Every review on that PR came from automated reviewers:

- `copilot-pull-request-reviewer`
- `gemini-code-assist`
- `chatgpt-codex-connector`
- `chatgpt-codex-connector`
- `coderabbitai`
- `coderabbitai`

That matters because it shows the reviewer network is no longer just "bots help
the maintainer." The bots are now reviewing one another, and the PR carries the
state machine around with it via labels.

This lines up with the canonical Q3 flow naming:

- `generative` = `issue-to-draft`
- `review` = `draft-to-pr`
- `integration` = `pr-to-merge`

The reviewer network belongs to the middle lane, but the archive shows the lane
is already machine-dense by this point.

---

## 3. The Gate Era Narrows Review Comments But Widens The Trust Surface

PR `#533`,
`feat: implement standardized CI gate harness`,
shows a different reviewer network shape.

Verified archive facts:

- `2` reviews
- `3` comments
- review identities:
  - `gemini-code-assist`
  - `copilot-pull-request-reviewer`

The point here is not that review got smaller because quality dropped. The
point is that the trust burden moved out of the review thread and into gates,
receipts, benchmark baselines, and CI policy.

That is visible in the PR itself:

- the comment thread is thin
- the review identities are fewer
- the infrastructure around the PR is much larger

So the reviewer network does not disappear. It becomes one layer in a wider
verification stack.

---

## 4. Reviewer Identities Map To Workflow Eras

The archive suggests a rough identity map by era:

- Early mixed-review era: `EffortlessSteven`, `copilot-pull-request-reviewer`,
  `gemini-code-assist`, `chatgpt-codex-connector`, `coderabbitai`,
  `codiumai-pr-agent-free`, `bito-code-review`
- Q3 swarm era: the same bot reviewer set becomes the dominant review network,
  with labels and staged flows carrying more of the orchestration
- Later gate era: fewer review comments, more CI/gate/receipt machinery, and
  reviewer identities mostly reduced to the bot reviewers that remain attached
  to the PR

The important historical point is that the reviewer network evolves with the
workflow:

- direct delivery can tolerate a broad, ad hoc review surface
- staged swarm delivery turns review into a machine-dense lane
- gate-based delivery pushes trust outward into receipts and CI

That is why the reviewer identities are historically useful. They are not just
names in comments. They are signatures of the workflow era.

---

## 5. What The Archive Actually Proves

The archive proves a few bounded things:

- human-only review was not the norm once the PR surfaces got serious
- mixed human/bot review was already normal by the early PR-heavy era
- AI-reviewing-AI is explicit and normal in the Q3 swarm period
- later gate-heavy PRs carry less review-thread burden because trust moved into
  CI, receipts, and policy

It does not prove that every review was equally effective, or that every era was
perfect. It does prove that the reviewer network changed shape as the delivery
model changed shape.

---

## Evidence Pointers

- PR `#153`
- PR `#160`
- PR `#209`
- PR `#533`
- [`REVIEW_LABEL_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/REVIEW_LABEL_ARCHAEOLOGY.md)
- [`PR_REVIEW_RECEIPT_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_REVIEW_RECEIPT_ARCHAEOLOGY.md)
- [`Q3_SWARM_PR_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md)
- [`RECEIPTS_LIE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/RECEIPTS_LIE_ARCHAEOLOGY.md)
