# Every Deep Review Found A Real Bug: Why Two-Pass AI Review Is Non-Negotiable

---

## The Claim

In Era 7 session 2 of perl-lsp development, every PR that went through deep
review had at least one real bug caught by that review. Not one missed.

Thirteen PRs deep-reviewed. Thirteen had real bugs. 100% hit rate.

This is not a headline designed to impress — it is a data point that should
make you uncomfortable if your team is running one review pass and calling it
done.

---

## What "Two-Pass Review" Means

The pipeline has two review stages:

**Standards pass (haiku model):** Check formatting. Verify no banned constructs
(`unwrap()`, `expect()`, `panic!()`). Confirm the PR description matches the
diff. Catch obvious scope creep. This is fast — 90 seconds, cheap model.

**Deep pass (sonnet model):** Read the issue spec. Trace the actual execution
paths. Construct adversarial inputs. Verify Perl semantics for parser changes.
Check that tests actually test what they claim to test. Push fixes directly to
the branch. This takes 5 minutes and costs more.

The standards pass is not the review. It is the pre-flight. The deep pass is
the review.

Here is what the deep pass found across 13 PRs in a single session.

---

## The Bug List

### #2590 — Three Regex Bugs in the Pre-Tool-Use Hook

The PR added a `pre-tool-use.sh` hook to block dangerous commands before
agents could execute them. The standards pass confirmed it existed, was
executable, and had tests.

The deep reviewer read the actual regex patterns.

**Bug 1: `--force-with-lease` was blocked, `-f` alone was not.** The pattern
matched `git push --force` and `git push --force-with-lease` but did not match
`git push origin main -f`. A one-character flag bypass on a safety guard.

**Bug 2: `rm -rf` only matched root paths.** The pattern required `rm -rf /`
(or similar root-path form) but did not match `rm -rf ~` or `rm -rf ./src`.
The guard would allow `rm -rf ~/code` to execute unchallenged.

**Bug 3: `/tmp` false positive.** The pattern was broad enough to match
legitimate operations targeting `/tmp` paths, which would block routine
temp-file cleanup. A safety hook that blocks normal operations gets disabled.

The standards pass saw: tests exist, pattern is present, file has correct
permissions. The deep pass saw: this regex has three distinct bypass vectors
and one false positive mode.

---

### #2593 — Missing `)` Guard on Condition Parse Path

The PR fixed recovery from missing semicolons in C-style `for` loops. The
before-and-after was clear: six cascading errors reduced to one clean error.
Tests passed.

The deep reviewer traced the condition parse path.

When the first semicolon is missing and parsing continues into the condition
expression, the `expect_closing_delimiter` call for the `)` that closes the
`for(...)` block had a recovery path that was inconsistently applied. One path
added a recovery node; another path returned `Err` directly. The inconsistency
meant that specific combinations of missing tokens could revert to the
pre-fix behavior of generating cascading errors.

The fix was a two-line guard. Without it, the recovery was incomplete.

---

### #2597 — Vacuous Test and Per-Item Heap Allocation

The PR added `commit_characters` to the LSP `CompletionItem` struct and wired
both completion dispatch paths to emit them correctly.

Two bugs found:

**Bug 1: Vacuous test.** One of the three new integration tests asserted that
`commitCharacters` was present in the response without checking what it
contained. The test would pass whether the field contained the correct Perl-
specific characters or an empty array. It was testing presence, not correctness.

**Bug 2: Per-item heap allocation on the hot completion path.** The
`commit_chars_for_kind` helper returned `Option<Vec<String>>`. The `Vec` is
allocated fresh for each completion item, which in a typical completion
response means 20-80 allocations for what is effectively constant data. The
fix is to return `Option<&'static [&'static str]>` and let the JSON serializer
handle the conversion once at serialization time.

The standards pass saw: 34 tests pass, clippy clean, fmt clean. The deep pass
saw: one of those 34 tests is unfalsifiable, and the hot path allocates where
it does not need to.

---

### #2598 — Command Injection in Double-Quoted Templates

The PR updated skill files so agents post GitHub wrapup comments at the end of
each session. The templates included `gh issue comment` and `gh pr comment`
commands.

The templates were double-quoted shell strings:

```sh
gh issue comment "$ISSUE_NUMBER" --body "Wrapup: $SUMMARY"
```

If `$SUMMARY` is set to a value containing a backtick sequence or `$(...)`,
the shell expands it. An agent filling in a summary from tool output that
contains command substitution syntax could execute arbitrary shell commands.

The fix is single-quoted bodies or heredoc syntax. The distinction between
`"$SUMMARY"` and `'$SUMMARY'` (or `<<'EOF'`) is the difference between
command injection and literal string injection.

A standards reviewer checks that the command exists and the field names match.
They do not simulate what happens when the field content contains shell
metacharacters.

---

### #2601 — Accidental Deletion of 26 Files

This one is structural.

The PR was scoped to fix `$::{$key}` main stash access — a two-crate change
touching the lexer and the parser. The diff included 26 unrelated files from
a different fix that had been staged in the same worktree.

The standards pass would catch this only if it checked file scope against the
issue spec. The deep pass explicitly checks: does every file in this diff
belong here?

