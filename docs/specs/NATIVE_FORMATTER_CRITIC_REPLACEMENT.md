# Native Formatter and Critic Replacement Contract

**Status**: Draft
**Scope**: native replacement contract for `perltidy` and `perlcritic`
**Current adapter state**: `perl-lsp-perltidy` shells out to `perltidy` through
`perl-subprocess-runtime` and keeps a small built-in fallback; `perl.runCritic`
is an existing execute-command surface; `perl-diagnostics` is the canonical
diagnostic model.

---

## Goal

`perltidy` and `perlcritic` should become optional legacy adapters, not normal
operational dependencies. During normal editing, review, and CI, users should be
able to rely on native Rust formatting and critic diagnostics that use the same
parser, lossless syntax, semantic facts, workspace index, diagnostics catalog,
LSP handlers, code actions, and receipt flow as the rest of `perl-lsp`.

This does not mean cloning every historical option or policy bit-for-bit. It
means replacing the practical value users expect from the traditional tools with
native behavior that is faster, deterministic, explainable, testable,
workspace-aware, and integrated into editor and CI proof surfaces.

The native path must be the obviously better default, not merely an acceptable
fallback. External tools may remain valuable for migration, comparison, and edge
cases, but routine use should prefer the native engines because they have fewer
runtime dependencies, richer spans, safer edits, better provenance, direct code
actions, and receipt-backed CI/editor feedback.

---

## Definitions

| Term | Meaning |
|---|---|
| Native formatter | Rust implementation that formats from repository-owned syntax/token data and returns LSP-ready edits without shelling out. |
| Native critic | Rust rule engine that emits diagnostics, explanations, suppressions, and code actions through `perl-diagnostics` and LSP surfaces. |
| Legacy adapter | Explicit opt-in shell-out path to external `perltidy` or `perlcritic`. |
| Compatibility mode | A mode that approximates common external-tool profiles while still running through native code where safe. |
| Receipt | Structured evidence produced by tests or xtask checks that records coverage, safety, parity, or migration status. |

---

## Operating Modes

Formatting and critic analysis must expose explicit modes instead of silently
depending on external binaries:

```text
native            default when the native engine is production-ready
compat            native behavior tuned toward common perltidy/perlcritic profiles
external-legacy   explicit opt-in shell-out to traditional tools
off               disabled
```

Initial implementation may keep the existing external behavior as the default,
but every PR in this lane should move toward this final state:

```text
default editor path: native
external shell-out: explicit legacy/compat opt-in
```

Capability advertisement must match the configured mode. A missing external
binary must not disable native formatting or native critic diagnostics.

---

## Formatter Architecture

The formatter must be lossless-first:

```text
source text
  -> lossless tokens / CST / trivia
  -> formatting view
  -> document IR
  -> stable pretty-printer
  -> LSP TextEdit values
```

The formatting view may use AST and semantic facts, but it must not depend on a
simplified AST alone. Perl formatting must preserve syntax that users care about:

```text
comments
POD
heredocs
quote-like operators
regular expressions and substitutions
sigils and typeglobs
DATA sections
intentional blank lines
trailing comments
valid but unusual Perl idioms
```

### Formatter API Contract

The first native formatter API should be independent of the existing
`PerlTidyFormatter` subprocess adapter:

```rust
pub trait PerlFormatter {
    fn format_document(&self, source: &str, config: &FormatConfig) -> FormatResult;
    fn format_range(&self, source: &str, range: TextRange, config: &FormatConfig) -> FormatResult;
}

pub struct FormatResult {
    pub formatted: String,
    pub edits: Vec<TextEdit>,
    pub changed: bool,
    pub diagnostics: Vec<FormatDiagnostic>,
}
```

`FormatResult` must distinguish "already formatted" from "unsafe to format".
Unsafe cases should return diagnostics and no edits rather than guessing.

### Document IR

The native formatter should use a small document tree instead of ad hoc string
concatenation:

```text
Text
Line
Group
Indent
SoftLine
HardLine
Space
IfBreak
LiteralPreserve
```

The first formatter wave should render safe, common constructs:

```text
package/use declarations
sub declarations
blocks
if/elsif/else
while/for/foreach
my/our/state declarations
function calls
hash/list literals
method calls
basic operators
```

Regexes, substitutions, heredocs, POD, DATA sections, and unknown constructs
should first be preserved as literals. Later PRs may format inside them only
after preservation tests and explicit safety gates exist.

### Formatter Safety Invariants

Every native formatter PR that changes formatting behavior must prove:

