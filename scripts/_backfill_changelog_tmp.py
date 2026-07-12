from __future__ import annotations

from pathlib import Path
import re

PATH = Path("CHANGELOG.md")
TEXT = PATH.read_text(encoding="utf-8")
MARKER = "<!-- changelog-backfill:0.15.1-0.17.0 -->"

if MARKER in TEXT:
    raise SystemExit("changelog backfill already applied")


def take_section(text: str, heading: str, next_heading: str) -> tuple[str, str, str]:
    start = text.index(heading)
    end = text.index(next_heading, start)
    return text[:start], text[start:end], text[end:]


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


# ---------------------------------------------------------------------------
# 0.17.0 — add missing user-visible work and explain the delayed ancestry import.
# ---------------------------------------------------------------------------
prefix, sec17, remainder = take_section(
    TEXT,
    "## [0.17.0] - 2026-06-28",
    "## [0.16.0] - 2026-06-06",
)

sec17 = replace_once(
    sec17,
    "## [0.17.0] - 2026-06-28\n\n",
    """## [0.17.0] - 2026-06-28

Release notes: [v0.17.0](docs/releases/v0.17.0.md)

> **Release-history note.** The 0.17 source sync was a history-preserving,
> two-parent complete-tree merge from `perl-lsp-swarm` RC `c04e06b8c`. The
> source comparison `v0.16.0...v0.17.0` is inflated because this merge also
> connected the logical commits that produced 0.16 but were not ancestors of
> `v0.16.0`. Use swarm first-parent range `a87f766ab...c04e06b8c` for logical
> 0.17 accounting and the source comparison for final-tree verification.
>
> """ + MARKER + """

""",
    "0.17 provenance",
)

added17 = """
#### Navigation, completion, rename, and folding

- **Method-call parameter inlay hints.** In-file and workspace-resolved OO calls
  can display parameter names after dropping the implicit invocant. Unknown
  methods and one-visible-parameter calls remain quiet to avoid noise.
  (perl-lsp-swarm#1311)
- **Native `class` and `method` navigation.** Hover and go-to-definition now
  traverse native class/package bodies, resolve method declarations, and locate
  class definitions used by calls such as `MyClass->new`.
  (perl-lsp-swarm#1223)
- **Cross-package rename promoted.** Safe package rename plans can include exact
  definition, reference, import-list, and export-list edits. Dynamic boundaries,
  ambiguity, and empty plans still fail closed. (perl-lsp-swarm#1465)
- **Workspace completion auto-imports.** Eligible unimported subroutine,
  variable, and constant completions can attach an `additionalTextEdits`
  insertion for `use Module;`, extending the existing method-completion path.
  (perl-lsp-swarm#1919)
- **Nested `#region` folding.** VS Code-style `#region` / `#endregion` markers,
  including spaced and nested forms, now produce folding ranges; unmatched
  markers are ignored. (perl-lsp-swarm#1862)
"""
sec17 = replace_once(sec17, "\n### Fixed\n", added17 + "\n### Fixed\n", "0.17 added backfill")

dap17 = """
- **No-session `stackTrace` returns no frames.** The adapter no longer fabricates
  an example `main` frame when no debug session exists. (perl-lsp-swarm#1212)
- **Full stack depth survives pagination.** `totalFrames` is captured before the
  requested page is sliced, so clients receive the real total rather than the
  current page length. (perl-lsp-swarm#1237)
- **Large frame and variable-reference IDs fail safely.** Unchecked `i64` to
  `i32` truncation and overflow-prone reference arithmetic are rejected instead
  of wrapping into another reference space. (perl-lsp-swarm#1218)
- **Evaluate validates the selected frame.** Invalid, stale, or non-stopped
  `frameId` values no longer silently evaluate in a default scope.
  (perl-lsp-swarm#1246)
"""
sec17 = replace_once(sec17, "\n#### Editor settings\n", "\n" + dap17 + "\n#### Editor settings\n", "0.17 DAP backfill")

