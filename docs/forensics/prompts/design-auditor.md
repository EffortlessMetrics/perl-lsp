# Design Auditor Prompt

## Purpose

The Design Auditor analyzer assesses **Maintainability** impact of a PR. It evaluates module boundaries, coupling changes, and API surface stability to determine if the code is easier or harder to work with after the change.

**Quality Surface**: Maintainability

## Required Inputs

Provide the following context to the analyzer:

### 1. Git Diff
```
<git_diff>
[Full or summarized git diff output]
</git_diff>
```

### 2. Public API Surface (before/after or diff)
```
<api_surface>
[List of pub functions, types, traits that changed]
[Can be extracted via: grep -r "pub fn\|pub struct\|pub enum\|pub trait" in diff]
</api_surface>
```

### 3. Dependency Changes
```
<dependency_changes>
[Cargo.toml diff or dependency tree delta]
</dependency_changes>
```

### 4. Module Structure
```
<module_structure>
[mod.rs files, use statements, module declarations]
</module_structure>
```

### 5. Features/Schema Files (if changed)
```
<schema_files>
[features.toml, STABILITY.md, or similar schema definitions]
</schema_files>
```

### 6. Diff Scout Output (recommended)
```
<diff_scout>
[Output from diff-scout analyzer for file categories and hotspots]
</diff_scout>
```

## Output Schema

The analyzer must produce output conforming to this YAML schema:

```yaml
analyzer: design-auditor
pr: <number>
timestamp: <ISO8601>
coverage: <github_only|github_plus_agent_logs|receipts_included>

boundary_changes:
  - module: <path, e.g., "crates/perl-parser/src/lsp/">
    change: <added|removed|modified|split|merged>
    responsibilities_before: <description or "N/A" if new>
    responsibilities_after: <description>
    clarity_delta: <improved|unchanged|degraded>

coupling_delta:
  - from: <module path>
    to: <module path>
    change: <new|removed|strengthened|weakened>
    evidence: <use statement, function call, or type reference>

api_surface:
  public_functions:
    added: [<list of "module::function_name">]
    removed: [<list>]
    modified: [<list with brief change description>]
  public_types:
    added: [<list of "module::TypeName">]
    removed: [<list>]
    modified: [<list>]
  breaking_changes:
    - item: <function or type>
      reason: <why it breaks API>
      mitigation: <if any, e.g., "deprecated with migration path">

dependency_delta:
  added:
    - crate: <name>
      version: <version>
      justification: <if provided in PR>
  removed:
    - crate: <name>
      version: <version>
  updated:
    - crate: <name>
      from: <old version>
      to: <new version>
      breaking: <yes|no|unknown>

complexity_indicators:
  files_over_500_lines:
    before: <count>
    after: <count>
  deep_nesting_introduced: <yes|no>
  circular_dependency_risk: <yes|no|unknown>

findings:
  - id: <unique_id, e.g., "DA-001">
    severity: <P1|P2|P3|info>
    category: <boundary_blur|coupling_increase|api_break|complexity_spike|dependency_bloat>
    summary: <one line>
    evidence:
      - anchor: <file:line or module path>
        content: <excerpt>
    recommendation: <action>
    confidence: <high|medium|low>

summary:
  verdict: <pass|warn|fail>
  key_findings:
    - <bullet 1>
    - <bullet 2>
  maintainability_delta: <+2|+1|0|-1|-2>

assumptions:
  - <what was assumed>
```

## Key Questions Answered

1. **Did module boundaries get clearer or blurrier?** - Are responsibilities more or less well-defined?
2. **Are there new coupling patterns?** - Did dependencies between modules increase or decrease?
3. **Is the public API stable?** - Were there breaking changes? Are they documented?
4. **Did complexity concentrate or distribute?** - Are large files getting larger?
5. **Are new dependencies justified?** - Is there a reason for new crates?

## Boundary Clarity Assessment

### Improved Boundaries
- Large file split into focused modules
- Generic code extracted to shared module
- Single responsibility enforced
- Clear interface between layers

### Degraded Boundaries
- Multiple unrelated changes in one file
- God module gaining more responsibilities
- Cross-cutting concerns spread across modules
- Circular dependencies introduced

