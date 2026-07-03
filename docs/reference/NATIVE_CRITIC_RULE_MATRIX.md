# Native Critic Rule Matrix

The **native critic** is perl-lsp's built-in static-analysis linter. It ships in
the server binary and requires **no external tools** — `perlcritic` /
`Perl::Critic` are only used when you explicitly select the legacy engine (see
[Configuration](#configuration)). This page is the product reference for every
rule the native analyzer ships: its rule ID, category, severity, the LSP
diagnostic severity it maps to, and which profile enables it.

> Source of truth: rule definitions live in
> `crates/perl-lsp-rs-core/src/tooling/perl_critic/native.rs`; the profiles and
> filtering live in `.../perl_critic/native/native_registry.rs`; the severity
> model lives in `.../perl_critic/types.rs`. This doc is transcribed from that
> code and is enforced against it — if it drifts, treat the code as correct.

## Configuration

The native critic is configured by the `[critic]` table in `.perl-lsp.toml`
(and the equivalent `perl-lsp.critic.*` editor settings). See
[CONFIG.md](CONFIG.md) for the full schema.

| Key | Default | Meaning |
|-----|---------|---------|
| `[critic] engine` | `native` | `native` (built-in) or `legacy` (external `perlcritic`). |
| `[critic] profile` | `recommended` | Base rule set: `recommended` (16 rules) or `strict` (all 28). |
| `[critic] include` | `[]` | **Whitelist within the selected profile.** When non-empty, only the listed rule IDs run — and only if they are already in the profile. It does **not** add rules from outside the profile. |
| `[critic] exclude` | `[]` | Rule IDs to remove from the selected profile. |

**Severity threshold** is configured separately, **not** under `[critic]`. It is
`[diagnostics] perlcritic_severity` in `.perl-lsp.toml` (or the
`perl-lsp.perlcritic.severity` / `perl-lsp.critic.severity` editor setting), and
it applies to the native engine too: a rule is reported only when its severity
is at or above the threshold (default `3`).

> To run a strict-only rule (e.g. `native.variables.unused_lexical`), set
> `profile = "strict"` — adding it to `include` under the `recommended` profile
> will **not** enable it, because `include` only whitelists rules that the
> profile already contains.

## Severity model

Each rule has a native `Severity`. The numeric value drives both the severity
threshold filter (`perlcritic_severity`, see [Configuration](#configuration))
and the mapping to an LSP diagnostic severity:

| Native severity | Numeric | LSP `DiagnosticSeverity` |
|-----------------|:-------:|--------------------------|
| `Gentle` | 5 | `ERROR` |
| `Stern` | 4 | `WARNING` |
| `Harsh` | 3 | `WARNING` |
| `Cruel` | 2 | `INFORMATION` |
| `Brutal` | 1 | `HINT` |

A finding is kept only when `rule_severity >= perlcritic_severity`. At the default
threshold of `3`, every shipped native rule passes (the lowest native severity
in use is `Harsh` = 3). Raising the threshold to `4` drops the `Harsh` rules;
`5` keeps only the two `Gentle` rules. No shipped native rule uses `Cruel` (2)
or `Brutal` (1), so native diagnostics are only ever `ERROR` or `WARNING`.

## Profiles

`profile` selects the active rule set. **Recommended** is the balanced
default (16 rules). **Strict** is a strict superset — the same 16 plus 12 more
(28 total). `include` (whitelist) / `exclude` (blacklist) then narrow that set
by rule ID — they filter *within* the profile and cannot pull in a rule the
profile does not contain.

## Rule matrix

Categories in use: **Security**, **Syntax**, **Semantic**, **Maintainability**,
**Documentation**. (The `Workspace` and `Style` categories exist in the model
but no shipped rule uses them.)

| Rule ID | Category | Severity | LSP | Flags | Recommended | Strict |
|---------|----------|----------|-----|-------|:-----------:|:------:|
| `native.testing.require_use_strict` | Syntax | Harsh (3) | WARNING | Code does not `use strict` | ✓ | ✓ |
| `native.testing.require_use_warnings` | Syntax | Harsh (3) | WARNING | Code does not `use warnings` | ✓ | ✓ |
| `native.common.assignment_in_condition` | Syntax | Stern (4) | WARNING | Assignment in condition — did you mean `==`? | ✓ | ✓ |
| `native.common.printf_format_arity` | Syntax | Stern (4) | WARNING | `printf`/`sprintf` format specifier count ≠ args | ✓ | ✓ |
| `native.common.deprecated_defined` | Syntax | Stern (4) | WARNING | Use of `defined @array` / `defined %hash` is deprecated | ✓ | ✓ |
| `native.common.undef_comparison` | Syntax | Stern (4) | WARNING | Comparing with `undef` — use `defined()` first | ✓ | ✓ |
| `native.common.stale_dollar_at` | Syntax | Stern (4) | WARNING | Checking `$@` after `eval` can observe a stale error | ✓ | ✓ |
| `native.common.unreachable_code` | Maintainability | Harsh (3) | WARNING | Unreachable code: statement cannot execute | ✓ | ✓ |
| `native.io.bareword_filehandle` | Syntax | Stern (4) | WARNING | Bareword filehandle should be lexical | ✓ | ✓ |
| `native.io.two_arg_open` | Security | Harsh (3) | WARNING | Two-argument `open` should use an explicit mode | ✓ | ✓ |
| `native.io.pipe_open` | Security | Harsh (3) | WARNING | Pipe-`open` executes a shell command | ✓ | ✓ |
| `native.io.unchecked_open_close` | Security | Stern (4) | WARNING | Unchecked `open`/`close` I/O call | ✓ | ✓ |
| `native.security.qx_readpipe` | Security | Harsh (3) | WARNING | `qx`/`readpipe` command execution detected | ✓ | ✓ |
| `native.security.backtick_exec` | Security | Harsh (3) | WARNING | Backtick command execution detected | ✓ | ✓ |
| `native.security.string_eval` | Security | Harsh (3) | WARNING | String `eval` is a security risk | ✓ | ✓ |
| `native.security.system_exec` | Security | Harsh (3) | WARNING | `system()`/`exec()` runs a shell command | ✓ | ✓ |
| `native.variables.unused_lexical` | Semantic | Stern (4) | WARNING | Lexical variable declared but never used | – | ✓ |
| `native.variables.unused_parameter` | Semantic | Stern (4) | WARNING | Signature parameter is never used | – | ✓ |
| `native.variables.duplicate_parameter` | Semantic | Gentle (5) | ERROR | Parameter appears more than once in a signature | – | ✓ |
| `native.variables.parameter_shadows_global` | Semantic | Stern (4) | WARNING | Parameter shadows an outer declaration | – | ✓ |
| `native.variables.duplicate_lexical` | Semantic | Gentle (5) | ERROR | Lexical declared more than once in the same scope | – | ✓ |
| `native.variables.shadowed_lexical` | Semantic | Stern (4) | WARNING | Lexical variable shadows an outer declaration | – | ✓ |
| `native.regex.capture_without_match` | Semantic | Stern (4) | WARNING | Capture variable used with no preceding regex match | – | ✓ |
| `native.variables.undeclared` | Semantic | Stern (4) | WARNING | Variable used but not declared | – | ✓ |
| `native.variables.uninitialized` | Semantic | Stern (4) | WARNING | Variable used before initialization | – | ✓ |
| `native.syntax.unquoted_bareword` | Syntax | Stern (4) | WARNING | Bareword not allowed under `strict` | – | ✓ |
| `native.documentation.require_pod_sections` | Documentation | Harsh (3) | WARNING | POD is missing a required `=head1` section | – | ✓ |
| `native.syntax.prohibit_leading_zeros` | Syntax | Stern (4) | WARNING | Integer literal with a leading zero is octal | – | ✓ |

**Recommended profile** (16 rules): the first 16 rows above (through
`native.security.system_exec`) — the security- and correctness-critical set.
**Strict profile** (28 rules): all rows.

## Notes on defaults

- The documented config default is `critic.profile = "recommended"`, so a normal
  configuration yields the **16-rule Recommended** profile.
- If the profile string fails to parse (absent or an unrecognized token), the
  runtime falls back to **Strict**, not Recommended
  (`NativeCriticProfile::parse(...).unwrap_or(NativeCriticProfile::Strict)` in
  the diagnostics, pull-diagnostics, and `perl.runCritic` paths). Set a valid
  `recommended`/`strict` token to get deterministic behavior.
- `## no critic` suppression comments are honored (see
  `native/native_suppressions.rs`).

## Suppressing a rule

- Disable one rule from the active profile: add its ID to `[critic] exclude`.
- Run a strict-only rule: set `[critic] profile = "strict"` (then optionally
  `exclude` the ones you don't want). Adding it to `include` under `recommended`
  will not enable it — `include` only whitelists rules the profile contains.
- Suppress inline: a `## no critic` comment on the offending line.

## Related

- [CONFIG.md](CONFIG.md) / [CONFIGURATION.md](CONFIGURATION.md) — the `[critic]`
  configuration schema.
- [NATIVE_STACK_POLICY.md](NATIVE_STACK_POLICY.md) — why the native stack is the
  product and external tools are compatibility-only.
