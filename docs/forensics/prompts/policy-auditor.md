# Policy Auditor Prompt

## Purpose

The Policy Auditor analyzer assesses **Governance Integrity** of a PR. It evaluates whether truth surfaces remained honest - did feature catalog updates track capability changes, did computed metrics stay in sync, did schemas remain compliant, and did anti-drift guardrails catch violations?

**Quality Surface**: Governance

## Required Inputs

Provide the following context to the analyzer:

### 1. features.toml Diff
```
<features_diff>
[Git diff of features.toml if changed, or "no changes"]
</features_diff>
```

### 2. CURRENT_STATUS.md Diff
```
<status_diff>
[Git diff of docs/CURRENT_STATUS.md if changed, or "no changes"]
</status_diff>
```

### 3. Status Check Output
```
<status_check_output>
[Output from `just status-check` or `python3 scripts/update-current-status.py --check`]
[If available: both before and after the PR]
</status_check_output>
```

### 4. Schema Validation Results
```
<schema_validation>
[Validation results for dossier and receipt formats]
[If available: JSON schema checks, YAML lint output]
</schema_validation>
```

### 5. Code Changes That Affect Capabilities
```
<capability_changes>
[Code changes that add/modify/remove LSP features or capabilities]
[From crates/perl-parser/src/lsp/, crates/perl-lsp/src/]
</capability_changes>
```

### 6. Doc Drift Markers
```
<drift_markers>
[Any TODO/FIXME/NOTE markers added or removed]
[Changes to IGNORED_TESTS_ROADMAP.md or ignored test baseline]
</drift_markers>
```

## Output Schema

The analyzer must produce output conforming to this YAML schema:

```yaml
analyzer: policy-auditor
pr: <number>
timestamp: <ISO8601>
coverage: <github_only|github_plus_agent_logs|receipts_included>

catalog_drift:
  detected: <true|false>
  violations:
    - capability: <feature id from features.toml>
      change_type: <added|modified|removed|maturity_change>
      code_evidence: <file:line where capability changed>
      catalog_updated: <yes|no|partial>
      severity: <P1|P2|P3>
      notes: <explanation>
  corrections:
    - capability: <feature id>
      before: <previous state>
      after: <corrected state>
      evidence: <commit or diff line>

metrics_drift:
  detected: <true|false>
  violations:
    - metric: <metric name from CURRENT_STATUS.md>
      claimed_value: <what CURRENT_STATUS.md says>
      actual_value: <what evidence shows>
      drift_magnitude: <percentage or delta>
      severity: <P1|P2|P3>
      evidence: <file:line or output>
  auto_correction_available: <yes|no>
  correction_command: <command to fix, e.g., "just status-update">

schema_compliance:
  dossier:
    status: <pass|warn|fail>
    issues:
      - location: <dossier file and section>
        violation: <what schema rule was violated>
        severity: <P1|P2|P3>
  receipts:
    status: <pass|warn|fail>
    issues:
      - location: <receipt file or PR comment>
        violation: <what schema rule was violated>
        severity: <P1|P2|P3>

guardrail_effectiveness:
  status: <pass|warn|fail>
  status_check:
    ran: <yes|no>
    caught_drift: <yes|no|N/A>
    false_positives: <count>
  ignored_test_tracking:
    baseline_updated: <yes|no|N/A>
    delta: <change in ignored test count>
    properly_justified: <yes|no|partial>
  ci_gate:
    enforced: <yes|no>
    would_have_caught: <yes|no|N/A>
    notes: <explanation>

claim_provenance:
  new_claims:
    - claim: <statement from PR description or commits>
      location: <where claim appears>
      evidence_provided: <yes|partial|no>
      evidence_anchor: <link to test output, CI run, etc.>
      confidence: <high|medium|low>
  claims_without_evidence:
    - claim: <statement>
      location: <where claim appears>
      severity: <P1|P2|P3>
      recommendation: <how to anchor it>

findings:
  - id: <unique_id, e.g., "POL-001">
    severity: <P1|P2|P3|info>
    category: <catalog_drift|metrics_drift|schema_violation|guardrail_gap|unanchored_claim>
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
  governance_delta: <+2|+1|0|-1|-2>

assumptions:
  - <what was assumed>
```

## Key Questions Answered

