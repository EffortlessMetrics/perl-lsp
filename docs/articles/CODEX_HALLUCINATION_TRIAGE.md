# Codex Hallucination Triage

**Status:** Playbook derived from the 2026-04-23/04-24 session where 21+ hallucinated PRs were closed across 6 product clusters.

## The failure mode

Codex, when given a broad prompt involving Perl framework detection ("improve framework support", "expand Moo-family detection", "recognize Exporter patterns"), will sometimes produce a coherent 3-4 PR cluster that teaches perl-lsp to recognize **a name from its training periphery** (an agentic editor, an AI coding tool, a JS visual builder) as a Perl framework.

The shape is always:

1. `feat(parser-core): add <Name> template extension support` — adds `.<name>` to `PERL_SOURCE_EXTENSIONS`
2. `feat(semantic-analyzer): add <Name> web route detection` — adds `WebFrameworkKind::<Name>` + route extraction patterns
3. `feat(semantic-analyzer): add <Name> framework aliases` — treats `use <Name>;` / `use <Name>::Role` as Moo-family
4. `fix(execute-command): skip <Name>CLAW modules in go-to-implementation` — adds namespace to skip list

Each individual PR is:
- Coherent Rust code
- Clippy clean
- Uses idiomatic patterns already in the crate
- Has unit tests that pass locally
- Has a plausible motivation paragraph

The violation isn't against Rust coding standards. It's against the **Perl ecosystem** — the named module does not exist on CPAN, the file extension is never used by any Perl project, the `Name::Role` pair is pure fabrication.

## Known-closed hallucinations (examples)

| Fake Perl thing | What it actually is |
|---|---|
| OpenClaw | Agentic coding editor |
| Droid / Droid::Factory | Factory.ai terminal coding agent |
| Builder::IO::Fusion | builder.io — a JavaScript AI visual-builder |
| Google::Antigravity | Google's agentic development browser |
| Hermes Agent (framework flavor) | Nous Research model family |
| `.mcp` as Mason extension | Anthropic MCP — a protocol, not a Perl template format |

## Why standards review misses this

Haiku standards review checks: banned-pattern grep (`unwrap`, `expect`, `panic`, `dbg`, `todo`), title-format regex (`(#NNN)`), scope boundaries (files touched match title). Every hallucinated PR passes all three. The code is clean; the *premise* is false.

Web research is the only way to catch it.

## The pre-gate: MetaCPAN check

Before any PR can add an entry to a framework-detection table, verify the name exists on CPAN.

```bash
curl -s "https://fastapi.metacpan.org/v1/module/_search?q=<Name>&size=3" \
  | jq '.hits.total'
```

Zero results + the name matches a known AI product → close with hallucination comment.

### Tables that require pre-gate verification

These are the common hallucination vectors. Any new entry must be MetaCPAN-verified:

- `WebFrameworkKind` enum (semantic-analyzer)
- `IMPLICIT_STRICT_MODULES` list
- `IMPLICIT_EXPORT_SKIP_LIST`
- `COMMON_MODULES_TIER_1` (completion)
- `PERL_SOURCE_EXTENSIONS`
- `try_extract_web_route_declaration` framework dispatch
- `detect_framework` / `update_framework_context` (class-model aliasing)

### Alias-hallucination (Moo-family / Dancer / Mojolicious)

A subtler variant: Codex adds code that treats `X` as an alias of `Moo` / `Dancer` / `Mojolicious`. The cue phrase in the PR body is usually `Treat X and X::Role as aliases for the <RealFramework> family`.

Detection: file-path grep for PRs touching `class_model.rs` + `frameworks_moo.rs` (or sibling framework detector files). High-precision — no keyword matching needed.

## Recommended closure comment

When closing a hallucinated framework-detection PR:

> Closing as a Codex hallucination. **`<Name>`** is `<what it actually is — AI editor / agent / tool>`, not a Perl framework or module on CPAN. This PR adds `<mechanism>` for a name that has no Perl-ecosystem presence. If the intent was integration with the **`<real product>`** editor, that belongs in `docs/EDITORS/<NAME>_SETUP.md` (no code changes needed — perl-lsp works with any LSP-compliant client).
>
> Same Codex task produced similar hallucinations: `<list closed siblings>`. Recommend research-verifier pre-check on any future PR adding entries to `WebFrameworkKind` / `IMPLICIT_STRICT_MODULES` / `COMMON_MODULES_TIER_1` / `PERL_SOURCE_EXTENSIONS`.

## Distinguishing legitimate editor-integration docs from hallucinated framework support

**Legitimate:** `docs(editors): add <Product> setup guide` where `<Product>` is a real editor/tool with LSP support (Trae, Kiro, Zed, PearAI, Eclipse, Windsurf, Cursor, Codex CLI, Factory Droid, Hermes IDE, etc.). These PRs touch `docs/EDITORS/` and `README.md` Quick Start sections.

**Hallucinated:** `feat(semantic-analyzer): add <Name> framework detection` where `<Name>` is the same editor/tool being treated as a Perl framework. These PRs touch `crates/perl-semantic-analyzer/` or `crates/perl-parser-core/`.

The fifty-character tell: if a PR touches both `docs/EDITORS/` AND a semantic-analyzer crate, it's probably not a clean split.

## When standards passes a hallucination, what broke

Standards-review passed the hallucinated PRs because its check list doesn't include "is the thing being supported a real thing in the ecosystem we support". The fix is to add a MetaCPAN step to the reviewer agent workflow for the specific table-modification patterns above. This is a narrow enough signal (single grep for specific enum / list names) that the cost is low.

## Related memory

- `feedback_codex_framework_hallucination.md` — the original memory entry
- `feedback_broad_scope_codex_stack_diversity.md` — a neighboring pattern (broad scopes produce layer-diversity, not this hallucination)
- `feedback_codex_ensemble_pattern.md` — the normal 4-shot pattern where each PR is a useful design variant (this is the FAILURE mode of that pattern)