| Invariant | Required proof |
|---|---|
| Idempotence | `format(format(source)) == format(source)` for fixtures touched by the PR. |
| Parse preservation | `parse(source)` and `parse(format(source))` succeed, or the formatter returns no edits for unsafe input. |
| Structural preservation | A normalized structural signature is unchanged where the parser exposes one. |
| Comment/POD/heredoc preservation | Preserved byte-for-byte unless the PR explicitly owns a safe transformation. |
| Range safety | Range formatting edits only the requested range plus documented indentation boundaries. |
| Stable LSP edits | Returned edits are deterministic and minimally scoped enough for format-on-save. |

---

## Critic Architecture

The native critic replacement should be a rule registry over existing compiler
and workspace facts:

```text
source text
tokens / CST / AST
semantic facts
workspace index
module, import, POD, and config facts
suppression map
  -> rule registry
  -> critic findings
  -> diagnostics + code actions + receipts
```

Rules must be small and independently testable. There should be no monolithic
"native critic" pass whose behavior cannot be attributed to a stable rule ID.

### Critic Rule Contract

```rust
pub trait CriticRule {
    fn id(&self) -> &'static str;
    fn category(&self) -> CriticCategory;
    fn default_severity(&self) -> DiagnosticSeverity;
    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>);
}
```

Every finding must include:

```text
stable rule ID
category
severity
precise span
message
explanation
suppression key
related information when useful
safe fix or suggested fix when available
```

Rule IDs should be structured and stable, for example:

```text
native.variables.unused_lexical
native.variables.shadowed_lexical
native.control_flow.unreachable_statement
native.policy.missing_strict
native.workspace.module_not_found
```

### Rule Families

Implementation order should follow user value, not historical perlcritic catalog
order.

First wave, high-signal editor rules:

```text
unused lexical variable
shadowed lexical variable
duplicate declaration
unreachable statement
missing use strict / use warnings where safe
bareword filehandle hazards
implicit global / package variable hazards
suspicious assignment in condition
always-true / always-false simple predicate
dead code after return/die/exit
```

Second wave, maintainability:

```text
sub too long
too many branches
too many nested blocks
too many parameters
duplicate code shape
complex regex warning
```

Third wave, configurable conventions:

```text
naming conventions
package/file path mismatch
POD required for public subs/modules
import ordering
preferred builtins
preferred error handling style
```

Fourth wave, workspace-aware rules:

```text
module not found
symbol imported but unused
exported symbol not found
duplicate package declaration
stale module path
cross-file rename hazards
```

### Critic Safety Invariants

Every native critic rule PR must prove:

| Invariant | Required proof |
|---|---|
| Stable identity | Rule ID, category, default severity, and docs are present. |
| Precise span | Fixtures assert the diagnostic range, not just message text. |
| Suppression | Inline and file-level suppression behavior is tested when the rule can be suppressed. |
| Explanation | Finding includes a useful explanation or documentation link. |
| Fix safety | Code actions are labeled `safe`, `suggested`, or `manual-only`. |
| Output parity | LSP diagnostics, pull diagnostics, CLI/check output, and CI receipts agree where those surfaces apply. |

---

## Configuration and Import Model

Native formatter and critic configuration should share one project config
surface, with legacy external modes explicit:

```toml
[format]
engine = "native"          # native | compat | external-legacy | off
line_width = 100
indent = 4
tabs = false
profile = "default"

[critic]
engine = "native"          # native | compat | external-legacy | off
enabled = true
profile = "recommended"
severity.default = "warning"

[critic.rules.native.variables.unused_lexical]
severity = "warning"

[critic.rules.native.control_flow.too_many_branches]
severity = "info"
max = 12
```

Configuration import commands should be explicit and report gaps:

```bash
perl-lsp config import-perltidy .perltidyrc
perl-lsp config import-perlcritic .perlcriticrc
```

Import output must classify every setting:

```text
supported
mapped with approximation
unsupported but harmless
unsupported and requires external-legacy mode
```

Unsupported settings should never be silently ignored.

### Suppressions

Native critic suppressions must be stable and traceable:

```perl
## no critic native.variables.unused_lexical
## no perl-lsp-critic native.control_flow.too_many_branches -- generated dispatch table
```

Suppression records should preserve:

```text
rule ID
span or file scope
optional reason
optional expiry in a later PR
```

---

## LSP, CLI, and CI Surfaces

The native path must work through existing user surfaces:

| Surface | Formatter expectation | Critic expectation |
|---|---|---|
| LSP | `textDocument/formatting`, `textDocument/rangeFormatting`, and format-on-save edits. | Push diagnostics, pull diagnostics, code actions, and `perl.runCritic` compatibility. |
| CLI | Check and apply modes for local/CI use. | Check mode with structured output and optional SARIF if adopted. |
| CI | Idempotence, parse-preservation, and compatibility receipts. | Rule matrix, suppression matrix, code-action matrix, and policy receipts. |
| Config | Native/compat/external modes and imported profile reports. | Native/compat/external modes, profile import, severity remapping, suppressions. |