1. **Did capability changes update the catalog?** - Are new LSP features reflected in features.toml?
2. **Are computed metrics accurate?** - Does CURRENT_STATUS.md match actual state?
3. **Did anti-drift guardrails work?** - Would status-check have caught violations?
4. **Are schemas compliant?** - Do dossiers and receipts follow contracts?
5. **Are claims properly anchored?** - Do assertions have evidence pointers?

## Catalog Drift Detection

### What to Check

For each code change that affects LSP capabilities:

1. **New feature implementation** → Should add entry to features.toml
2. **Feature maturity change** (e.g., beta→ga) → Should update maturity field
3. **Feature removal/deprecation** → Should update advertised field or add deprecation note
4. **Test coverage change** → Should update tests field

### Severity Classification

| Severity | Criteria |
|----------|----------|
| **P1** | Advertised capability with no catalog entry; GA feature marked as beta |
| **P2** | Catalog entry missing test references; maturity mismatch |
| **P3** | Description outdated; minor field omissions |

## Metrics Drift Detection

### Sources of Truth

Compare CURRENT_STATUS.md claims against:

1. **Test counts** - From `cargo test` output or CI logs
2. **LSP coverage** - From `features.toml` (advertised_ga / trackable)
3. **Ignored test baseline** - From `.ignored-baseline` file
4. **Mutation score** - From mutation testing output (if available)

### Auto-Correction Check

If `just status-update` exists and would fix the drift:
- Mark as `auto_correction_available: yes`
- Recommend running the command
- Reduce severity (fixable process issue, not governance failure)

## Schema Compliance

### Dossier Schema Requirements

Check forensics dossiers for:

- Cover sheet with required fields: Issue(s), PR(s), Exhibit ID
- Review map with file paths
- Verification section with receipts
- Known limits section
- Reproducibility command

### Receipt Schema Requirements

Check PR artifacts for:

- CI run links with status
- Test output excerpts
- Mutation testing results (if claimed)
- Evidence anchors (file:line references)

## Guardrail Effectiveness Assessment

### Status Check Gate

1. **Did it run?** - Check CI logs or local output
2. **Would it have caught this?** - Simulate what status-check would show
3. **Was it bypassed?** - Check if PR merged despite failures

### Ignored Test Tracking

1. **Baseline updated?** - Check `.ignored-baseline` diff
2. **Justifications present?** - Check test annotations for ignore reasons
3. **Debt trending down?** - Compare before/after counts

### CI Gate Enforcement

1. **Required checks enabled?** - Verify branch protection
2. **Local-first development?** - Check if CLAUDE.md documents local gate
3. **Would have prevented merge?** - Assess if issues were detectable

## Claim Provenance Analysis

### Strong Provenance (High Confidence)

- Claim backed by CI run link
- Claim backed by code anchor (file:line)
- Claim backed by test output excerpt
- Claim backed by mutation testing results

### Weak Provenance (Low Confidence)

- Claim with "should" or "will" without evidence
- Performance claims without benchmark output
- Coverage claims without measurement
- "Fixes X" without regression test reference

## Example Input

```
<pr_metadata>
PR Number: 265
Title: Add workspace symbol provider
Stated Scope: Implement textDocument/workspaceSymbol LSP feature
</pr_metadata>

<features_diff>
No changes to features.toml
</features_diff>

<status_diff>
No changes to docs/CURRENT_STATUS.md
</status_diff>

<capability_changes>
diff --git a/crates/perl-parser/src/lsp/workspace_symbols.rs b/crates/perl-parser/src/lsp/workspace_symbols.rs
new file mode 100644
+pub fn workspace_symbol(index: &WorkspaceIndex, query: &str) -> Vec<SymbolInformation> {
+    // Implementation...
+}

diff --git a/crates/perl-lsp/src/handlers.rs b/crates/perl-lsp/src/handlers.rs
+    .workspace_symbol_provider(Some(OneOf::Left(true)))
</capability_changes>

<status_check_output>
Running: just status-check
✓ LSP Coverage metric matches features.toml
✓ Ignored test baseline in sync
</status_check_output>
```

## Example Output

