# perl-lsp: Curiosities, Records, and Surprising Facts

A catalog of the interesting, extreme, and unexpected things discovered
while building an LSP server for one of programming's most
unparseable languages.

---

## By the Numbers

| Metric | Value |
|--------|-------|
| Total commits | 2,697 |
| Pull requests | 2,221+ |
| GitHub issues | 2,218+ |
| Rust source files | 1,530 |
| Lines of Rust | 547,852 |
| Workspace crates | 131 |
| LSP features defined | 97 |
| CI gates | 13 |
| Test corpus files | 80 |
| Tech debt markers | 49 |
| Lines added over project lifetime | ~2.97 million |

The project started on **2022-07-17** with a single commit:
`start tapping out grammar; statement + declaration`. Nine months of
quiet tinkering, then an explosion: 855 commits in March 2026 alone.

---

## Records and Extremes

### Busiest days by commit count

| Date | Commits |
|------|---------|
| 2026-03-04 | 152 |
| 2026-03-18 | 144 |
| 2026-03-16 | 121 |
| 2026-03-15 | 119 |
| 2025-07-16 | 96 |

Four of the five busiest days happened in a single two-week stretch in
March 2026, during the AI swarm sessions.

### Smallest crate: `perl-line-index` -- 44 lines

A single `lib.rs`, 44 lines of Rust. Converts byte offsets to line/column
positions. Tiny, correct, and depended on by dozens of other crates.

### Largest crate: `perl-lsp` -- 120,596 lines

The LSP server binary. 325 Rust source files, housing completion, hover,
diagnostics, semantic tokens, and every other LSP capability. This was
once even larger before the microcrate extraction campaign split out
providers like `perl-lsp-code-lens` and `perl-lsp-selection-range`.

### PR #209: The Largest Single Merge

248 files changed, 69,505 lines added, merged in one shot on
2025-10-09. It landed the entire Phase 1 DAP (Debug Adapter Protocol)
support, bridging to Perl::LanguageServer. The PR passed all 10 CI
gates on the first attempt.

### PR #2057: The 9-Line Wiring Fix

A 3-file, 433-line addition that wired existing lint checks into the
diagnostic pipeline. The infrastructure was already built -- it just
was not connected. This pattern ("built but not wired") turned out to
be the highest-ROI class of changes: scout for existing infrastructure
first, then connect it.

### The Commit That Touched 2,498 Files

A single commit implementing `executeCommand` with `perl.runCritic`
touched 2,498 files -- the most in project history.

---

## The Three Parser Story

perl-lsp has been through three complete parser implementations. The
folklore says "only Perl can parse Perl." This project tried to prove
that wrong -- three times.

**v1 -- C/tree-sitter (2022-2023)**
The original approach: write a tree-sitter grammar in C. Fast and
incremental, but tree-sitter's LR formalism could not handle Perl's
context-sensitive syntax. Kept in the repository for benchmarking
comparisons.

**v2 -- Pest PEG (2023)**
A parser-expression-grammar approach using the Pest library. More
expressive than LR, but PEGs are greedy and Perl's ambiguity defeated
the ordered-choice model. Archived; its crate directories still exist
but are excluded from the default build.

**v3 -- Native Recursive Descent (2024-present)**
The current parser. Hand-written recursive descent with explicit
context threading. This is the only approach flexible enough to handle
Perl's "the parser must know what the lexer saw" requirement. Now
parses 80%+ of CPAN cleanly.

All three parsers coexist in the repository, and benchmarks still
compare them.

---

## Architecture Curiosities

### 133 crates, zero circular dependencies

The workspace contains 133 crates organized into families
(`perl-module-*`, `perl-lsp-*`, `perl-dap-*`, `perl-ts-*`). Despite
this scale, there are zero circular dependencies. The microcrate
architecture makes this possible: each crate has a single
responsibility and a narrow public API. This also enables safe
parallelism -- 50-100 AI agents can work simultaneously in isolated
worktrees without merge conflicts.

### Dual indexing

Workspace symbols are indexed under both their qualified name
(`Foo::Bar::baz`) and their bare name (`baz`). This means both
"go to definition" and "workspace symbol search" work correctly
whether you type the full path or just the function name. See PR #122
for the original design.

### 3-layer path traversal prevention

Path handling goes through three independent layers of sanitization.
This was not defense-in-depth by design -- each layer was added after
discovering the previous one had edge cases.

### Feature governance pipeline

Every LSP capability goes through a governance pipeline:
`features.toml` (98 features defined) -> microcrate implementation ->
runtime feature gates. A feature cannot be advertised to clients
until it passes through all three stages.

### The one allowed `expect()` in production code

The coding standard bans `unwrap()`, `expect()`, `panic!()`, `todo!()`,
and `unimplemented!()` in all production code. The single exception is
documented in `crates/perl-lsp-rs/src/util/uri.rs`, which has an explicit
`#[allow(clippy::expect_used)]`. Test code gets its own allowances, but
in production, exactly one `expect()` has been deemed acceptable.

---

## Perl-Specific Weirdness

These are the parsing challenges that make Perl unique among languages.

### `/` is division OR regex

```perl
my $x = $a / $b;        # division
my $x = /pattern/;      # regex
my $x = $a /pattern/;   # ...actually still division! ($a / pattern /)
```

The lexer must track context to decide. The same character means
completely different things depending on what came before it. This is
why tree-sitter's context-free approach failed.

### `{}` is hash ref OR block OR bare block