lsp17 = """
- **Multi-root completion isolation.** Non-module workspace completions are
  limited to the workspace folder that owns the current document, preventing
  symbols from another root from leaking into the result set.
  (perl-lsp-swarm#1217)
- **Qualified-variable completions carry their replacement edit.** The computed
  `textEdit`, including UTF-16 coordinates, is now serialized so strict clients
  replace the typed prefix instead of appending a duplicate qualified name.
  (perl-lsp-swarm#2603)
- **Workspace-folder change notifications are advertised truthfully.** The
  server now declares its actual `changeNotifications` behavior instead of
  conditioning it on an unrelated client capability. (perl-lsp-swarm#1855)
"""
sec17 = replace_once(sec17, "\n#### Formatting\n", "\n" + lsp17 + "\n#### Formatting\n", "0.17 LSP backfill")

parser17 = """
- **`print` with subscripts is parsed as an expression.** `print $hash{key}` and
  `print $array[index]` are no longer mistaken for indirect-object or filehandle
  syntax. (perl-lsp-swarm#1214)
- **Delimiter recovery resynchronizes at the next statement.** An unclosed
  delimiter no longer necessarily swallows later top-level declarations; the
  parser can recover at the following statement boundary.
  (perl-lsp-swarm#1456)
"""
sec17 = replace_once(sec17, "\n#### Module resolution\n", "\n" + parser17 + "\n#### Module resolution\n", "0.17 parser backfill")

notes17 = """
### Notes

- HIR/PIR, semantic snapshots, shadow providers, and provider-comparison work in
  this release are compiler and verification substrate unless a live provider
  cutover is stated explicitly. They should not be read as broad editor-visible
  provider replacements.
- The 0.17 source tag range includes the delayed import of 0.16 logical history;
  raw commit counts from that range are not a valid measure of 0.17 scope.

"""
sec17 = replace_once(sec17, "\n---\n\n### Under the hood", "\n" + notes17 + "---\n\n### Under the hood", "0.17 notes")

TEXT = prefix + sec17 + remainder

# ---------------------------------------------------------------------------
# 0.16.0 — reconstruct the broad product delta hidden by the snapshot sync.
# ---------------------------------------------------------------------------
prefix, sec16, remainder = take_section(
    TEXT,
    "## [0.16.0] - 2026-06-06",
    "## [0.15.2] - 2026-05-26",
)

provenance16 = """
> **Historical provenance correction.** The release tree was promoted from
> `perl-lsp-swarm` RC `a87f766ab` through source commit `6925335f` as a
> one-parent content-state mirror. The tree is correct, but the individual swarm
> squash commits are not ancestors of `v0.16.0`; they became reachable through
> the later 0.17 history-preserving sync. Treat `v0.15.2...v0.16.0` as a tree
> comparison, not the complete logical-commit ledger. The logical development
> boundary is `151c5ecee...a87f766ab` in `perl-lsp-swarm`.

"""
sec16 = replace_once(sec16, "\n### Added\n", "\n" + provenance16 + "### Added\n", "0.16 provenance")

# The command and registration behavior were introduced in 0.15.1. Remove the
# duplicate attribution and replace the registration bullet with the actual
# 0.16 contribution: contract hardening.
sec16 = replace_once(
    sec16,
    """- **`perl.explainProviderDecision` execute-command** — Returns the structured
  provider decision explanation payload; reports a low-confidence fallback when
  no provider-specific receipt is attached.
""",
    "",
    "0.16 duplicate provider decision",
)
sec16 = replace_once(
    sec16,
    """- **Inline completion LSP 3.18 registration** — Static clients receive top-level
  `inlineCompletionProvider`; dynamic-capable clients receive
  `client/registerCapability`; `experimental.inlineCompletionProvider` is never
  emitted.
""",
    """- **Inline-completion registration contract hardened.** The standards-based
  static/dynamic registration introduced in 0.15.1 is retained and protected by
  negative-contract tests so the obsolete experimental capability cannot
  reappear.
""",
    "0.16 registration reattribution",
)