```yaml
analyzer: policy-auditor
pr: 265
timestamp: 2025-01-07T12:00:00Z
coverage: github_plus_agent_logs

catalog_drift:
  detected: true
  violations:
    - capability: lsp.workspace_symbol
      change_type: added
      code_evidence: "crates/perl-lsp/src/handlers.rs:42"
      catalog_updated: no
      severity: P1
      notes: "New LSP feature advertised but not in features.toml catalog"
  corrections: []

metrics_drift:
  detected: true
  violations:
    - metric: "LSP Coverage"
      claimed_value: "82% (27/33 GA advertised)"
      actual_value: "85% (28/33 GA advertised)"
      drift_magnitude: "3%"
      severity: P2
      evidence: "features.toml + new workspace_symbol capability"
  auto_correction_available: yes
  correction_command: "just status-update"

schema_compliance:
  dossier:
    status: pass
    issues: []
  receipts:
    status: warn
    issues:
      - location: "PR description"
        violation: "Claims 'workspace symbol provider works' without test output"
        severity: P3

guardrail_effectiveness:
  status: warn
  status_check:
    ran: yes
    caught_drift: no
    false_positives: 0
  ignored_test_tracking:
    baseline_updated: N/A
    delta: 0
    properly_justified: N/A
  ci_gate:
    enforced: yes
    would_have_caught: no
    notes: "status-check doesn't validate features.toml completeness vs code"

claim_provenance:
  new_claims:
    - claim: "Workspace symbol provider implemented"
      location: "PR description line 3"
      evidence_provided: partial
      evidence_anchor: "Code diff shows implementation, but no test output"
      confidence: medium
  claims_without_evidence:
    - claim: "Supports fuzzy matching for symbol search"
      location: "PR description line 8"
      severity: P2
      recommendation: "Add test output or code anchor showing fuzzy match logic"

findings:
  - id: POL-001
    severity: P1
    category: catalog_drift
    summary: New workspace_symbol capability not added to features.toml
    evidence:
      - anchor: crates/perl-lsp/src/handlers.rs:42
        content: ".workspace_symbol_provider(Some(OneOf::Left(true)))"
      - anchor: features.toml
        content: "No lsp.workspace_symbol entry found"
    recommendation: "Add [[feature]] entry for lsp.workspace_symbol with maturity, tests, description"
    confidence: high

  - id: POL-002
    severity: P2
    category: metrics_drift
    summary: LSP Coverage metric needs recomputation after capability addition
    evidence:
      - anchor: docs/CURRENT_STATUS.md:39
        content: "82% (27/33 GA advertised)"
    recommendation: "Run `just status-update` to recompute LSP Coverage metric"
    confidence: high

  - id: POL-003
    severity: P3
    category: unanchored_claim
    summary: Fuzzy matching claim lacks evidence
    evidence:
      - anchor: "PR description"
        content: "Supports fuzzy matching for symbol search"
    recommendation: "Add test case demonstrating fuzzy match behavior or remove claim"
    confidence: medium

summary:
  verdict: warn
  key_findings:
    - New LSP capability advertised but missing from features.toml catalog (P1)
    - LSP Coverage metric needs update after capability addition (P2)
    - status-check gate doesn't validate catalog completeness (guardrail gap)
  governance_delta: -1

assumptions:
  - features.toml is the single source of truth for LSP capabilities
  - status-update command would correctly recompute metrics
  - workspace_symbol_provider(true) indicates GA-ready feature
```

## Trust Model

### Can Be Inferred (High Confidence)

- Presence/absence of features.toml changes
- Code changes that advertise LSP capabilities
- Metric values in CURRENT_STATUS.md
- Schema structure violations (missing fields, wrong types)

### Can Be Inferred (Medium Confidence)

- Whether a code change constitutes a "new feature" vs "enhancement"
- Maturity level appropriate for a capability (beta vs GA)
- Severity of catalog drift (P1 vs P2)
- Whether a claim requires evidence

### Cannot Be Inferred

- Whether status-update script is correct (requires execution)
- Actual test coverage percentage (requires instrumentation)
- Whether CI gate would catch in future (requires config inspection)
- User impact of governance violations

### Red Flags to Note

- New LSP capability advertised without features.toml entry
- Metrics in CURRENT_STATUS.md that can't be traced to source
- Claims of "fixes #NNN" without corresponding test changes
- Ignored test baseline changes without justification
- Dossiers missing required sections (receipts, known limits)

## Integration Notes

Policy Auditor uses:
- **Diff Scout output**: File categorization to identify governance-relevant changes
- **Verification Auditor output**: Test evidence for claim validation
- **Docs Auditor output**: Documentation drift signals

Policy Auditor feeds into:
- **Dossier synthesis**: Governance delta for cover sheet
- **Lessons learned**: Guardrail gaps and improvements
- **Process improvements**: Recommendations for automated checks

For full analysis, run `just status-check` before and after the PR to detect drift.