Editor-only and CI-only paths are not allowed. A rule or formatter behavior must
be testable outside the editor.

---

## Proof Receipts

The replacement lane must be receipt-first because formatting and linting
directly affect user trust.

Formatter receipts:

```text
native-format-fixtures.json
native-format-idempotence.json
native-format-parse-preservation.json
native-format-preservation-matrix.json
native-format-perltidy-compat.json
```

Formatter metrics:

```text
format_idempotence_rate
parse_preservation_rate
comment_preservation_rate
pod_preservation_rate
heredoc_preservation_rate
range_format_success_rate
perltidy_parity_rate
intentional_difference_count
unsafe_format_bailout_count
```

Critic receipts:

```text
native-critic-rule-matrix.json
native-critic-suppression-matrix.json
native-critic-code-action-matrix.json
native-critic-config-import.json
native-tooling-replacement-status.md
```

Critic metrics:

```text
rule_count
rules_with_fixtures
rules_with_suppression_tests
rules_with_code_action_tests
false_positive_fixture_count
workspace_rule_count
perlcritic_config_import_coverage
```

Receipts must identify the commit, scenario, fixture set, pass/fail result, and
unsupported or intentionally different behavior.

---

## Acceptance Criteria

### Formatter Done Criteria

The native formatter is done when:

```text
native formatter is default
full-document and range formatting work through LSP
format-on-save is safe enough for normal users
formatting is idempotent across corpus fixtures
comments, POD, heredocs, DATA sections, and literal-preserve regions are preserved
common perltidy profiles can be imported or approximated
unsupported options are reported honestly
external perltidy mode is explicit legacy/compat opt-in
```

### Critic Done Criteria

The native critic is done when:

```text
native diagnostics cover high-value practical perlcritic use cases
rules have stable IDs, severities, categories, suppressions, and docs
diagnostics have precise spans and useful explanations
safe rules expose code actions
CI, editor, and CLI outputs agree
.perlcriticrc import covers common configurations or reports gaps
external perlcritic mode is explicit legacy/compat opt-in
```

### Replacement Closeout Criteria

The whole lane is done when:

```text
normal editing, review, and CI no longer require perltidy or perlcritic
users choose the native path because it is faster, safer, richer, and better integrated
external modes are documented as legacy adapters
native formatter and critic status are visible in release docs
policy prevents accidental shell-out dependencies in the default path
receipts explain remaining gaps without requiring log archaeology
```

---

## Phased PR Map

### Track A: Formatter

```text
A1. docs(format): define native formatter and critic replacement contract
A2. feat(format): add native formatter trait and result model
A3. feat(format): add formatting IR and basic block/sub/declaration rendering
A4. test(format): add idempotence and parse-preservation fixtures
A5. feat(format): wire native formatter into LSP behind config
A6. feat(format): support range formatting through native edit model
A7. test(format): add comment/POD/heredoc preservation matrix
A8. feat(format): import common perltidy config subset
A9. docs(format): publish native-vs-perltidy compatibility matrix
A10. chore(format): make native formatter default, external perltidy explicit
```

### Track B: Critic

```text
B1. docs(critic): define native critic rule contract and config model
B2. feat(critic): add rule registry and CriticContext
B3. feat(critic): add stable rule IDs, severities, suppressions
B4. feat(critic): implement first high-signal syntax rules
B5. feat(critic): implement semantic rules using perl-semantic-analyzer
B6. feat(critic): expose native critic findings through LSP diagnostics
B7. feat(critic): add safe code actions for first fixable rules
B8. feat(critic): import common perlcritic config subset
B9. test(critic): add rule/suppression/code-action matrix
B10. chore(critic): make native critic default, external perlcritic explicit
```

### Track C: Replacement Closeout

```text
C1. xtask: add native-tooling-replacement-status report
C2. ci: add advisory native formatter/critic receipts
C3. docs: add migration guide from perltidy/perlcritic
C4. release: advertise native formatting and native critic as default alpha/beta capability
C5. policy: prevent accidental shell-out dependency in the default path
```

---

## Non-Goals for the Contract PR

This contract does not:

```text
change runtime behavior
change capability advertisement
remove perltidy or perlcritic support
make native formatting default
add a new CI gate
claim feature parity before receipts exist
```

The next implementation PR should add the native formatter trait/result model or
the native critic rule registry, but only after this contract is merged.
