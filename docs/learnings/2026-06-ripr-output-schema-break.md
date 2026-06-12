---
tags: [coverage-integrity, ci, tooling, ripr, version-bump]
repos: [perl-lsp-swarm]
related: ["#1289", "#1329", "#1335", "#1336"]
portable: false
article_asset: true
search_terms: [ripr_evidence.rs, grip_class, seam.file, weakly_gripped, suppressed_by_policy, RIPR_VERSION, ripr_pr_summary_counts, ripr_finding_path, classification, probe.file, weakly_exposed]
---

# ripr 0.9.x output-schema rename silently broke suppression matching

**Date**: 2026-06
**Hazard class**: coverage-integrity
**Portable lesson**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) (Class 6)

## What happened

ripr 0.9.x (bumped in #1329) renamed per-finding JSON fields compared to 0.5.x:
 ->  (values  -> ) and
 -> . The xtask gate evidence parser in
 only knew the 0.5.x field names. Under 0.9.x all
findings were silently skipped during suppression matching:  stayed
0 and path-based suppressions in  never fired, producing
false-positive  failures on PRs whose gaps were covered by existing
policy entries. The gate was over-strict, not neutered (gross counts still came from
ripr's own summary section, which 0.9.x computes correctly).

## Why

The version bump PR (#1329) merged without diffing the tool's output schema between
versions. The schema break was silent: no parse error, no test failure at bump time,
just silently-empty suppression matches that only manifested when a subsequent PR
triggered the gate with a suppression in place.

## Fix

PR #1336:  now reads both  (0.5.x) and
 (0.9.x) via .  tries 
(0.5.x) then  (0.9.x). Two unit tests:
 (suppression fires) and
 (gate retains teeth).

## Spec impact

Added to  gate-level checklist: "Before bumping
any external tool version, diff the tool's OUTPUT SCHEMA (JSON/text emitted) between
old and new versions. A schema change in the tool's output requires code changes in
every consumer." Also motivates Class 6 in .

## Portable lesson

This is a concrete instance of the coverage/measurement integrity hazard class: a
transformation layer (suppression matching) silently stopped working because the
upstream data format changed without the consumer being updated. The same pattern
occurs whenever a tool's JSON output schema is treated as stable without a contract.

- **Pattern**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md)
- **Class**: Class 6 -- Coverage/Measurement Integrity
- **Generalization**: Diff external tool output schemas before bumping; silent empty results are a distinct failure mode from parse errors.

## Related PRs

- [#1329](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1329) -- bumped RIPR_VERSION 0.5.0 -> 0.9.0
- [#1335](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1335) -- issue: identified the schema break
- [#1336](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1336) -- fix: handle both field-name versions in ripr_evidence.rs
- [#1289](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1289) -- original version-bump tracking issue
