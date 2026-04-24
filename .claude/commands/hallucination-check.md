---
description: Ensemble-curator step 2 — MetaCPAN / authoritative-source verification for framework-detection and module-import PRs
---

# Hallucination Check

The most decisive close-with-confidence signal for external-agent PRs. When a PR adds a Perl module name or framework to a detection table, verify the name exists on CPAN. Zero hits + name matches AI product = HALLUCINATED.

See `memory/feedback_codex_framework_hallucination.md` and `docs/articles/CODEX_HALLUCINATION_TRIAGE.md`.

## When to run

Required for any PR touching these tables / functions:

- `WebFrameworkKind` enum
- `IMPLICIT_STRICT_MODULES` list
- `IMPLICIT_EXPORT_SKIP_LIST`
- `COMMON_MODULES_TIER_1`
- `PERL_SOURCE_EXTENSIONS`
- `detect_framework()` function (alias-hallucination check)
- `update_framework_context()` function
- Any `use <Name>;` / `use <Name>::Role` route-extraction patterns
- Any completion-suggestion additions

Trigger phrase in title: `feat(*semantic-analyzer*): add <Name> framework detection`, or
`feat(*parser-core*): add <Name> template extension support`, or
`feat(completion): prioritize <Name> module suggestions`.

## Quick check

```bash
# For each name being added:
NAME="<extracted-name>"
RESULT=$(curl -s "https://fastapi.metacpan.org/v1/module/_search?q=${NAME}&size=3" | jq -r '.hits.total')
echo "$NAME: $RESULT hits on CPAN"

# Alias-hallucination subvariant (Moo::Role pattern):
RESULT_ROLE=$(curl -s "https://fastapi.metacpan.org/v1/module/_search?q=${NAME}::Role&size=3" | jq -r '.hits.total')
echo "$NAME::Role: $RESULT_ROLE hits"
```

## Interpreting

- `>0 hits`: real CPAN module. Continue with other checks; don't close on hallucination grounds.
- `0 hits` AND name matches known AI product (OpenClaw, Droid, Builder.io Fusion, Google::Antigravity, Hermes Agent, Factory Droid, Jules, Aider, Cursor, Claude, Codex, Warp, Perplexity, Grok, Anthropic, Replit, Fusion, Antigravity, Continue, Roo, Kilo, PearAI, Crush, OpenCode): **HALLUCINATED**.
- `0 hits` AND name is plausible-but-obscure: check WebFetch for the product site. If it's a known AI tool, **HALLUCINATED**. If it's a plausible niche Perl module, leave a comment asking for CPAN link and don't close.

## Known hallucinations from 2026-04-23 session

Already-closed (don't re-detect):

- OpenClaw, Droid, Droid::Factory
- Builder::IO::Fusion
- Google::Antigravity
- Hermes Agent (the Perl-framework variant; `Hermes IDE` is a real editor)
- `.mcp` as Mason extension (MCP protocol is real, but `.mcp` is not a Perl file extension; Mason uses `.mc`/`.mp`/`.mi`)
- UTF-7 / GB18030 / Windows-1252 as Perl `use encoding` pragma targets (these are real encodings but not pragma-registered as Codex claimed)

## Closure comment template

When HALLUCINATED:

> Closing as a Codex hallucination. **`<Name>`** is `<what it actually is — AI editor / agent / tool>`, not a Perl framework or module on CPAN (verified: 0 hits on MetaCPAN). This PR adds `<mechanism>` for a name that has no Perl-ecosystem presence.
>
> If the intent was to integrate with the **`<real product>`** tool as an editor/agent, that belongs in `docs/EDITORS/<NAME>_SETUP.md` (no code changes needed — perl-lsp works with any LSP-compliant client).
>
> Same Codex task produced similar hallucinations: `<list closed siblings>`. Recommend `/hallucination-check` + MetaCPAN pre-gate on any future PR adding entries to `WebFrameworkKind` / `IMPLICIT_STRICT_MODULES` / `COMMON_MODULES_TIER_1` / `PERL_SOURCE_EXTENSIONS`.

## What this skill outputs

Either:
- `HALLUCINATED: <name> 0 CPAN hits + matches AI product <product>` → emit close
- `REAL: <name> N CPAN hits` → continue to next check
- `UNCLEAR: <name> 0 hits but not a known AI product` → comment asking for reference, leave open

## False-positive prevention

Before emitting HALLUCINATED:

1. Check the PR is actually adding the name to a FRAMEWORK-DETECTION table, not elsewhere (e.g., docs PRs mentioning the name are fine).
2. Check you're not confusing `X` with `X::Subpackage` — search both.
3. Prefer over-investigate than over-close: 0 hits with a plausible enterprise-only name may be fine to leave open with a comment.

The cost of one over-close is low (reopen). The cost of shipping a hallucinated framework-detection is polluted code. Err toward closing when the name is unambiguously an AI product.
