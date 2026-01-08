# Docs Auditor Prompt

## Purpose

The Docs Auditor analyzer assesses **Reproducibility** of a PR. It evaluates gate clarity, executable snippet accuracy, documentation drift risk, and whether a third party could verify the work with available information.

**Quality Surface**: Reproducibility

## Required Inputs

Provide the following context to the analyzer:

### 1. Documentation Files in Diff
```
<docs_diff>
[Git diff of documentation files: *.md, README, CLAUDE.md, etc.]
</docs_diff>
```

### 2. Gate Commands
```
<gate_commands>
[Commands mentioned in docs or CI for verification]
[e.g., from CLAUDE.md, justfile, CI workflows]
</gate_commands>
```

### 3. Code Snippets in Docs
```
<code_snippets>
[Code blocks in documentation that claim to be executable]
</code_snippets>
```

### 4. CI/Gate Configuration
```
<ci_config>
[.github/workflows/*.yml, justfile, or similar gate definitions]
</ci_config>
```

### 5. Receipt Availability
```
<receipts>
[Links to CI runs, local output logs, test receipts if available]
</receipts>
```

### 6. Known Limits Section (if exists)
```
<known_limits>
[Known limitations or caveats documented in PR or docs]
</known_limits>
```

## Output Schema

The analyzer must produce output conforming to this YAML schema:

```yaml
analyzer: docs-auditor
pr: <number>
timestamp: <ISO8601>
coverage: <github_only|github_plus_agent_logs|receipts_included>

gate_clarity:
  single_command: <yes|no>
  command: <the canonical gate command, e.g., "nix develop -c just ci-gate">
  documented_in: <file path>
  verification_steps: <count of steps needed if not single command>
  prerequisites: [<list of setup requirements>]

executable_snippets:
  total: <count of code blocks in docs>
  verified: <count known to work>
  unverified: <count not tested>
  broken:
    - location: <file:line>
      snippet: <the code block>
      issue: <what's wrong>

drift_risk:
  - doc: <documentation file>
    code: <code file it documents>
    risk: <high|medium|low>
    reason: <why drift is likely>
    last_sync: <commit or date if known, else "unknown">

receipt_availability:
  ci_runs:
    - url: <link>
      status: <pass|fail>
      relevant_to: <what it proves>
  local_output: <yes|no|partial>
  agent_logs: <yes|no>
  coverage_level: <receipts_included|github_plus_agent_logs|github_only>

known_limits:
  declared: <yes|no>
  location: <file path or "not found">
  limits:
    - limit: <description>
      documented: <yes|no>
      impact: <high|medium|low>

environment_reproducibility:
  pinned_deps: <yes|no|partial>
  nix_flake: <yes|no>
  lockfile_committed: <yes|no>
  "works_on_my_machine" _risk: <high|medium|low>

findings:
  - id: <unique_id, e.g., "DOC-001">
    severity: <P1|P2|P3|info>
    category: <gate_unclear|broken_snippet|drift_risk|missing_receipt|undeclared_limit|env_fragile>
    summary: <one line>
    evidence:
      - anchor: <file:line or link>
        content: <excerpt>
    recommendation: <action>
    confidence: <high|medium|low>

summary:
  verdict: <pass|warn|fail>
  key_findings:
    - <bullet 1>
    - <bullet 2>
  reproducibility_delta: <+2|+1|0|-1|-2>

assumptions:
  - <what was assumed>
```

## Key Questions Answered

1. **Can someone verify with one command?** - Is there a single canonical gate?
2. **Are code snippets executable?** - Do documented commands actually work?
3. **Are docs and code in sync?** - What's the drift risk?
4. **Are receipts available?** - Can claims be verified from artifacts?
5. **Are limits explicit?** - Are caveats and known issues documented?

## Gate Clarity Assessment

### Good Gate Clarity
- Single command documented prominently
- Prerequisites clearly listed
- Works on fresh checkout
- Output is interpretable (pass/fail obvious)

### Poor Gate Clarity
- Multiple manual steps required
- Implicit prerequisites
- Order-dependent commands
- Requires tribal knowledge to interpret

## Drift Risk Classification

