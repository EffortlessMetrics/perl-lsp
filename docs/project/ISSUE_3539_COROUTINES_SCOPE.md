# Issue #3539 Coroutine Scope Decision

## Summary

`#3539` should not be implemented as core-language parser support in its current form.

Current project direction:

1. **Defer core syntax work** (`coro sub`, `yield`) until upstream Perl publishes a stable, documented syntax contract.
2. **Split delivery into two tracks**:
   - upstream-tracking issue (core status + syntax contract)
   - user-value issue for CPAN coroutine ecosystems (for example, `Coro` hover/completion support)

## Why This Is Deferred

The current issue draft conflates three different scopes:

- hypothetical core syntax (`coro sub`, `yield`)
- version/status claims for core Perl experimental features
- CPAN library APIs (`Coro`/`async` style workflows)

These require different implementation paths and should not ship under one mixed issue.

## Scope Split

### Track A: Upstream Core Status (no parser changes)

Question to resolve:

> Is there a released and documented core Perl coroutine syntax target that perl-lsp should parse?

Until this answer is yes, parser/AST semantics for core coroutine syntax remain deferred.

### Track B: CPAN Ecosystem Support (user value now)

Potential scope (no grammar changes):

- hover/completion support for known coroutine APIs from CPAN packages (for example, `Coro`)
- method completion/hover for known coroutine object methods where inference is reliable
- tests proving no parser regressions and no new grammar surface

## Implementation Guidance

If follow-up work is scheduled now, prioritize **Track B** and keep parser grammar unchanged.

If parser groundwork is desired, keep it infrastructure-only (extensibility hooks, feature gating plumbing), without introducing unshipped syntax behavior.

## Decision Checklist for Reopening Core Syntax Work

Before any core coroutine parser implementation starts, capture all of the following in the issue:

- authoritative upstream syntax documentation link(s)
- explicit version/feature gate and warning behavior
- syntax examples that define parse shape and error recovery expectations
- compatibility notes for existing identifiers named `yield` or `coro`

Without this checklist, core syntax implementation remains deferred.
