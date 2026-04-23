# `agents4` Canonical Q3 Archaeology
## Why `agents4` Is The Best Preserved Perl-LSP-Native Q3 Swarm Surface

The historical value of `agents4` is not just that it exists after `agents3`.

`agents4` is where the repo's Q3 swarm stops reading like a donor or
transitional control-plane layer and starts reading like a perl-lsp-native
three-phase operating surface.

That makes it the clearest preserved form of the Q3 Claude Code swarm as it
actually fit this repository.

---

## 1. `agents4` Specializes The Three-Phase Swarm For Perl LSP

The three-phase structure is already familiar elsewhere in the archaeology:

- `generative/`
- `review/`
- `integration/`

with matching top-level flow files:

- `issue-to-draft`
- `draft-to-pr`
- `pr-to-merge`

What makes `agents4` special is the degree of perl-lsp-specific specialization
inside that structure.

[`.claude/agents4/issue-to-draft.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/issue-to-draft.md)
anchors the generative phase in repo-native concerns:

- Perl parsing feature specifications
- parser / lexer / LSP validation
- cross-file navigation
- incremental parsing efficiency
- Tree-sitter highlight testing
- adaptive threading for CI environments

Its evidence language is also clearly repo-native rather than donor-native:

```text
tests: cargo test: 295/295 pass; parser: 180/180, lsp: 85/85, lexer: 30/30
parsing: ~100% Perl syntax coverage; incremental: <1ms updates with 70-99% node reuse
lsp: ~89% features functional; workspace navigation: 98% reference coverage
benchmarks: parsing: 1-150μs per file
```

That is not generic flow doctrine. It is a perl-lsp proof contract.

---

## 2. The September 24 Refactor Is The Key Specialization Moment

The strongest history signal is commit `46196e37d` on `2025-09-24`:

`refactor: Update Integrative Flow for Perl LSP production readiness`

The commit summary is unusually explicit about what changed:

- replace MergeCode quality compliance with Perl LSP production readiness
- reflect parsing SLO validation
- focus agents on Perl LSP performance metrics
- align check runs and gates with parsing performance standards
- emphasize LSP protocol compliance and parsing performance

That commit heavily rewrites the three top-level flow files in `agents4`.

Historically, that matters more than the original import. It shows the repo not
just copying a flow pack, but rewriting the swarm around its own parser, LSP,
and release-readiness concerns.

---

## 3. `agents4` Encodes Phase-Specific Evidence, Not Just Staging

The flow files in `agents4` do more than split work into stages.

They distinguish which kinds of proof belong in which stage.

In
[`.claude/agents4/issue-to-draft.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/issue-to-draft.md),
the gate evolution table says:

- Generative establishes `benchmarks`
- Review validates `perf`
- Integrative validates `throughput`

It also expands the gate vocabulary around repo-specific needs:

- `parsing`
- `lsp`
- `highlight`

in addition to the more general:

- `spec`
- `format`
- `clippy`
- `tests`
- `build`
- `docs`
- `mutation`
- `fuzz`
- `security`

That is a useful historical distinction. `agents4` is not only a staged PR
pipeline. It is a staged evidence pipeline for this codebase.

---

## 4. The Customizer Layer Makes The Adaptation Explicit

[`.claude/agents4/agent-customizer-generative.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/agent-customizer-generative.md)
spells out the specialization directly.

It tells subagents to adapt generic workflows to:

- the `docs/` surface and Diataxis structure
- the `perl-parser`, `perl-lsp`, `perl-lexer`, `perl-corpus`, and `xtask`
  workspace layout
- zero-warning Rust workspace validation
- highlight testing and LSP protocol validation
- parser accuracy and incremental parsing efficiency

That means `agents4` is not just incidentally more specific than `agents3`.
The pack understands itself as a specialization layer.

This is one reason it reads as canonical rather than transitional.

---

## 5. The Pack Stayed Live Beyond The Initial Q3 Burst

`agents4` also stayed in use long enough to keep receiving meaningful edits
after its initial late-September landing.

Local git history for core `agents4` files shows:

- `104bdc17e` on `2025-09-23` introduces the tracked pack
- `46196e37d` on `2025-09-24` specializes it for Perl LSP production readiness
- `58ce94542` on `2025-09-27` continues editing `agents4` during the PR `#170`
  executeCommand wave
- `7f5b5290d` on `2026-02-20` still adjusts `agents4` during `v0.9.1`
  public-alpha release alignment

That persistence matters historically.

It means `agents4` was not just a one-day experiment. It remained part of the
repo's live operating vocabulary long enough to absorb release-truth and
quality-surface changes months later.

---

## 6. Why `agents4` Matters Alongside `agents3`

The repo preserves both `agents3` and `agents4`, but they tell different parts
of the story.

- `agents3` shows donor or transplant lineage
- `agents4` shows what the swarm looks like once rewritten around perl-lsp
  reality

That is why the two notes belong together:

- [MERGECODE_ROOTS_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/MERGECODE_ROOTS_ARCHAEOLOGY.md)
  explains where some of the doctrine came from
- this note explains why `agents4` is the best preserved canonical Q3 swarm
  surface for this repo itself

In short:

- `agents3` is lineage
- `agents4` is local embodiment

---

## Evidence Pointers

- [`.claude/agents4/issue-to-draft.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/issue-to-draft.md)
- [`.claude/agents4/draft-to-pr.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/draft-to-pr.md)
- [`.claude/agents4/pr-to-merge.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/pr-to-merge.md)
- [`.claude/agents4/agent-customizer-generative.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/agent-customizer-generative.md)
- [CONTROL_PLANE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md)
- [Q3_SWARM_PR_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md)
- [MERGECODE_ROOTS_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/MERGECODE_ROOTS_ARCHAEOLOGY.md)
- commits `104bdc17e`, `46196e37d`, `58ce94542`, `7f5b5290d`
