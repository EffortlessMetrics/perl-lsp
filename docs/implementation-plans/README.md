# Perl-LSP Implementation Planning Documents

This directory contains comprehensive planning documents for the Perl-LSP project, derived from systematic analysis of 113 open issues. These documents are intended for implementation teams working on various components of the language server.

**Total Research Volume:** 243KB of analysis, specifications, and planning

---

## Document Index

### 1. Architecture RFC
**File:** `perl-lsp-architecture-rfc.md`

Three unified architectural proposals for core infrastructure improvements:
- **Parser:** Add `deref_base` annotation for reference-dereference operations
- **EffectiveSemantics Layer:** Unified version/pragma/feature tracking (replaces `version_compat`, `implicit_global_registrar`, `pragma_registrar`)
- **BUILTIN_INITIALIZERS PHF Registry:** Systematic handling of position-aware builtin initializers

**Key Issues Addressed:** #3338, #3344, #3345, #3346, #3347, #3348, #3349, #3350, #3353, #3355, #3363, #3364, #3365, #3375

---

### 2. Prioritized Implementation Plan
**File:** `perl-lsp-prioritized-plan.md`

The master implementation roadmap organized into 6 phases (~46-60 days total):
- **Phase 0:** Foundation (EffectiveSemantics layer, BUILTIN_INITIALIZERS)
- **Phase 1:** Sigil/Dereference (fix #3338 and related)
- **Phase 2:** Scope/Symbol (signatures, packages, try/catch)
- **Phase 3:** Builtins/Special Vars (regex vars, UNIVERSAL methods)
- **Phase 4:** Pragmas/Features (version→feature mapping, mro)
- **Phase 5:** Method Resolution (AUTOLOAD, mro pragma)

Contains detailed Top 10 by Impact/Effort ratio and full issue registry organized by severity (P0/P1/P2).

**Key Issues Addressed:** All 103 analyzed issues, with full cross-referencing

---

### 3. EffectiveSemantics Detailed Specification
**File:** `perl-lsp-effective-semantics-spec.md`

Complete technical specification for the unified semantic tracking layer:
- Motivation and design principles
- Core data structures (`EffectiveState`, `EffectiveSnapshot`)
- API design with code examples
- Integration with existing `version_compat` and `implicit_global_registrar`
- Migration strategy (parallel → unified)

**Key Issues Addressed:** #3344, #3348, #3349, #3350, #3353, #3375

---

### 4. BUILTIN_INITIALIZERS Detailed Specification
**File:** `perl-lsp-builtin-initializers-spec.md`

Complete technical specification for the PHF-based builtin initializer registry:
- Design goals (position-aware declarations)
- PHF schema and implementation
- 63 builtin functions with position-aware capability
- Integration with existing `implicit_global_registrar`
- Test coverage requirements

**Key Issues Addressed:** #3345, #3346, #3347

---

### 5. Test Coverage Plan
**File:** `perl-lsp-test-plan.md`

Comprehensive test strategy for all planned changes:
- 76 regression tests mapped to specific issues
- Unit test structure for each architectural component
- Integration test scenarios
- Snapshot test specifications for BUILTIN_INITIALIZERS
- Test data and fixtures for regex, encoding, and OO scenarios

**Coverage Areas:** Builtins, EffectiveSemantics, sigil bridging, signatures, packages, try/catch, phasers, AUTOLOAD, mro, encoding

---

### 6. Release Strategy
**File:** `perl-lsp-release-plan.md`

Version planning and release coordination:
- **v0.12.4:** Hotfix release with P0 scope fixes (#3338, #3361, #3356)
- **v0.13.0-alpha.1:** Foundation release (EffectiveSemantics, BUILTIN_INITIALIZERS)
- **v0.13.0-alpha.2-4:** Progressive phase releases
- **v0.13.0:** Final release with all phases

Includes detailed milestone definitions, quality gates, and rollback procedures.

---

### 7. PR Followup Checklist
**File:** `perl-lsp-pr-followup.md`

Post-merge coordination document for implementation teams:
- Issue → Document mapping (which planning doc applies to which issue)
- Cross-issue dependencies and blocking relationships
- Team coordination notes for parallel work
- Testing verification checklist
- Documentation update requirements

**Contains:** 5-column matrix linking all 103 issues to their implementation documents

---

## Recommended Reading Order

### For New Implementation Team Members
1. **Architecture RFC** — Understand the three core architectural directions
2. **Prioritized Implementation Plan** — Get the full roadmap and issue context
3. **Release Strategy** — Understand version boundaries and coordination

### For Engineers Starting Implementation
1. **Check PR Followup** — See which issues are assigned to your component
2. **Read relevant Detailed Spec:**
   - EffectiveSemantics work → `perl-lsp-effective-semantics-spec.md`
   - Builtin/initializer work → `perl-lsp-builtin-initializers-spec.md`
3. **Reference Test Plan** — Understand test expectations for your work

### For Project Managers / Release Coordinators
1. **Release Strategy** — Version timeline and quality gates
2. **Prioritized Implementation Plan** — Phased approach and effort estimates
3. **PR Followup** — Cross-team dependency tracking

### For Test Engineers
1. **Test Coverage Plan** — Full test matrix
2. **Relevant Detailed Spec** — Understand expected behavior per component
3. **Prioritized Implementation Plan** — Issue severity and release phasing

---

## Key Issue Groups by Component

### Scope Analysis
- #3338, #3363, #3364, #3365 (sigil bridging)
- #3361 (signatures)
- #3356 (packages)
- #3378 (try/catch)

### Builtins / Special Variables
- #3351, #3354 (regex variables)
- #3345, #3346, #3347 (BUILTIN_INITIALIZERS)

### Version / Feature Mapping
- #3344 (given/when)
- #3348, #3349, #3350 (version→feature mappings)

### Method Resolution / OO
- #3383 (UNIVERSAL methods)
- #3384, #3385 (mro pragma)
- #3386 (AUTOLOAD)

---

## Document Status

- [x] Architecture RFC — Complete
- [x] Prioritized Plan — Complete
- [x] EffectiveSemantics Spec — Complete
- [x] BUILTIN_INITIALIZERS Spec — Complete
- [x] Test Plan — Complete
- [x] Release Strategy — Complete
- [x] PR Followup — Complete

**Last Updated:** 2026-04-09  
**Generated From:** Systematic analysis of 113 open issues