## Coupling Classification

| Change Type | Description | Risk Level |
|-------------|-------------|------------|
| `new` | New dependency between modules | Medium |
| `removed` | Coupling eliminated | Positive |
| `strengthened` | More calls/references between modules | High if bidirectional |
| `weakened` | Fewer direct dependencies | Positive |

## Example Input

```
<pr_metadata>
PR Number: 122
Title: Dual indexing for workspace symbols
Stated Scope: Index symbols under both qualified and bare names
</pr_metadata>

<api_surface>
+ pub fn index_symbol_dual(name: &str, qualified: &str, symbol: SymbolRef)
* pub fn get_symbol(name: &str) -> Option<&SymbolRef>  // now searches both forms
</api_surface>

<dependency_changes>
No new dependencies
</dependency_changes>

<module_structure>
crates/perl-parser/src/lsp/
  mod.rs
  workspace.rs    <- primary changes
  indexing.rs     <- new module
  symbols.rs
</module_structure>
```

## Example Output

```yaml
analyzer: design-auditor
pr: 122
timestamp: 2025-01-07T12:00:00Z
coverage: github_only

boundary_changes:
  - module: crates/perl-parser/src/lsp/
    change: modified
    responsibilities_before: "Workspace indexing with qualified names only"
    responsibilities_after: "Workspace indexing with dual (qualified + bare) name support"
    clarity_delta: improved
  - module: crates/perl-parser/src/lsp/indexing.rs
    change: added
    responsibilities_before: "N/A"
    responsibilities_after: "Centralized indexing logic for dual-form symbol registration"
    clarity_delta: improved

coupling_delta:
  - from: crates/perl-parser/src/lsp/workspace.rs
    to: crates/perl-parser/src/lsp/indexing.rs
    change: new
    evidence: "use crate::lsp::indexing::index_symbol_dual"

api_surface:
  public_functions:
    added:
      - "lsp::indexing::index_symbol_dual"
    removed: []
    modified:
      - "lsp::workspace::get_symbol - now searches both qualified and bare forms"
  public_types:
    added: []
    removed: []
    modified: []
  breaking_changes: []

dependency_delta:
  added: []
  removed: []
  updated: []

complexity_indicators:
  files_over_500_lines:
    before: 1
    after: 1
  deep_nesting_introduced: no
  circular_dependency_risk: no

findings:
  - id: DA-001
    severity: info
    category: boundary_blur
    summary: New indexing.rs module cleanly separates dual-indexing concern
    evidence:
      - anchor: crates/perl-parser/src/lsp/indexing.rs:1-50
        content: "Module contains focused index_symbol_dual function"
    recommendation: None - good separation
    confidence: high

summary:
  verdict: pass
  key_findings:
    - Extracted indexing logic to dedicated module improves boundary clarity
    - No breaking API changes
    - No new external dependencies
  maintainability_delta: +1

assumptions:
  - No circular dependency exists (would need full dependency graph to verify)
  - Module responsibilities inferred from file names and function signatures
```

## Trust Model

### Can Be Inferred (High Confidence)
- Public API changes from diff (pub fn/struct/enum/trait)
- New module additions from file structure
- Dependency changes from Cargo.toml diff
- Coupling from use statements and function calls

### Can Be Inferred (Medium Confidence)
- Boundary clarity (requires understanding of module purposes)
- Responsibility changes (inferred from code content)
- Breaking changes (requires understanding of consumer usage)

### Cannot Be Inferred
- Whether coupling is appropriate for the domain
- Historical context of why boundaries exist
- Consumer impact of API changes (requires downstream analysis)
- Runtime coupling (dynamic dispatch, traits)

### Red Flags to Note
- Module responsibilities becoming unclear or overlapping
- Files growing beyond 500-800 lines without split
- New bidirectional coupling between modules
- Breaking changes without deprecation path
- Dependencies added without clear justification

## Integration Notes

Design Auditor uses:
- **Diff Scout output**: File categories and hotspots focus the analysis

Design Auditor feeds into:
- **Dossier synthesis**: Maintainability delta for cover sheet
- **Policy Auditor**: Schema alignment checks

For API stability concerns, cross-reference with `features.toml` and `STABILITY.md`.