added16 = """
#### Project-aware deterministic inline completion

- **Declared-variable test assertions.** Test::More and Test2 files can suggest
  conservative `ok(...)` and `is(...)` statements using visible declared
  scalars rather than placeholder names. Suggestions remain import-gated.
  (perl-lsp-swarm#497)
- **Current-package receiver methods.** Self-like receiver completion uses
  methods declared in the current package and supplies safe partial-fragment
  replacement ranges. (perl-lsp-swarm#504)
- **Style-aware constructors.** Constructor bodies follow the local file style:
  signatures, `@_` extraction, or `shift`-based code.
  (perl-lsp-swarm#523)
- **Workspace module completion.** `use` and `require` ghost text can suggest
  reachable project modules from effective include roots and the live workspace
  index. (perl-lsp-swarm#529, perl-lsp-swarm#564)
- **Context-derived candidates.** DBI database/statement receivers, visible
  arrays and hashes for loop bindings, partial return variables, and visible
  scalars in guard conditions replace generic placeholders where evidence is
  available. (perl-lsp-swarm#536, #540, #542, #729, #736)
- **Confidence and parse-safety ranking.** Candidate ranking incorporates source
  confidence, and candidates that clearly worsen parser recovery are rejected.
  (perl-lsp-swarm#513, perl-lsp-swarm#517)

#### Code actions and CodeLens

- **PL405 `printf`/`sprintf` arity repair.** A guarded quick fix inserts missing
  `undef` arguments for supported call shapes and fails closed on unsafe ranges.
  (perl-lsp-swarm#430)
- **Recognized-function import repair.** Undefined bareword diagnostics can
  offer imports for recognized common functions, including JSON and
  File::Basename helpers. (perl-lsp-swarm#856)
- **PL410 undefined loop-label repair.** Quick fixes remove the bad label while
  preserving `next`, `last`, or `redo`. (perl-lsp-swarm#1172)
- **CodeLens resolve and tooltips.** Clients advertising command resolve support
  can receive lazy command lenses; unsupported clients retain eager commands.
  Command lenses also carry deterministic plain-text tooltips.
  (perl-lsp-swarm#500, perl-lsp-swarm#514)

#### Capability-gated LSP 3.18 behavior

- Supporting clients can receive `CompletionList.itemDefaults.data`,
  `CompletionList.applyKind.data`, static code-action documentation,
  snippet-aware current-document pragma edits, applyEdit metadata, and lazy
  CodeLens command resolution. Unsupported clients retain compatible fallback
  shapes. (perl-lsp-swarm#525, #530, #534, #592, #657)

#### OpenAI-compatible connector configuration

- Inline AI connectors can configure the API-key header name and prefix rather
  than being forced to use a Bearer `Authorization` header. Header names,
  prefixes, and key content are validated before request construction.
  (perl-lsp-swarm#739)
"""
sec16 = replace_once(sec16, "\n### Fixed\n", "\n" + added16 + "\n### Fixed\n", "0.16 added backfill")

fixed16 = """
#### Parser and lexer coverage

- Scalar filehandle readline (`<$fh>`) is classified as `Readline`, not glob.
- Native `class` declarations accept optional versions.
- Parenthesized builtins bind correctly before following ternaries.
- Unbraced dereferences (`$$ref`, `@$ref`, `%$ref`) produce dereference nodes.
- Perl 5.22+ double-diamond `<<>>` parses.
- Signature defaults accept calls, binary expressions, and ternaries.
- Assignment to lvalue builtins such as `pos`, `substr`, and `vec` is preserved.
- Lowercase Perl 5.32+ `isa` is recognized as an infix operator.
- Methods with signatures accept trailing attributes.
- Empty-search `tr///` character-count forms are accepted.
- Three-part version directives such as `use 5.10.1;` are parsed correctly.
- Unknown bare calls whose arguments combine builtins and concatenation bind
  correctly.
- Methods named after quote operators (`method y`, `method s`, `method tr`) lex
  as declarations in method context.
- Parenthesized variable-attribute arguments are retained.
- Parser error snippets are sliced only on UTF-8 character boundaries.

Representative work: perl-lsp-swarm#708, #711, #715, #725, #744, #754, #759,
#762, #770, #771, #775, #777, #784, #801, and #802.

#### Debug adapter behavior

- **Current-stop stack selection.** Framed debugger output and the session's
  latest authoritative stack state take precedence over historical output that
  could surface an earlier stop.
- **Lexical locals inspection.** Local variables are read from the active Perl
  frame's pad through the `B` module instead of being approximated through the
  package symbol table.
"""
sec16 = replace_once(sec16, "\n### Documentation\n", "\n" + fixed16 + "\n### Internal / claim boundaries\n\n- **Next-edit remained disabled.** The release added configuration, safety, and\n  receipt scaffolding for future deterministic next-edit families, but did not\n  register a provider or emit editor-visible next-edit suggestions.\n- RIPR, Patch-95 routing, and smoke receipts validate the release; they are not\n  additional product features.\n\n### Documentation\n", "0.16 fixed and boundary backfill")

