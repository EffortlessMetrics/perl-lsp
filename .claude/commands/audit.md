---
description: Deep audit of a specific crate — API quality, tests, error handling, docs, panic safety
argument-hint: "<crate-name> e.g. 'perl-dap', 'perl-parser-core', 'perl-workspace-index'"
---

# Audit: Deep Crate Review

Perform a thorough audit of: **$ARGUMENTS**

## Process

Resolve the crate path: `crates/$ARGUMENTS/`. If the crate name has no `perl-` prefix, try `crates/perl-$ARGUMENTS/`.

Spawn an Explore agent to perform the audit:

```
Agent(
  subagent_type: "Explore",
  prompt: "
    Crate: $ARGUMENTS
    Path: crates/<resolved-crate>/

    Perform a deep audit of this crate across all dimensions below.
    Read every source file in src/ and every test file in tests/.

    ## 1. API Quality
    - Are public types and functions well-named?
    - Are there unnecessary pub items that should be pub(crate)?
    - Is the API surface minimal and coherent?
    - Are there builder patterns where constructors have too many args?

    ## 2. Test Coverage
    - What percentage of public functions have tests?
    - Are edge cases covered (empty input, max values, Unicode, errors)?
    - Are there integration tests in addition to unit tests?
    - List specific untested functions/paths.

    ## 3. Error Handling
    - Are errors descriptive and actionable?
    - Any unwrap/expect/panic in non-test code?
    - Are error types specific (not just String or anyhow)?
    - Is error propagation clean (using ? operator)?

    ## 4. Documentation
    - Do all public items have doc comments?
    - Are there module-level docs explaining purpose?
    - Are examples provided for non-obvious APIs?
    - Is there a crate-level README or lib.rs doc?

    ## 5. Panic Safety
    - Any panic!, todo!, unimplemented!, unreachable! in production code?
    - Any index operations that could panic (arr[i] vs arr.get(i))?
    - Any slice operations without bounds checking?
    - Any integer overflow possibilities?

    ## 6. Performance
    - Unnecessary allocations (String where &str suffices)?
    - Redundant clones?
    - O(n^2) or worse algorithms?
    - Missing capacity hints for Vec/HashMap?
    - Hot loops with allocations inside?

    ## 7. Dependencies
    - Are all dependencies actually used?
    - Are there lighter alternatives for heavy deps?
    - Are feature flags used efficiently?

    ## 8. Code Quality
    - Dead code or unreachable branches?
    - Overly complex functions (>50 lines)?
    - Magic numbers without constants?
    - Duplicated logic that could be extracted?

    For each finding, record:
    - Dimension (1-8 above)
    - Severity: critical / warning / info
    - File:line
    - Description
    - Suggested fix

    After the audit, invoke /scout-report to create a single comprehensive GitHub issue titled:
    'audit(<crate-name>): <N> findings across <M> dimensions'

    The issue body should be a structured report with sections for each dimension,
    sorted by severity (critical first).
  ",
  run_in_background: true,
  name: "audit-<crate-name>"
)
```

## Output

A single GitHub issue with:
- Title: `audit(<crate>): <N> findings across <M> dimensions`
- Label: `swarm-discovered`
- Body: structured findings by dimension, sorted by severity
- Each finding has file:line, description, and suggested fix

## Examples

```
/audit perl-dap                # Audit the DAP server
/audit perl-parser-core        # Audit parser core infrastructure
/audit perl-workspace-index    # Audit workspace indexing
/audit perl-lsp-diagnostics    # Audit LSP diagnostics provider
```

## When to Use

- Before a release: `/audit <crate>` for each crate in the release
- After major refactoring: verify the refactored crate is clean
- When picking up an unfamiliar crate: get oriented fast
- Pre-swarm: audit a crate to generate a queue of improvement slices
