# Deep review — PR #3313: `perl-workspace-core` + `perl-tree-sitter-compat`

**Reviewer**: reviewer-deep (correctness gate)
**Base**: `main` @ `520740d5` (PR opened against; current `main` tip has since advanced with unrelated commits)
**PR tip reviewed**: `4f75ad45ab02d8180f37b3d05ab3e6737332c31f`
**HEAD after fixes**: `5847f49` on `claude/perl-workspace-core-design-g2bptn`

## Scope

Two additive crates: `perl-workspace-core` (LSP-free project-facts substrate,
11/11 fact classes) and `perl-tree-sitter-compat` (first native consumer —
tree-sitter-shaped node/sexp/highlight adapter). Reviewed every producer file
end to end: `id.rs`, `range.rs`, `provenance.rs`, `fact_classes.rs`,
`file.rs`, `symbol.rs`, `package.rs`, `import.rs`, `import_walk.rs`,
`export.rs`, `effects.rs`, `boundary.rs`, `dist.rs`, `test.rs`, `pod.rs`,
`relation.rs`, `model.rs`, `builder.rs`, `error.rs`, `lib.rs`, plus the full
`perl-tree-sitter-compat` crate (`node.rs`, `convert.rs`, `sexp.rs`,
`highlight.rs`).

## What I verified

- `cargo fmt --check -p perl-workspace-core` and `-p perl-tree-sitter-compat`
  (separately, per CLAUDE.md) — clean, both before and after fixes.
- `cargo clippy -p perl-workspace-core --locked --all-targets -- -D warnings
  -A missing_docs` and same for `perl-tree-sitter-compat` — clean.
- `cargo test -p perl-workspace-core -p perl-tree-sitter-compat --locked` —
  101 tests total (79 unit in perl-workspace-core + 22 integration/doc, 12
  unit + 5 adapter + 1 doc in perl-tree-sitter-compat), all green after
  fixes.
- `cargo check --locked --workspace` — clean; `Cargo.lock` is current.
- `git diff origin/main -- crates/perl-kwalitee` — empty, both before and
  after my push. The #3309 collision was resolved correctly; the rebuilt
  branch does not touch `perl-kwalitee`.
- Root `Cargo.toml` registers exactly the two new crates as members + deps;
  `perl-kwalitee`'s entry from #3309 is untouched.

## Defects found and fixed (fix-forward, pushed to the PR branch)

1. **`pod.rs::scan_sections` — `=cut` prefix-match bug.** Used
   `trimmed.starts_with("=cut")` to detect the POD-closing directive. Real
   Perl parses the directive name as the first whitespace-delimited token, so
   an unrelated/unknown directive merely *prefixed* by "cut" — e.g.
   `=cutlery`, `=customs` — would be misclassified as closing the POD block,
   silently truncating everything after it (lost `documented_methods`,
   `sections`, `description`). Fixed with a token-exact `is_cut_directive()`
   helper, applied symmetrically to both the block-open guard and the
   block-close check. Added a direct unit test
   (`is_cut_directive_matches_exact_token_only`) and an integration-shaped
   regression (`cut_lookalike_directive_does_not_close_pod`) proving a
   `=cutlery` lookalike inside a real POD block no longer truncates the
   `=head2 run` / `documented_methods` that follow it.

2. **`dist.rs::CPANFILE_KEYWORDS` — missing `conflicts` keyword (found by a
   parallel Codex pass, verified independently before applying).**
   `RELATIONS` (used for META.json parsing) lists `conflicts` as a
   recognized prereq relation, but `CPANFILE_KEYWORDS` (used for cpanfile
   statement parsing) had no entry for it — `conflicts 'Foo';` in a cpanfile
   was silently dropped (`handle_cpanfile_statement`'s `find()` returns
   `None`, statement discarded). Verified against the actual const tables
   before fixing. Added `("conflicts", "conflicts", "runtime")` and a
   regression test (`cpanfile_conflicts_is_recognized`).

