---
description: Compare two approaches or implementations and write a trade-off analysis
argument-hint: "<A> vs <B> e.g. 'workspace index vs flat index' or 'Option A vs Option B from issue #1653'"
---

# Compare: Trade-Off Analysis

Compare: **$ARGUMENTS**

## Process

1. **Parse the comparison** from `$ARGUMENTS`. Expect one of:
   - Two approaches: `"<approach A> vs <approach B>"`
   - An issue reference: `"Option A vs Option B from issue #<N>"`
   - Two implementations: `"<crate-a>::<fn> vs <crate-b>::<fn>"`

2. **Gather context**:

   If an issue is referenced:
   ```bash
   gh issue view <N> --json title,body,comments
   ```

   If code is referenced, read the relevant source files.

3. **Spawn an Explore agent** to perform the comparison:

```
Agent(
  subagent_type: "Explore",
  prompt: "
    Compare: $ARGUMENTS

    ## Research Phase
    - Read all relevant source code for both approaches
    - Check if benchmarks exist (look in benches/ directories)
    - Look for prior art in the codebase (similar patterns already chosen)
    - Check git log for context on why current approach was chosen

    ## Analysis Dimensions

    For each approach, evaluate:

    ### Correctness
    - Does it handle all edge cases?
    - Are there known failure modes?

    ### Performance
    - Time complexity
    - Space complexity
    - Benchmark data if available

    ### Maintainability
    - Code complexity (LOC, cyclomatic complexity)
    - How many files/crates affected?
    - How easy to test?

    ### Compatibility
    - Does it break existing APIs?
    - SemVer implications?
    - Migration cost?

    ### Risk
    - What could go wrong?
    - How reversible is the choice?
    - Dependencies introduced or removed?

    ## Output

    Write the comparison as a structured analysis. If an issue was referenced,
    post the comparison as a comment on that issue:
    ```bash
    gh issue comment <N> --body '<comparison>'
    ```

    If no issue was referenced, invoke /scout-report to create a new issue titled:
    'compare: <A> vs <B>'
    with label 'swarm-architectural' (since comparisons usually need a design decision).

    The analysis MUST include:
    1. Summary table (dimension x approach)
    2. Recommendation with rationale
    3. Risks of the recommended approach
    4. Migration plan if switching from current to recommended
  ",
  run_in_background: true,
  name: "compare-<short-label>"
)
```

## Output Format

### Summary Table
| Dimension | Approach A | Approach B |
|-----------|-----------|-----------|
| Correctness | ... | ... |
| Performance | ... | ... |
| Maintainability | ... | ... |
| Compatibility | ... | ... |
| Risk | ... | ... |

### Recommendation
One paragraph: which approach and why.

### Risks
What could go wrong with the recommendation.

### Migration Plan
Steps to adopt the recommendation (if it differs from current).

## Examples

```
/compare "workspace index vs flat index"
/compare "Option A vs Option B from issue #1653"
/compare "perl-lexer::tokenize vs perl-tokenizer::scan"
/compare "pest parser vs recursive descent for heredocs"
/compare "async LSP vs sync LSP with thread pool"
```

## When to Use

- Design decision needed: two viable approaches, unclear winner
- Refactoring evaluation: is the new way actually better?
- Issue triage: someone proposed two options, need analysis
- Architecture review: compare current design against alternative
