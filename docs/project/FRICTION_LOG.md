# Friction Log

Tracks pain points encountered by agents and developers: confusing errors, hard-to-find code, unclear APIs, missing setup steps. Used to prioritize devex improvements.

Format: date, who hit it, what happened, suggested fix.

---

## 2026-03-15 — improver-docs / readme audit

**Category**: docs
**Who**: improver-docs agent during README audit
**What**: README.md contained a duplicate `## Parser Coverage` section (lines 83 and 123) and a duplicate `## History` section (lines 208 and 219). The second Parser Coverage block had different (complementary) content from the first. Both were added during separate PRs without a cross-check.
**Suggested fix**: The canonical check for README is `grep -n "^## " README.md` — any heading appearing twice is a duplicate. Fixed in PR docs(changelog+readme): deduplicate sections.

---

## 2026-03-15 — improver-docs / changelog audit

**Category**: docs
**Who**: improver-docs agent during CHANGELOG audit
**What**: After 20+ PRs merged since 0.11.0 (PRs #1521–#1555), the `[Unreleased]` section in CHANGELOG.md was empty. Users and release tooling (git-cliff) would not surface these changes until a release PR was opened.
**Suggested fix**: Run `git log --oneline <last-release-tag>..HEAD` after each merge wave and append summaries to `[Unreleased]`. Consider a CI check that fails if `[Unreleased]` is empty and commits have landed since the last release tag.

---

## 2026-03-16 — async migration / PR #1555

**Category**: test-debt
**Who**: Builder working on async runtime migration (PR #1555)
**What**: After migrating `LspServer` methods from `&mut self` to `&self`, approximately 150 test call sites still passed `&mut LspServer`. These generate clippy warnings (not errors) and do not break tests but create noise in CI output and mislead contributors about the actual signature.
**Files**: ~26 test files in `crates/perl-lsp-rs/tests/`
**Suggested fix**: A single-pass `sed` or automated refactor across the test directory. Tracked as Task #6 (Async test helper cleanup). See ADR-0031 for context.

---

## 2026-03-16 — async migration / PR #1555

**Category**: architecture
**Who**: Reviewer of PR #1555
**What**: `unsafe impl Send for LspServer` and `unsafe impl Sync for LspServer` are required because `ParentMap` holds raw pointers into AST nodes. The compiler cannot verify safety. The safety invariant (pointers are only accessed while `Arc<Mutex>` lock is held; pointed-to AST is not dropped while pointers live) is documented in the code but not in any ADR or architecture doc. Future contributors modifying `LspServer` field layout may not discover this constraint.
**Files**: `crates/perl-lsp-rs/src/server.rs` (approximately)
**Suggested fix**: ADR-0031 now documents this. Long-term: replace raw pointers with index-based references in `ParentMap` to eliminate the unsafe entirely.

---

## Template

```
## YYYY-MM-DD — <agent or developer> / <branch or context>

**Category**: docs | test-debt | devex | build | architecture | security
**Who**: <who encountered it>
**What**: <what happened — be specific enough that someone else can reproduce>
**Files**: <relevant paths with line numbers if known>
**Suggested fix**: <actionable recommendation>
```