| Risk Level | Criteria |
|------------|----------|
| **High** | Doc describes API that changed; doc not updated in same PR |
| **Medium** | Doc describes behavior that may have changed; not verified |
| **Low** | Doc is structural (README sections) or recently updated |

## Snippet Verification

For each code block in documentation:
1. Is it marked as executable (shell, bash, etc.)?
2. Does the command exist in the repo?
3. Are the paths valid?
4. Are dependencies available?

## Example Input

```
<pr_metadata>
PR Number: 260
Title: Status check documentation
Stated Scope: Document the status-check gate
</pr_metadata>

<docs_diff>
diff --git a/docs/CURRENT_STATUS.md b/docs/CURRENT_STATUS.md
+ ## Verification
+
+ ```bash
+ just status-check
+ ```
+
+ This command verifies that computed metrics match committed values.
</docs_diff>

<gate_commands>
From CLAUDE.md:
  nix develop -c just ci-gate

From justfile:
  status-check:
    @bash scripts/status-check.sh
</gate_commands>

<receipts>
CI Run: https://github.com/example/repo/actions/runs/12345 (pass)
</receipts>
```

## Example Output

```yaml
analyzer: docs-auditor
pr: 260
timestamp: 2025-01-07T12:00:00Z
coverage: github_plus_agent_logs

gate_clarity:
  single_command: yes
  command: "nix develop -c just ci-gate"
  documented_in: CLAUDE.md
  verification_steps: 1
  prerequisites:
    - "Nix with flakes enabled"
    - "just command runner"

executable_snippets:
  total: 1
  verified: 1
  unverified: 0
  broken: []

drift_risk:
  - doc: docs/CURRENT_STATUS.md
    code: scripts/status-check.sh
    risk: low
    reason: "Documentation added in same PR as feature"
    last_sync: "PR #260"

receipt_availability:
  ci_runs:
    - url: "https://github.com/example/repo/actions/runs/12345"
      status: pass
      relevant_to: "Proves status-check passes on CI"
  local_output: no
  agent_logs: no
  coverage_level: github_plus_agent_logs

known_limits:
  declared: no
  location: not found
  limits:
    - limit: "status-check requires metrics to be pre-computed"
      documented: no
      impact: medium

environment_reproducibility:
  pinned_deps: yes
  nix_flake: yes
  lockfile_committed: yes
  works_on_my_machine_risk: low

findings:
  - id: DOC-001
    severity: P3
    category: undeclared_limit
    summary: Status-check prerequisite not documented
    evidence:
      - anchor: docs/CURRENT_STATUS.md:5-8
        content: "just status-check - doesn't mention metrics must be computed first"
    recommendation: Add note that `just health` must run before status-check
    confidence: medium

summary:
  verdict: pass
  key_findings:
    - Single canonical gate command documented
    - New snippet is executable and verified via CI
    - Minor gap in prerequisite documentation
  reproducibility_delta: +1

assumptions:
  - CI run tests the documented command path
  - Nix flake ensures reproducible environment
```

## Trust Model

### Can Be Inferred (High Confidence)
- Presence of gate command in docs
- Snippet syntax and structure
- File paths referenced in docs
- Lockfile and Nix flake presence

### Can Be Inferred (Medium Confidence)
- Whether snippets are executable (syntax check only)
- Drift risk based on file modification dates
- Known limits from PR description or doc content

### Cannot Be Inferred
- Whether snippets actually work (requires execution)
- Actual drift between docs and behavior (requires testing)
- Whether receipts prove what they claim (requires audit)
- User-facing reproducibility issues (requires fresh checkout test)

### Red Flags to Note
- No single canonical gate command
- Code snippets with placeholder values (YOUR_VALUE, etc.)
- Documentation of features that changed without doc update
- CI passing but local gate steps unclear
- Known issues mentioned in PR but not in docs

## Integration Notes

Docs Auditor uses:
- **Diff Scout output**: To identify docs vs code file categories
- **CI configuration**: To verify gate commands

Docs Auditor feeds into:
- **Dossier synthesis**: Reproducibility delta for cover sheet
- **Policy Auditor**: Receipt linkage verification

For full verification, execute snippets locally and provide output as receipts.
