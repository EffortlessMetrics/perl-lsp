# DAP Implementation Plan: Phase 2 & 3 (Native Adapter)

**Status**: Planning / In Progress
**Context**: Moving from Phase 1 (Bridge Adapter) to Phase 2 (Native Rust Implementation) and Phase 3 (Hardening).
**Based on**: GitHub Issues #353-#362 and #449-#457.

---

## Phase 2: Native Adapter Completion

The goal of Phase 2 is to replace the Perl::LanguageServer bridge with a native Rust implementation of the Debug Adapter Protocol. This involves direct protocol handling and integration with the Perl debugger (`perl -d`).

### 1. Session & Protocol Management
- [ ] **#449 / #353: DAP Session Management (AC5)**
  - Implement `initialize`, `launch`, `attach`, `disconnect` lifecycles.
  - Handle capabilities negotiation.

### 2. Breakpoints & Execution Control
- [ ] **#450 / #354: DAP AST-Based Breakpoint Validation (AC7)**
  - Use `perl-parser` to validate breakpoint locations (verify they are on executable statements).
  - Resolve breakpoint locations to valid lines.
- [ ] **#454 / #356: DAP Control Flow Handlers (AC9)**
  - Implement `continue`, `next`, `stepIn`, `stepOut`.
  - Handle `pause` (interrupt).

### 3. State Inspection
- [ ] **#453: DAP Stack Trace Provider (AC8)**
  - Parse stack frames from debugger output.
  - Map frames to source files.
- [ ] **#452 / #355: DAP Variable Renderer with Lazy Expansion (AC8/AC10)**
  - Implement `variables` request.
  - Support scopes (Locals, Globals).
  - Lazy loading for large structures.
- [ ] **#455 / #357: DAP Safe Evaluation (AC10)**
  - Implement `evaluate` request (repl/watch).
  - Ensure evaluation is side-effect aware (where possible) or sandboxed.

---

## Phase 3: Production Hardening & Security

The goal of Phase 3 is to ensure the DAP implementation is secure, robust, and performant enough for enterprise use.

### 1. Security
- [ ] **#457: DAP Security Validation (AC16)**
  - Validate all file paths (path traversal protection).
  - Sanitize input arguments.
  - Ensure `evaluate` cannot be used for injection attacks beyond the debugged process scope.
- [ ] **#358: Phase 3 Security & Hardening (AC13-AC19)**
  - Comprehensive security review.

### 2. Testing & Infrastructure
- [ ] **#435: DAP Non-Regression Test Suite (Phase 3)**
  - End-to-end tests for debug sessions.
  - Regression tests for known bugs.
- [ ] **#420: DAP Bridge Test Infrastructure (Phase 1)**
  - (Likely completed or foundational for Phase 2).

---

## Tracking

See `GITHUB_ISSUES_SUMMARY.md` for the latest status of individual issues.
This plan aggregates the work items defined in the issue tracker.