```perl
my $h = { a => 1 };     # anonymous hash reference
if ($x) { print 1 }     # block
{ local $/ = undef; }   # bare block (no keyword!)
```

Three completely different AST nodes, same delimiters, disambiguated
only by context. The parser must look at what precedes the `{` and
sometimes what follows the `}`.

### Heredocs start on the next line

```perl
my $x = <<EOF;
This text belongs to the heredoc.
EOF
my $y = 42;   # This line is AFTER the heredoc
```

The heredoc body starts on the line after the `<<EOF` token, but
parsing must continue on the *same* line to handle the rest of the
statement. The parser effectively needs to read two lines simultaneously.

### `$/`, `$\`, `$;` -- punctuation variables that look like operators

```perl
local $/ = undef;    # input record separator (not division)
print $\;            # output record separator (not escape)
$hash{$a,$b}         # uses $; as subscript separator
```

Perl has dozens of punctuation variables (`$!`, `$@`, `$%`, `$^W`,
`$$`, `$"`, ...) that overlap with operators. The lexer must recognize
these as single tokens, not as a sigil followed by punctuation.

### `format` is a completely different mini-language

```perl
format STDOUT =
@<<<< @>>>>
$name, $age
.
```

The `format` keyword switches the parser into a wholly different mode
with its own syntax rules. The `.` on a line by itself ends the format
definition. This is essentially an embedded DSL that the parser must
handle as a special case.

---

## The Swarm in Numbers

The perl-lsp project pioneered AI-assisted development at scale using
Claude Code agent swarms.

| Metric | Value |
|--------|-------|
| Most agents in one session | 100 |
| Memory files encoding institutional knowledge | 98 |
| Archived agent definitions (replaced by skills) | 54 |
| Active skills | 10 |
| Development eras | 5 |
| PRs generated in Cycle 5 alone | 56 |
| Corpus improvement in one cycle | 51% -> 72% |

### Branch naming evolution

The branch naming tells the story of growing automation:
- **Era 1-2**: Human-chosen names (`feat/dap-support`, `fix/regex-slash`)
- **Era 3**: `codex/*` prefixes from batch tooling (613 branches)
- **Era 4-5**: `worktree-agent-HASH` from isolated Claude Code agents (94 commits)

### The optimal agent count

After running sessions with 10, 30, 50, 75, and 100 agents, the
empirical finding: **9 coding agents** is optimal. The bottleneck is
not agent capacity but CI throughput -- the merge queue is 3-wide,
and 75 agents generating 50+ PRs creates an unmanageable backlog.

### Research-then-build pattern

The most successful workflow: scout agents research root causes first,
then builder agents use the scout findings verbatim as prompts.
Constrained tasks (clear spec, single crate) succeed ~90% of the time;
unconstrained features succeed ~50%.

---

## When the AI Got Creative

### The benchmark confabulation

An early AI agent was asked to add benchmarks. It generated benchmarks
with hardcoded expected values that were "technically correct,
operationally meaningless" -- the benchmarks passed but measured nothing
useful. This led to the rule: always verify AI-generated test
assertions against real behavior.

### Three agents, one bug

During Cycle 4, three independent agents were assigned to different
parser error buckets. All three independently discovered the same root
cause in the expression parser. Rather than waste, this turned out to
be a feature: the second and third agents found better fixes because
they approached the problem from different angles.

### The revert that got reverted

`perf(lsp): use linear dedup for small highlight sets, eliminate clone`
was merged, reverted, and then the revert itself appears twice in
history. The optimization was correct but exposed a pre-existing
ordering assumption elsewhere.

### 52 agent definitions, 3 actual patterns

After defining 52 distinct agent types across six iterations
(`agents2` through `agents6`, plus `agents-compat`), analysis revealed
that all agents fell into just 3 patterns: scout, builder, and
reviewer. The later swarm kept the agent layer but pushed more of the
mechanical step instructions into composable skills.

---

## Hidden Infrastructure

### `deny.toml` -- 116-line supply chain firewall

The `deny.toml` configuration runs `cargo deny check` against every
dependency, enforcing license compatibility, advisory database checks,
and banning known-vulnerable crates. 116 lines of policy that prevent
supply chain attacks silently.

### The corpus ratchet

The CPAN corpus pass rate can only go up, never down. The
`just cpan-corpus-check` command enforces a manifest of known-clean
modules. If a parser change causes a previously-clean module to fail,
CI blocks the merge. New clean modules are added via
`just cpan-corpus-ratchet`.

### `GATE_REGISTRY.toml` -- 13 merge-blocking gates

Every PR must pass 13 gates defined in `.ci/GATE_REGISTRY.toml`:
formatting, clippy, tests, policy checks, and more. Each gate has a
defined timeout, cost tier, and failure impact level. The registry is
the single source of truth for what "CI green" means.

### The pre-commit hook pipeline

`scripts/install-githooks.sh` installs a pre-commit hook that runs
`perl-ci-hygiene`, a custom Rust binary built from the workspace
itself. The tool enforces zero panics, zero `dbg!()` calls, and
formatting compliance before a commit can even be created. The parser
project builds its own development tools.

### `features.toml` -- 97 LSP capabilities governed

The LSP feature catalog at `features.toml` defines 97 features with
their spec version, maturity level, test locations, and whether they
are advertised to clients. At 99% compliance with LSP 3.18, nearly
every capability the protocol offers is implemented.

---

*Last updated: 2026-03-19. Data gathered from commit `e6178969a` on `master`.*
