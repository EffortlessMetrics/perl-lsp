---
tags: [ci, ripr, gate-design, suppressions, static-analysis, weakly-exposed, verify-the-instrument, control-plane]
repos: [perl-lsp-swarm]
related: ["#3355", "#3363", "#3397", "#2015", "EffortlessMetrics/ripr#1429"]
portable: true
article_asset: true
search_terms: [weakly_exposed suppression, ripr#1429, predicate_infection_untraceable, activation_unknown, genuine_new_ripr_gap_count, ripr timed out 210s, tracer mis-association, new gap gate advisory, str::split closure predicate, discriminator test already exists]
---

# When a gate blocks on its own analyzer's limitations, fix the gate — not by adding suppressions forever

**Date**: 2026-07-04
**Hazard class**: Control-plane / gate-instrument reliability
**Portable lesson**: [docs/concepts/verify-the-instrument.md](../concepts/verify-the-instrument.md)

## What happened

The `ripr+ New Gap Gate` blocks a PR when RIPR reports new "severe" static-exposure
gaps. RIPR 0.9.x classifies some seams as `weakly_exposed` /
`activation_unknown` — meaning its static tracer *cannot connect* a test to a
predicate, even when a direct unit test with the exact boundary value already
exists. Two PRs in one day hit this:

- **#3355** (Test2::V1): flagged the `str::split(|c| ...)` closure char-class
  predicates at `test2.rs:544` as uncovered, though `v1_short_flag_predicate_boundary_discriminators`
  calls the function with exactly those inputs. The tracer can't follow oracles
  through a split closure (the known `EffortlessMetrics/ripr#1429` class).
- **#3397** (`PERL_LSP_TIMING`): flagged `trimmed == "0"` / `trimmed == "1"` at
  `timing.rs:108/:113` as *"no related test uses '0'/'1'"* — against a branch
  where `parse_mode_off_variants` / `parse_mode_stderr_variants` call the pure
  `parse_mode` with exactly `"0"` and `"1"`. The **same run's guidance pass also
  reported `tool_error: ripr timed out after 210s`** and emitted zero
  annotations — a degraded *instrument*, and the mis-association **reproduced on
  a clean re-run**.

Each was worked around with a narrow `[[suppress]]` in
`policy/ripr-suppressions.toml` (`kind = predicate_infection_untraceable`,
single file, cites ripr#1429, **names the covering tests**, expiring). By
mid-session the file had accumulated 3+ entries for the *same class*.

## The signal

Adding the Nth suppression for the identical failure class is a smell: the gate
is blocking on the analyzer's confidence limit, not on a real coverage gap.
Tests-first is still correct (exhaust a direct discriminator test before
suppressing), but once the discriminator exists and RIPR still can't see it,
**more tests won't clear it** — the gate itself is miscalibrated.

## The systemic fix (#3363, closes #2015)

`genuine_new_ripr_gap_count()` was added so the gate's *blocking* count is
`min(reachable_unrevealed + no_static_path, severe_gaps)` — i.e. it blocks only
on **actionable** exposure classes and treats `weakly_exposed` (an
analyzer-confidence limitation) as **advisory, still reported but not blocking**.
Producer suppressions still subtract from `severe_gaps` (the `min` cap honors
them). Visibility is preserved; the false-block class is retired at the source.

After #3363, the earlier expiring suppressions become redundant and can be pruned.

## Portable rule

- Distinguish **"the code is unproven"** (reachable_unrevealed / no_static_path →
  add a test or fix the seam) from **"the analyzer can't trace a proven test"**
  (weakly_exposed / activation_unknown → not a coverage gap).
- A gate must not hard-block on its analyzer's *confidence limit*. If it does,
  fix the gate's classification (advisory-not-blocking for the limited class),
  don't paper over each instance.
- Watch the instrument: a `tool_error`/timeout in the analyzer's own run
  (`ripr timed out after 210s`, zero annotations) means the evidence is degraded
  — treat a "gap" from a degraded run as unproven, not as truth. See
  [gate-names-must-match-failure-classes](../concepts/gate-names-must-match-failure-classes.md)
  and [verify-the-instrument](../concepts/verify-the-instrument.md).
