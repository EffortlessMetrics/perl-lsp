# Measured Perl Editor Trust

`perl-lsp` is becoming a Rust-native Perl workbench whose first durable product
surface is the language server. The near-term claim is not that `perl-lsp`
replaces Perl. The near-term claim is narrower and more useful for editor
users: provider answers should be explainable, measured, and conservative.

The long-term architecture should still point higher than LSP. The same parser,
HIR, scope, stash, compile-environment, import/export, framework, compile-effect,
PIR, and oracle layers that make editor answers trustworthy are also the layers
needed for a bounded Rust-native Perl implementation path.

In short:

```text
Current product target:
  Real Perl Editor Trust

Long-term platform direction:
  Rust-native Perl compiler / runtime / tooling replacement path
```

The current lane is building the compiler-backed trust substrate that eventually
makes replacement possible without starting a second project.

## What Trust Means

A useful editor answer should be able to explain:

```text
what fact produced it
whether the fact is source-backed
how fresh the fact is
how confident the provider is
what fallback was used
what dynamic boundary blocks a stronger claim
```

This matters more for Perl than for many languages because meaningful behavior
can flow through imports, exports, package stashes, typeglobs, `AUTOLOAD`,
`BEGIN`, symbolic references, framework-generated methods, and ambient `@INC`
state. A provider that silently treats those surfaces as static facts will feel
fast until it damages a workspace.

Measured trust means `perl-lsp` prefers a blocked or fallback answer with a
precise reason over a confident-looking guess.

## Current Editor Loop

The Real Perl Editor Trust lane ties the user-visible providers together:

```text
completion suggests it
hover explains it
definition jumps to it
references finds its uses
diagnostics trusts it
rename / safe-delete know whether it is safe
symbols and tokens expose project shape without noise
```

Those surfaces should consume the same compiler facts instead of growing
provider-specific guesses. For example, a generated framework member should have
one fact that completion can suggest, hover can label, symbols can show,
rename can block, safe-delete can refuse, and determinism receipts can record.

## What Users Can Ask

The product value is Perl project observability:

```text
Which import made this symbol visible?
Which framework generated this method?
Which symbols are source-backed rather than virtual?
Which facts depend on configured roots, system @INC, or PERL5LIB?
Which facts are stale?
Which code is behind eval, AUTOLOAD, symbolic refs, or dynamic require?
Which rename or delete operations are safe?
Which modules can be reasoned about without executing user code?
```

When the tool cannot prove an answer, the refusal is part of the feature. A
blocked rename because of `AUTOLOAD`, a symbolic reference, an exported symbol,
or a stale fact protects the user from unsafe automation.

## Replacement Path

The replacement path is not a current user-facing support claim. It is the
architectural direction.

Every LSP and tooling feature should improve the compiler substrate:

```text
source
-> lexer / parser
-> HIR
-> scope / pad facts
-> stash / package facts
-> compile environment
-> import / export visibility
-> framework-generated facts
-> compile-time effects
-> PIR / control flow / data flow
-> determinism receipts
-> editor, formatter, critic, refactor, release, and runtime tooling
```

Real Perl remains essential as a conformance oracle and compatibility proof
harness. It should not become the normal editor source of truth, and hidden
ambient Perl execution should not be required for baseline editor answers.

The long-term platform becomes credible when those same facts support:

```text
canonical HIR
compiler-grade scope and stash modeling
explicit compile environment and @INC roots
modeled imports, exports, frameworks, and compile-time effects
PIR with context, control flow, calls, data flow, and call graph
determinism receipts for modeled, ambient, stale, dynamic, and unknown inputs
differential agreement against real Perl
bounded compile or execution targets
```

That is the distinction:

```text
Real Perl Editor Trust now.
Rust-native Perl replacement path eventually.
Same architecture.
No rewrite.
```

## Where Current Claims Live

This page is explanatory. Current evidence and support claims live in the status
and support documents:

- [Real Perl Editor Trust v1 dashboard](../project/status/real_perl_editor_trust_v1.md)
- [Provider confidence matrix](../project/status/provider_confidence_matrix.md)
- [Provider cutover status](../project/status/provider_cutover.md)
- [Support tiers](../project/status/SUPPORT_TIERS.md)
- [Semantic shadow compare](../project/status/semantic_shadow_compare.md)
- [Semantic scorecard](../project/status/semantic_scorecard.md)
- [Compiler-backed LSP roadmap](../project/COMPILER_BACKED_LSP_ROADMAP.md)
