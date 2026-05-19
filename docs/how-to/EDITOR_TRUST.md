# Editor Trust

`perl-lsp` is designed to be useful without pretending Perl is statically
simple. It acts when facts are fresh, source-backed, and high confidence. It
falls back when evidence is weak. It refuses edits when the proof is unsafe.

The short version:

```text
The editor helps when it knows.
It falls back when it is unsure.
It refuses dangerous edits.
It labels generated and dynamic Perl boundaries.
It can explain its decision in a bug-report-friendly form.
```

This page is a user guide, not the support matrix. Current claim boundaries
live in [Support tiers](../project/status/SUPPORT_TIERS.md), and the broader
model is explained in [Measured Perl Editor Trust](../explanation/MEASURED_PERL_EDITOR_TRUST.md).

## Trust Words

You may see these words in provider explanations, diagnostics, previews, and
workspace trust reports:

| Word | Meaning |
| --- | --- |
| `source-backed` | The answer is tied to a concrete source range in the workspace. |
| `generated/framework` | The symbol comes from framework or generated behavior and is labeled instead of treated as an exact method body. |
| `dynamic boundary` | Perl behavior depends on dynamic runtime features such as symbolic names, typeglobs, `AUTOLOAD`, dynamic require, or framework dispatch. |
| `fresh` | The fact matches the current indexed document or workspace state. |
| `stale` | The fact may no longer match the current source and should not authorize a stronger answer. |
| `low confidence` | The editor found weak evidence and keeps conservative behavior. |
| `ambiguous` | More than one plausible identity exists, so exact navigation or edits are unsafe. |
| `fallback` | The provider used a safer legacy or no-edit path instead of a proof-backed action. |

## Commands

In VS Code, open the command palette and search for these commands.

| Command | Use it when |
| --- | --- |
| **Perl LSP: Explain Provider Decision** | Completion, goto, references, hover, symbols, tokens, rename, or safe-delete behavior looks surprising. |
| **Perl LSP: Copy Provider Decision Receipt** | You want a structured payload to paste into a bug report. |
| **Perl LSP: Show Workspace Trust Report** | You need to check Perl binary, include paths, workspace roots, setup hints, provider tiers, and dynamic-boundary policy. |
| **Perl LSP: Explain This Diagnostic** | A PL701/PL109 diagnostic needs a plain-language reason. This is also available as a diagnostic code action when supported by the editor. |
| **Perl LSP: Explain Missing Module Lookup** | A module cannot be found and you need to see the effective `@INC` lookup state. |
| **Perl LSP: Preview Safe Delete** | You want to see whether symbol deletion is allowed, blocked, or refused before any edit. |
| **Perl LSP: Preview Package Rename** | You want a package/compiler-backed rename preview without authorizing an edit. |

The output-channel text is meant to be readable first. The copied receipt is
meant for issues and support reports.

## Why Providers Fall Back

Completion, hover, goto, references, workspace symbols, and semantic tokens use
source-backed facts where the repo has proof. They keep fallback behavior for
generated, dynamic, stale, low-confidence, ambiguous, partial-index, or
no-source cases.

That means a result can be useful without becoming a stronger claim than the
evidence supports. For example, a framework accessor may appear as a labeled
virtual symbol, while a dynamic method name may stay gated because there is no
exact source-backed identity.

## Why Diagnostics Stay Conservative

Diagnostics are allowed to keep warnings when the editor cannot prove the code
is safe. A diagnostic explanation can tell you whether the warning came from a
true missing module, low-confidence semantic evidence, an ambiguous symbol, or a
dynamic boundary.

For PL701 missing-module diagnostics, use **Explain This Diagnostic** or
**Explain Missing Module Lookup** to see the requested module, expected file
path, effective include paths, configured include paths, and `PERL5LIB` policy.

## Why Rename Or Safe Delete Refuses

Rename and safe delete can damage code, so they use a higher bar than read-only
providers.

Safe edits require narrow proof:

```text
fresh
source-backed
high confidence
not generated
not dynamic
not ambiguous
rollback-safe where edits are returned
```

When proof is missing, `perl-lsp` returns a preview, fallback, blocker, or empty
edit with a reason. This is intentional. A refused unsafe edit is a successful
trust decision.

## Setup Problems

Many Perl LSP issues are setup issues rather than code issues:

```text
wrong Perl binary
missing include path
unexpected PERL5LIB policy
missing perldoc
DAP using different Perl settings
workspace opened at the wrong root
```

Start with **Perl LSP: Show Workspace Trust Report**. It aggregates existing
server state and setup hints without scanning the workspace, running perldoc,
starting DAP, probing Perl, or promoting support claims.

## Filing A Useful Bug Report

When behavior looks wrong:

1. Run **Perl LSP: Show Workspace Trust Report**.
2. Run **Perl LSP: Explain Provider Decision** or the diagnostic/module
   explanation command that matches the problem.
3. Run **Perl LSP: Copy Provider Decision Receipt** if the issue is provider
   behavior.
4. Paste the readable explanation and structured receipt into the issue.

A useful report says what the editor did and why it thought that was the safe
choice:

```text
provider: goto-definition
decision: fallback
reason: ambiguous_low_confidence_candidates
fact_source: compiler facts
freshness: fresh
confidence: low
fallback: legacy provider
support-tier: partial-live-with-fallback
```

That lets maintainers distinguish a code bug, a setup issue, a dynamic Perl
boundary, and a support-tier limitation.