The 26 files were not deletions with malicious intent — they were orphaned
staged changes from a previous operation in the worktree. But "accidental" does
not mean "harmless." Any of those 26 files could have removed a test,
reverted a fix, or introduced a regression that passed CI because the original
test for that behavior was in the deleted file.

The fix was to unstage the 26 files and commit only the two intended changes.
No one would have caught this in CI.

---

### #2606 — Two Logic Bugs in Die/Warn Hover and Code Actions

The PR enriched `die`/`warn` hover documentation and added a code action to
suggest `Carp::croak` for bare `die` calls in module files.

Two logic bugs:

**Bug 1: Parenthesised `die` form not recognized.** The `find_die_in_module`
code action scanned for `die` with a pattern that matched `die "message"` but
not `die("message")`. Perl permits both forms; the parenthesised form is common
in generated code and complex expressions. The action would silently skip the
parenthesised form.

**Bug 2: Multi-line `or die` not exempted.** The action explicitly exempted
`or die` and `|| die` patterns (system-call idioms that should not be
modernized to `croak`). But the pattern only matched single-line occurrences.
A multi-line expression like:

```perl
open(my $fh, '<', $file)
    or die "Cannot open: $!";
```

...would match the `die` but not the `or` on the previous line, and the
exemption would not fire. The action would incorrectly suggest `croak` for a
legitimate `or die` idiom.

The standards pass checks: tests pass, pattern exists, exemption documented.
The deep pass constructs: what are all the syntactic forms of this construct,
and does the pattern handle them?

---

### #2616 — Loop Ordering Bug and Zero-Timeout Footgun

The PR added thread-based subprocess timeouts for `perltidy` and `perlcritic`.
The core logic: spawn a child process in a background thread, poll every 50ms,
kill on deadline.

Two bugs:

**Bug 1: Finished check after deadline check.** The poll loop checked the
deadline first. If the process finished in the same 50ms window as the
deadline expired, it would be reported as a timeout even though it succeeded.
The correct order is: check if finished, then check if deadline exceeded. The
distinction matters at low timeout values.

**Bug 2: Zero-timeout footgun.** `with_timeout(0)` produced a timeout that
fired immediately, before the process could start. The constructor accepted 0
as a valid input. In practice this means a caller passing a config value of 0
(meaning "no timeout" in most APIs) gets an immediately-timing-out subprocess.
The fix is to treat 0 as `None` (no timeout) in the constructor.

Neither of these would fail tests — the tests used a 1-second timeout against
a `sleep 10` command, well outside the race window. The deep reviewer read the
loop logic and constructed the edge cases.

---

### #2619 — `List::MoreUtils` Incorrectly in `CORE_MODULES`, Empty Module Guard Missing

The PR added `PL701 ModuleNotFound` diagnostics with a ~130-entry
`CORE_MODULES` exemption list preventing false positives.

Two bugs:

**Bug 1: `List::MoreUtils` in `CORE_MODULES`.** `List::MoreUtils` is not a
core Perl module. It is a CPAN distribution. Including it in the exemption
list means `use List::MoreUtils;` would never fire PL701 even when the module
is genuinely missing. The module was probably added because it is extremely
common, but "extremely common" and "always installed" are not the same thing.
Eleven other similar CPAN-but-popular modules were audited and found clean.

**Bug 2: Empty module guard missing.** The resolver lookup was called with
the raw module name from the `use` statement. `use ;` (empty use) or a use
statement where the module name parsed to an empty string would call
`resolve_module_to_path("")`. The resolver's behavior on empty input was
undefined — depending on the filesystem, it could match a directory, panic,
or return a false `Some`. The fix is a guard that skips PL701 for zero-length
module names.

The standards pass sees: 99 tests pass, exemption list present. The deep pass
asks: is every entry in that list actually a core module?

---

### #2624 — Two Missing Workspace Symbol Extraction Arms

The PR fixed hover for Perl 5.38+ native `class`/`method` syntax — four
bugs fixed across symbol classification, class model building, and hover
resolution.

The deep review found two more:

**Missing arm 1: `NodeKind::Class` not extracted by workspace indexer.** The
`ClassModelBuilder` visitor was fixed to handle `NodeKind::Class` and
`NodeKind::Method`, but the workspace symbol extraction pass (a separate
traversal) was not updated. Native classes would still be invisible to
workspace-wide symbol search, go-to-definition across files, and the symbol
outline.

**Missing arm 2: `NodeKind::Method` not in the workspace index.** Same gap
for method nodes — they exist in the per-file semantic model but were not
promoted to the workspace symbol index. Cross-file navigation for native
methods would silently return nothing.

