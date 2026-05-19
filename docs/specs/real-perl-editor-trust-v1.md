# Real Perl Editor Trust v1 Boundary Spec

Status: Draft (normative)

## Purpose

This specification freezes the Real Perl Editor Trust v1 release boundary.

It defines what is live, what is preview-only, what is explanation-only, and what is explicitly blocked for v1 so support claims, CI checks, and provider behavior remain aligned.

## Core invariant

A fact can help the user only inside its proof boundary.

Corollaries for v1:

- Source-backed facts may drive exact behavior only when freshness/confidence and surface-specific guards pass.
- Generated facts must be labeled and must not silently claim exact source authority.
- Dynamic boundaries must block unsafe edit-producing behavior.
- Ambient inputs must be surfaced in user-visible trust/determinism reporting.
- Stale or low-confidence facts must not authorize edits.

## v1 surface classification

### Live

- completion: partial-live
- hover: provenance-backed
- definition/references: exact/imported partial-live
- diagnostics: partial-live with explanation payloads
- document symbols: source-backed
- workspace symbols: source-backed with generated-label pilot
- semantic tokens: source-backed trace slices
- rename: lexical and package-local pilot
- safe-delete: source-backed pilot

### Preview-only

- package rename preview
- safe-delete preview

### Explanation-only

- explain-provider-decision
- explain-diagnostic
- explain-missing-module lookup
- workspace trust report

### Blocked (v1)

- broad generated symbols
- broad compiler-token promotion
- broad package rename
- broad safe-delete
- generated/no-source edits
- dynamic edits
- stale/low-confidence edit authorization

## Decision model requirement

Every provider path in scope for v1 must resolve to one of the following outcomes:

- promote
- fallback
- block
- defer

Promotion, fallback, and blocker behavior must be bounded by explicit conditions; no implicit or silent promotion is permitted.

## User-facing requirements

For user-visible explanations and receipts in v1:

- decision state must be explicit (promoted, fallback, blocked, preview, explanation-only, or deferred)
- fact source/provenance class must be explicit
- confidence and freshness state must be explicit
- source-backed range state must be explicit when relevant to the surface
- fallback reason or blocker reason must be explicit when not promoted
- user guidance must not over-claim capability outside the decision boundary

## Non-goals for v1

This spec does not itself define schema fields, Rust type shapes, or CI implementation details for individual surfaces. Those belong to follow-on contracts (provider decision schema, provenance semantics, edit authorization, class ledgers, and determinism receipts).

## Change policy

Any proposed expansion from blocked/preview/explanation-only into live must include:

1. a machine-checkable policy or schema update,
2. fixture coverage proving promote/fallback/block behavior,
3. a support-claim update that remains consistent with this boundary.
