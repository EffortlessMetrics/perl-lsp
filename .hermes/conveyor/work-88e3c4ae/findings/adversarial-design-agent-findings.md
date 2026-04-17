# Adversarial Design Findings — work-88e3c4ae

## Current Approach

The plan proposes a three-phase approach: (1) extend SweepReport schema and build a 30-fixture "malformed fixture bank" to measure recovery salvage, (2) create `.ci/metrics/parser.json` with all five new metrics as `improvement` entries and surface them in `parser.md`, and (3) write documentation. The rationale is that error density is almost implementable but recovery salvage requires new infrastructure, so foundation must come before scorecard wiring. All new metrics start as `improvement` (non-blocking) per the ratchet model.

---

## Alternative Approaches

### Alternative 1: Measure Recovery Salvage on Real Dirty Files Instead of a Synthetic Fixture Bank

**Core idea:** The plan builds 30 synthetic broken Perl files to measure "recovery salvage." Instead, run the salvage metric directly on the 157 real dirty files already identified in the Ubuntu corpus baseline. For each dirty file: parse it (dirty tree) → run semantic analysis → measure whether LSP features produce usable output. The salvage rate is the fraction of dirty files that still yield useful semantic results.

**Why it might be better:**
- **Representativeness**: Real dirty files have authentic error patterns, not artificially injected ones. The error bucket distribution in `parser-corpus-baseline.json` (26 `unexpected_token_in_expr`, 20 `unclosed_paren_identifier`, etc.) tells you exactly which error types to measure.
- **No fixture maintenance burden**: A synthetic fixture bank requires designing, validating, and maintaining 30+ fixtures. Real corpus files need no maintenance—they change organically as the corpus is updated.
- **Eliminates the "broken vs. clean" pairing problem**: The plan requires each broken fixture to have a "clean" version alongside it for comparison. Real dirty files don't have this problem—you compare dirty-file semantics against what the same file *would* produce if fixed, which is a superset of the clean parse result.
- **Removes the arbitrary 50% threshold question**: The plan's salvage definition (`salvage_rate >= 0.5` threshold) is invented. Measuring "can we still do hover/completion/goto on this dirty file?" has a natural threshold: yes or no. Aggregate to % of dirty files where it works.

**Why it might be worse:**
- Symbol extraction on dirty parse trees may fail for reasons unrelated to recovery quality (e.g., the semantic analyzer explicitly bails on trees with errors). If `SemanticModel::extract_symbols()` is not resilient to dirty trees, this approach fails where the synthetic fixture approach could be controlled.
- Dirty files have varying error severity—some have 1 error, some have 20. The salvage measurement needs to stratify by error count or the metric is too noisy.
- The sweep pipeline would need to run semantic analysis on dirty files, which adds cost to a job that currently only does parsing.

**What it sacrifices:**
- The plan's before/after protocol (broken fixture vs. clean fixture) gives a clean ratio. Real-file salvage on dirty trees is a noisier measurement because you can't precisely control the "before" state.

---

### Alternative 2: Skip the Scorecard JSON Entirely; Extend the Existing Sweep Receipt Schema Instead

**Core idea:** Instead of creating `.ci/metrics/parser.json` as a new artifact, extend the existing `SweepReport` schema (in `parser_corpus_sweep.rs`) to include `total_lines`, `median_errors_per_dirty_file`, and `recovery_salvage_rate`. The scorecard JSON's purpose is to be machine-readable input to gates and dashboards—but the sweep receipts already have a schema, are already read by `update_status`, and are already committed to the repo.

**Why it might be better:**
- **Fewer files, less schema drift**: The metrics README says "one scorecard is edited by one PR at a time" to avoid JSON merge conflicts. But the sweep receipts already exist and already follow a schema. Extending them rather than creating a parallel JSON file reduces the total number of metric artifacts.
- **No new wiring needed**: `update_status/parser.rs` already reads the sweep receipts. Adding new fields to `SweepReport` automatically makes them available to the status generator. Creating a separate scorecard JSON requires new code in `parser.rs` to write it.
- **Floors can live in the existing gate receipts**: The `just common-corpus-check` receipt is already a floor gate—the pass/fail is determined by whether all 10 pinned modules parse cleanly. If we want a floor on "unreadable count must be ≤ 48", that can be checked in the existing `corpus-sweep-check` gate, not in a new scorecard JSON.

**Why it might be worse:**
- The scorecard JSON is explicitly designed to be a single source of truth per subsystem, separate from the narrative status file. If we conflate the sweep receipt schema with the scorecard schema, we lose the clear separation between "corpus measurement" and "subsystem scorecard."
- The scorecard JSON format has `floor`/`improvement` split, `stability_window`, and `recorded_at`/`commit` provenance. The sweep receipts don't have this structure. Adding it would be retrofitting the wrong schema.

**What it sacrifices:**
- The clean separation between "what did the sweep measure?" (sweep receipt) and "what is the health of the parser subsystem?" (scorecard). The metrics README explicitly values this separation.

---

### Alternative 3: Question Whether "Error Density" Is the Right Metric at All

**Core idea:** The plan accepts "error density per 1k LOC" as a metric to add without questioning whether it answers a useful question. Propose instead using error-type distribution as the primary error quality metric, and removing the LOC-normalization entirely.