3. **`builder.rs::is_indexable` — extensionless scripts invisible (found by
   the same parallel Codex pass, verified independently before applying).**
   `is_indexable` only recognized known Perl extensions (`pm`/`pl`/`t`/
   `pod`/`psgi`) plus known metadata filenames. A distribution's `bin/app` or
   `script/tool` executable shipped with no extension — a common CPAN
   packaging pattern — was silently invisible to the substrate at the
   `collect_perl_files` stage, even though `FileRole::from_path` already
   classifies such a path as `Script` once it gets that far. Verified the
   gap is real by reading both functions side by side. Fixed with a
   deliberately tight `is_shebang_perl_script()` check: only an
   **extensionless** file whose first line is a real Perl shebang
   (`#!.../perl` or `#!.../env perl`, read via a bounded 256-byte prefix, no
   full-file read, never panics on I/O/encoding failure) is indexed — a
   non-Perl extensionless file (shell script, README, …) stays out. Added
   positive tests for both shebang forms and a negative test for a `#!/bin/sh`
   script.

## Coverage gaps closed (Codecov `Patch 95` risk — `--lib` only)

Found and closed with cheap `--lib` unit tests, no behavior change:

- `builder.rs::visibility_of` — only the `None` (public, no declarator)
  branch had test coverage; the `my`/`state` → Private, `our`/`local` →
  Public, and `Some(_)` → Unknown branches were untested new lines. Added
  `visibility_of_maps_declarators`.
- `builder.rs`'s read-failure limitation path (`std::fs::read_to_string`
  `Err` arm) had no test — the doc comment explicitly claims "never silently
  drop," but nothing proved it. Added
  `unreadable_file_records_limitation_not_silent_drop`, using invalid UTF-8
  content as a portable trigger (permission bits don't block root, which
  these sandboxes run as; a malformed-encoding file fails
  `read_to_string` regardless of privilege).
- `id.rs`'s `Display` impls for `FileId`/`PackageId`/`SymbolId` and
  `convert.rs`'s `Display` impl for `TreeError` were one-line `fmt::Display`
  bodies with no direct test exercising the `{}`/`.to_string()` path (only
  `Digest`'s and `WorkspaceCoreError`'s were covered). Added
  `id_display_impls_match_as_str` and `tree_error_displays_readably`.

## Things I checked and found correct (no change needed)

- **`import_walk.rs` block-scoped package snapshot/restore**: `Block`
  handling snapshots `current_package`, recurses, then restores — verified
  against the existing `statement_package_in_nested_block_does_not_leak_context`
  test and by hand-tracing `package Outer; { package Inner; } use base 'Role';`.
  Correct: `use base` after the block attributes to `Outer`, not `Inner`.
- **`import_walk.rs` pragma vs module classification**: `looks_like_pragma`
  (lowercase first char) is a reasonable heuristic; `is_pragma` correctly
  gates the RELATIONS `Uses`/`Tests` edge synthesis so pragmas never become
  module-load edges.
- **`relation.rs` per-package parent scoping**: `synthesize_relations` builds
  `Inherits` edges from `parents_by_package` (already correctly scoped by the
  import walk) and `Uses`/`Tests` edges only for non-pragma `Use`/`Require`
  imports — correctly excludes pragmas including `parent`/`base` (already
  captured as `Inherits`).
- **`dist.rs` META.json absent vs unparseable**: `parse_meta_json` returns
  `None` only on JSON parse failure (`invalid_json_returns_none` test);
  `builder.rs::extract_dist_metadata` only calls it when the file is present
  and named `META.json` — a missing file never reaches this code at all (no
  entry in `collect_perl_files`'s output), and a present-but-corrupt file
  correctly yields no `DistMetadataFacts` (not a fabricated empty one) while
  the `FileRecord` for the corrupt file still exists with role
  `DistMetadata`. No silent-success-on-garbage path found.
  `cpanfile`'s `parse_cpanfile` never returns `None` (a cpanfile with no
  recognized statements just yields empty `prereqs`) — this asymmetry is
  intentional and documented (cpanfile has no name/version/license fields to
  fail to parse).
- **`builder.rs` fact-class gating**: traced every fact vector's population
  point against `FactClasses` bit checks. A class not requested truly
  produces nothing: `wants_parse` gates whether `extract_facts` runs at all;
  within `extract_facts`, `wants_imports`/`wants_effects` gate the shared AST
  walk, and each individual fact vector (`imports`, `exports`, `relations`,
  `dynamic_boundaries`, `symbols`, `tests`, `compile_effects`) is
  individually gated before extending the model — confirmed no fact class
  can leak into the model when unrequested, and `every_fact_class_has_a_producer`
  / `full_coverage_no_unimplemented_limitations` back this with a request for
  `FactClasses::all()` and assert no `unimplemented_fact_class` limitation.
