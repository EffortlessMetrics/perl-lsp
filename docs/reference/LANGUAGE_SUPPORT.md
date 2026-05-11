# Perl Language Support

This page documents the Perl language constructs perl-lsp understands across
its four primary consumers (PL701 diagnostic, completion, goto-definition,
hover). It is a user-facing reference; authoritative definitions of behavior
live in the linked status documents.

> **Rule**: Indexes provide candidates. Context determines authority.
>
> If this page and a status doc disagree, the status doc wins.

## Module resolution

The full consumer-consistency matrix and `@INC` rail status live in
[`docs/project/status/module_resolution.md`](../project/status/module_resolution.md).

Summary: `use Foo;`, `use lib '...';`, `no lib '...';`, `FindBin`-relative
includes, `PERL5LIB`, and interpreter-startup `@INC` are all supported across
the four consumers, gated by `usePerl5lib` and `useSystemInc` workspace flags.

## Literal `require` and `import`

Tracked under spec issue
[#8616](https://github.com/EffortlessMetrics/perl-lsp/issues/8616) (umbrella
ux-journey: [#4280](https://github.com/EffortlessMetrics/perl-lsp/issues/4280)).

All literal-form resolution flows through `EffectiveIncContext` (introduced by
[#8544](https://github.com/EffortlessMetrics/perl-lsp/pull/8544)). Forms in
the in-scope column are resolved by every consumer; forms in the out-of-scope
column are flagged as "dynamic; cannot statically resolve" but never guessed.

### Supported forms

| Form | Example | Resolution |
|---|---|---|
| Bareword `require` | `require Foo;` | resolve `Foo` via `EffectiveIncContext` |
| Literal single-segment | `require "Foo.pm";` | resolve same as bareword `Foo` |
| Literal multi-segment | `require "Foo/Bar.pm";` | resolve same as bareword `Foo::Bar` |
| Static `import` | `import Foo;` | resolve same as `use Foo;` |
| Static method-call import | `Foo->import;` | resolve `Foo` only; do not interpret import list |

### Boundary table

| In scope | Out of scope |
|---|---|
| `require Foo;` (bareword) | `eval STRING` containing `use`/`require` |
| `require "Foo.pm";` | `require $module;` (variable path) |
| `require "Foo/Bar.pm";` | `require "${prefix}::Foo";` (string interpolation) |
| `import Foo;` (static, no list) | Runtime `import` with computed module names |
| `Foo->import;` (static method call) | Plugin frameworks synthesizing module names at runtime |

Authoritative behavior matrix:
[`docs/project/status/module_resolution.md` → "Literal `require` / `import`"](../project/status/module_resolution.md#literal-require--import).