**Why it might be better:**
- **LOC is not a meaningful normalizer for parser quality**: A 500-line XS (C-to-Perl glue) file is a fundamentally different parsing challenge than a 500-line pure-Perl business logic file. They have different token distributions, different structural complexity, different error profiles. Dividing by LOC doesn't make them comparable.
- **Error density adds no information over error count when the denominator is fixed**: The Ubuntu corpus has a fixed set of files. If we know `total_error_nodes = 604` across `157 error files`, the median is `604/157 ≈ 3.85 errors per dirty file`. That's the metric we want—the LOC adds nothing unless we're comparing across corpora of different average file sizes.
- **Error bucket distribution is more actionable**: The baseline already has `first_error_buckets` (26 `unexpected_token_in_expr`, 10 `unclosed_brace`, etc.). A "top error type" metric that tracks which bucket is largest is directly useful for prioritizing parser improvements. "Error density per 1k LOC" tells you how bad the dirty files are; the bucket distribution tells you *why*.

**Why it might be worse:**
- Error bucket distribution is less intuitive than a single density number. A single number is easier to track on a dashboard. The bucket distribution requires more sophisticated visualization.
- Comparing error density across corpora (Ubuntu vs. CPAN) is genuinely useful—they have different file size distributions. LOC normalization enables that comparison.

**What it sacrifices:**
- The ability to compare error severity across corpora with different average file sizes. If CPAN files are systematically larger than Ubuntu system Perl files, error density per 1k LOC would reveal that CPAN's errors are "thinner" (same error count spread over more lines).

---

## Strongest Argument Against Current Approach

The plan builds a 30-fixture synthetic "malformed fixture bank" to measure recovery salvage, based on the premise that "recovery salvage has zero infrastructure." **This premise is wrong.** The `parser-corpus-baseline.json` already contains `files_by_bucket: BTreeMap<String, Vec<String>>` — a map of error type to the 157 real file paths that produced that error. These are authentic malformed Perl files with authentic error patterns, collected from real Ubuntu system Perl. The plan ignores this existing data and instead proposes building a synthetic fixture bank from scratch.

This is the most expensive way to be wrong: designing 30 fixtures requires arbitrary design decisions (which error types, how many per type, what constitutes "clean" vs. "broken" for each pair) that the plan itself identifies as risks ("medium-high" risk, "underdetermined"). Meanwhile, the plan already acknowledges using the error bucket distribution to guide fixture selection — which means it already knows which real files have which error types. The real files are more representative, already exist, and require zero maintenance.

The second strongest argument: **the scorecard JSON as designed changes zero merge-gate behavior**. Every metric is added as `improvement`, which "does not block." The existing `corpus-sweep-check`, `cpan-corpus-check`, and `common-corpus-check` gates remain the only things that block merges. The scorecard JSON becomes an informational artifact that duplicates what the sweep receipts already contain, without establishing any new floors. The plan's own risk analysis identifies this (#5: "Scorecard schema evolution conflicts with concurrent edits") but doesn't address the fundamental issue: if we're not adding floors, why is the scorecard JSON the delivery artifact?

---

## Recommended Action

**Modify, not replace.** The plan's approach to error density and scorecard wiring is largely correct. The fundamental flaw is in the recovery salvage approach (Alternative 1) and the scope of the scorecard JSON (Alternative 2 as critique).

Specific changes:

1. **Replace the malformed fixture bank with real-file salvage measurement.** Use the `files_by_bucket` data in the existing baseline JSONs to identify real dirty files. For each dirty file in the sweep, after parsing, run semantic analysis and measure whether LSP features produce usable output. This eliminates the fixture bank entirely and produces a more representative metric.

2. **Clarify what the scorecard JSON is for before building it.** The plan creates `.ci/metrics/parser.json` as a deliverable but doesn't establish any new floors in it. Either: (a) add at least one genuine floor metric in the scorecard JSON that wasn't a floor before (e.g., `unreadable_count` as a floor), or (b) treat the scorecard JSON as a Phase 2 refinement that only happens after the underlying metrics are validated in the sweep receipts.

3. **Reconsider error density normalization.** Either drop LOC normalization and use raw error counts (since the corpus is fixed), or validate that LOC is a meaningful normalizer for this corpus before adding it. The current plan assumes it without justification.

---

## Long-Term Cost Assessment

**If we build the malformed fixture bank as planned:**
- 6 months: The 30 fixtures need maintenance as Perl evolves. New error patterns aren't automatically added. The metric is frozen in time unless someone remembers to update the fixtures.
- 2 years: The fixture bank may be completely unrepresentative of real Perl errors if the corpus or Perl version changes. A metric on an unrepresentative fixture set is misleading—it gives false confidence in recovery quality.
- Long-term: Every future implementation agent working on parser error recovery has to update the fixture bank. This is a maintenance tax that compounds.

**If we use the scorecard JSON as currently scoped (all improvement, no floors):**
- 6 months: The scorecard JSON exists but blocks nothing. Developers learn to ignore it because it never stops their PRs. It becomes a documentation artifact rather than a governance artifact.
- 2 years: When someone finally tries to add a floor metric, they discover the stability window requires 5 consecutive runs, but the metric has been recorded inconsistently or with different denominators because the underlying schema was still changing. The ratchet can't engage because the metric wasn't stable from the start.
- Long-term: The scorecard model (floor/improvement split, stability-window ratchet) is undermined because the parser scorecard never actually used it. This makes the model look weak when the first real floor promotion is attempted.

**If we skip LOC normalization for error density:**
- 6 months: No visible cost. The metric still tracks error severity.
- 2 years: If the CPAN corpus grows files with systematically different sizes, comparing Ubuntu error density to CPAN error density becomes misleading. But if the comparison isn't being made, nothing breaks.