- **`fact_classes.rs`**: hand-rolled bitset (`contains`/`intersects`/`union`)
  is correct bit arithmetic; `all()` matches the 11 documented classes 1:1.
- **`id.rs` FNV-1a identity**: NUL-separated field hashing correctly
  prevents field-boundary collisions (`["a","bc"]` vs `["ab","c"]` — tested);
  `SymbolId` includes the span so re-declaration doesn't collide; digest/ID
  derivation is host-path-free and deterministic (checked the dependency
  contract test enforces no `lsp-types`/`tokio`/`perl-lsp-*`/`perl-dap`/
  `perl-workspace ` (trailing space, correctly scoped to not also match
  `perl-workspace-core`) in the crate's dependency tree — this test ran and
  passed, not skipped, since `cargo tree` succeeded in this environment).
- **`range.rs` `Utf8LineIndex`**: correctly handles `\n`/`\r\n`/lone `\r`;
  byte-column (not codepoint) counting verified with a 2-byte UTF-8
  character (`é`); out-of-range offsets clamp rather than panic.
- **`perl-tree-sitter-compat::node::pascal_to_snake`**: correct for every
  `NodeKind` variant name that actually exists in `perl-ast` today (checked
  the full `kind_name()` match arm list — no variant has adjacent uppercase
  letters, e.g. no `XSCall`/`PODSection`-shaped acronym). **Residual latent
  risk, not blocking**: the function inserts an underscore before *every*
  uppercase letter, so a future `NodeKind` variant with an acronym (e.g. a
  hypothetical `XSCall`) would snake_case to `x_s_call` instead of
  `xs_call`. Not exploitable today; flagged for whoever adds such a variant
  next, not worth a speculative fix now (YAGNI — no such variant exists, and
  a fix without a real case to test against risks being wrong in a different
  way).
- **`test.rs` framework/assertion detection**: correctly distinguishes
  `done_testing;` (bareword `Identifier`) from `done_testing();`
  (`FunctionCall`) per the documented parser quirk; only the `FunctionCall`
  form increments `assertion_count` (intentional, not a bug — the `Identifier`
  form still sets `has_plan`).
- **`builder.rs` symlink handling**: `file_type.is_symlink()` check (not
  `path.is_dir()`, which would follow the link) correctly prevents a
  circular directory symlink from infinite-looping — verified via the
  existing `circular_directory_symlink_does_not_recurse_forever` test, which
  passed.

## Verdict

**Fixes pushed to the PR branch** (fast-forward `4f75ad4` → `5847f49`).
1 correctness bug found and fixed by me (`=cut` prefix match), 2 additional
correctness bugs from a parallel Codex pass verified independently and fixed
before applying (cpanfile `conflicts`, extensionless shebang scripts), and 5
coverage gaps closed with cheap `--lib` unit tests (no behavior change) to
reduce `Codecov / Patch 95` risk. All gates re-verified green after the
fixes: fmt (both crates), clippy (both crates, `-D warnings`), full test
suite (101 tests), `cargo check --locked --workspace`, and the
`perl-kwalitee` non-collision invariant.

**Residual risk for downstream gates**:
- `Codecov / Patch 95` — I closed the coverage gaps I found by direct
  reading (visibility branches, read-failure path, Display impls). I did not
  run an actual coverage tool (`cargo llvm-cov` / `cargo tarpaulin`) in this
  environment, so there may be additional uncovered branches I didn't spot by
  inspection alone (e.g. `collect_perl_files`'s `read_dir` `Err` branch,
  `parse_on_phase`'s bareword-phase fallback path). green-ci or a dedicated
  coverage pass should confirm the actual Patch 95 number post-push.
- `ripr+ New Gap Gate` — not evaluated by this reviewer; this PR makes no
  runtime behavior change to shipped binaries (additive crates only,
  confirmed by the PR's own claim boundary and by `perl-kwalitee`'s empty
  diff), so I'd expect this gate to be a formality, but it should still run
  fresh on the new HEAD SHA `5847f49`.
- I did **not** re-verify the three-way #3312 staged-rollout framing (PRs 5,
  6, 8 as documented follow-ups) — that's a project-alignment question for
  maintainer-pr, not a correctness question for this pass.

**Label action**: applying `deep-reviewed` only, per instructions. Not
setting `merge-ready` — that is the orchestrator's call after `ci-green` and
`diff-audited` receipts also land on HEAD `5847f49`.