TEXT = prefix + sec16 + remainder

# ---------------------------------------------------------------------------
# 0.15.1 — make the latency rail and broken Cargo package explicit.
# ---------------------------------------------------------------------------
prefix, sec151, remainder = take_section(
    TEXT,
    "## [0.15.1] - 2026-05-26",
    "## [0.15.0] - 2026-05-22",
)

sec151 = replace_once(
    sec151,
    """- **Neovim / lean-editor latency rail** — `--runtime-mode e2e` /
  `PERL_LSP_E2E=1` profile: zero diagnostic debounce, syntax-only diagnostics,
  no eager workspace indexing, no file watchers by default.
""",
    """- **Neovim / lean-editor workload profile** — `--runtime-mode e2e` /
  `PERL_LSP_E2E=1` defaults to zero diagnostic debounce, syntax-only diagnostics,
  no eager workspace indexing, and no file watchers. Explicit flags can restore
  individual behaviors without changing the advertised feature surface.
- **Syntax-only push and pull diagnostics** — `--diagnostic-mode syntax-only` /
  `PERL_LSP_DIAGNOSTIC_MODE=syntax-only` limits both paths to parser errors and
  skips semantic, Perl::Critic, module-resolution, and workspace dead-code work.
- **Configurable diagnostic debounce** — `--diagnostic-debounce-ms <ms>` /
  `PERL_LSP_DIAGNOSTIC_DEBOUNCE_MS=<ms>` controls the publish window; `0`
  bypasses the debouncer for latency-focused harnesses.
- **Eager-index suppression in E2E mode** — `initialized` does not start a
  workspace scan unless `--eager-workspace-indexing=true` explicitly restores it.
""",
    "0.15.1 latency expansion",
)

install_note = """
- **Install-surface caveat.** The crates.io `perl-lsp-rs-core` 0.15.1 package
  omitted `build_catalog.rs`, so `cargo install perllsp --version 0.15.1` and
  `cargo install perl-dap --version 0.15.1` failed. Source/tag behavior is
  unaffected, but Cargo users should install 0.15.2 or later.
"""
sec151 = replace_once(
    sec151,
    """- This release does not implement true incremental AST reuse. Latency
  improvements come from skipping avoidable background work and cancelling
  stale reads earlier.
""",
    """- This release does not implement true incremental AST reuse. Latency
  improvements come from skipping avoidable background work and cancelling
  stale reads earlier.
- Raw-RPC and Neovim receipts prove wiring and behavior boundaries, not a
  hardware-independent wall-clock latency guarantee.
""" + install_note,
    "0.15.1 notes expansion",
)

TEXT = prefix + sec151 + remainder

# Final invariants: only the intended recent sections changed, all anchors remain,
# and the backfill is idempotently marked.
for heading in (
    "## [0.17.0] - 2026-06-28",
    "## [0.16.0] - 2026-06-06",
    "## [0.15.2] - 2026-05-26",
    "## [0.15.1] - 2026-05-26",
    "## [0.15.0] - 2026-05-22",
):
    if TEXT.count(heading) != 1:
        raise RuntimeError(f"unexpected heading count for {heading}")

if TEXT.count(MARKER) != 1:
    raise RuntimeError("backfill marker missing or duplicated")
if "## [0.14.0] - 2026-05-12" not in TEXT:
    raise RuntimeError("older changelog history was lost")

PATH.write_text(TEXT, encoding="utf-8")
print("Backfilled CHANGELOG.md for 0.15.1 through 0.17.0")