The PR's own tests passed because they tested per-file hover, not
cross-file symbol resolution. The deep reviewer read the issue spec ("fix
native class/method hover") and asked: does "hover" also mean "go-to-definition
from another file"? Yes. Is that path wired? No.

---

## Why Standards Review Isn't Enough

A standards review is a checklist pass. It answers:

- Does the code compile?
- Are banned constructs absent?
- Does the PR description match the diff roughly?
- Are there tests?
- Does clippy pass?

These checks catch a real and important class of bugs. They are necessary.
They are not sufficient.

A standards reviewer does not:

- Trace execution paths to find the inputs that reach a dangerous branch
- Verify that a regex pattern handles all syntactic variants of what it
  claims to match
- Check whether the exemption list in a 130-entry constant is actually correct
- Ask whether the test suite would detect the bug it claims to detect
- Simulate what happens when a field contains shell metacharacters

The patterns that the deep reviewer catches are not subtle. They are not
creative security research. They are "this regex does not match `-f`" and
"this list contains a module that is not in the standard library." They are
the kinds of bugs that a careful human reviewer with context would catch in
a 20-minute review.

The standards pass is not that reviewer. It is a lint check with extra steps.

---

## Why Deep Review Works

The deep reviewer has three things the standards reviewer does not:

**1. The issue spec.** The deep reviewer reads what the PR was supposed to do,
not just what it did. This is how the missing workspace indexer arms were
caught — the issue said "fix class/method hover" and the reviewer asked
"hover includes cross-file, is cross-file fixed?"

**2. Adversarial inputs.** The deep reviewer does not ask "do the happy-path
tests pass?" It asks: what are the inputs that would exercise the failure
mode? For #2590, that means: what `git push` invocation bypasses this regex?
For #2606, that means: what syntactic forms of `die` does this pattern miss?

**3. The ability to push fixes directly.** The reviewer does not file a
comment saying "consider handling `-f`." It writes the fix, pushes it to the
branch, and the PR goes green. The fix is in the PR, not in a follow-up.

This last point matters more than it appears to. A code review that produces
comments produces work for the author. An adversarial reviewer that produces
fixes produces a better PR. The former has a 48-hour turnaround. The latter
has a 5-minute turnaround.

---

## The Economics

Deep review at sonnet cost: approximately 5 minutes and $0.50-1.00 per PR.

The alternative: bugs on master. Not hypothetical bugs — real ones. The PR
with 26 accidentally-deleted files would have landed on master. The hook with
three regex bypass vectors would have protected nothing. The code action that
mishandles parenthesised `die` would have silently skipped half the cases
it was designed to find.

The cost of a bug on master is not just the hotfix PR. It is:

- **Trust erosion.** An agent that writes code and an agent that reviews it
  share a failure mode: both will normalize the same patterns. When the first
  bug reaches master, the next reviewer's confidence in "green CI" goes down.
- **Compounding cost.** Bugs that slip through one PR create the conditions
  for future bugs. A missing guard on an empty module name means any future
  PR that adds to the module resolver must work around the undefined-empty-
  input behavior. Technical debt compounds.
- **Silent failures.** The worst bugs in this list are the ones that pass all
  tests. The vacuous completion test, the loop ordering race, the multi-line
  `or die` exemption miss — none of these would have failed CI. They would
  have been deployed, and the failure mode would have appeared in user-reported
  edge cases.

Deep review is cheaper than not deep-reviewing.

---

## The Pattern

The standards pass and deep pass are not redundant. They catch different things.

The standards pass catches approximately 60% of issues by volume: formatting,
lint violations, obvious scope violations, missing tests. These are cheap to
fix and easy to detect automatically.

The deep pass catches the remaining 40%. But that 40% is not uniformly
distributed. It contains:

- Logic bugs that pass all existing tests
- Safety mechanism bypasses that only fail on adversarial inputs
- Correctness gaps in new features that test the happy path but not the
  edge cases
- Accidental side effects from worktree state

These are the bugs that reach users. They do not appear in CI. They do not
trigger lint warnings. They survive because no automated check ever asks
"is this regex actually correct for all inputs?" or "does this list actually
contain what it claims to contain?"

The separation of duties is real. The haiku model is fast, cheap, and good
at checklists. The sonnet model is slower, costs more, and reasons about
execution paths. You need both. They are not interchangeable at different
price points — they are different tools doing different work.

---

## What This Means For Teams

If you are using AI agents to write code and AI agents to review it, one review
pass is not sufficient.

The first pass normalizes. It learns what "good enough" looks like from the
training distribution and applies that judgment consistently. When a pattern
like "have tests, have a description, pass clippy" appears on 1000 PRs, the
first-pass reviewer learns to approve PRs that match that pattern.

The second pass adversarializes. It does not ask "does this match the approval
pattern?" It asks "if this code were wrong, how would it be wrong, and would
anyone notice?"

These are different questions. The first question is easier and cheaper.
The second question is the one that matters.

The 100% hit rate across 13 PRs is not evidence that the first pass was
useless. The first pass caught real things — formatting issues, pattern
violations, scope checks. The 100% hit rate is evidence that the first pass
consistently left something unfound. Every PR had a residual. The second pass
found it every time.

That residual is not random noise. It has a shape: logic over structure, edge
cases over happy paths, adversarial inputs over representative inputs. It is
systematically the work that checklist review cannot do.

Run the checklist. Then run the adversary.

---

*Data from Era 7 session 2. Bug descriptions are from the `feedback_deep_review_value.md` memory file and PR review records. PR numbers are stable references to the perl-lsp GitHub repository.*
