# PR Slice Size Archaeology
## Bounded Changes, Campaign PRs, And Why The Repo Ships Both

This note uses GitHub PR metadata to look at change size, not just PR count.

I pulled a 300-PR snapshot from the live archive with `additions`, `deletions`, `changedFiles`, `title`, and `headRefName`, then grouped the results by rough slice size. The goal is not to claim every PR in the repository behaves the same way. The goal is to show what the archive suggests about the repo's working style.

The answer is mixed, but not chaotic: the repo strongly prefers small bounded slices, yet it also uses medium refactors and the occasional very large umbrella PR when the work calls for it.

---

## 1. The Small-Slice Default Is Real

In the 300-PR slice I pulled, 77 PRs fit a tight-slice profile:

- `changedFiles <= 3`
- `additions + deletions <= 100`

That is a strong signal. A lot of the repo's PRs are intentionally narrow: one file, two files, or a small edit set that can be reviewed quickly.

Representative examples from the same slice:

- `#1828` `Improve workspace discovery logging` - 1 file, 100 total line changes
- `#1836` `Improve async LSP read scheduling` - 2 files, 100 total line changes
- `#1994` `test(parser-core): add oneliner regression coverage` - 1 file, 100 total line changes
- `#1948` `test(parser-core): add paren recovery test coverage` - 2 files, 99 total line changes
- `#2121` `chore(swarm): add /merge-queue skill` - 1 file, 98 total line changes
- `#2229` `feat(lsp): add large file size guard (#2163)` - 3 files, 96 total line changes

These are bounded slices in the literal sense. They are easy to isolate, easy to review, and easy to dispose of or merge independently.

That matches the rest of the control-plane archaeology:

- worktree isolation
- one-agent slices
- draft PR staging
- explicit review and merge gates

The repository does not appear to prefer giant PRs as a default. It prefers small slices by design.

---

## 2. Medium Slices Fill The Middle

The same 300-PR snapshot also contains 67 medium-sized PRs:

- more than 3 files
- up to 15 files
- up to roughly 1,000 total changed lines

That middle band matters because it is where many real refactors live. These are not tiny polish changes, but they are still reviewable without turning into campaign PRs.

This is the sweet spot for a lot of the repo's ordinary engineering work:

- parser corrections
- test expansion
- feature gating
- modular refactors
- docs adjustments tied to a behavior change

The archive suggests a healthy bounded-change culture, not an all-or-nothing one.

---

## 3. Large PRs Are Deliberate, Not Exceptional Noise

The large end of the distribution is smaller in count, but it is not accidental. In the 300-PR slice, 43 PRs qualify as large:

- more than 15 files, or
- more than 1,000 total line changes

Representative outliers from the same slice:

- `#2004` `revert(claude): restore archived historical agent directories` - 552 files, 62,135 total changes
- `#1911` `chore: remove abandoned agent directory iterations (agents2-6, agents-compat)` - 552 files, 62,126 total changes
- `#2034` `test(vscode): add unit and integration tests (#1660)` - 7 files, 8,295 total changes
- `#2230` `docs: consolidate historical analyses under docs/articles` - 20 files, 6,743 total changes
- `#1794` `refactor(execute-command): extract execute-command provider into perl-lsp-execute-command microcrate` - 7 files, 3,933 total changes
- `#1849` `Modularize execute_command into internal module tree (executor + tests)` - 4 files, 3,832 total changes
- `#2208` `refactor(dap): split debug_adapter.rs into domain modules` - 6 files, 3,782 total changes

These are not random oversized diffs. They are campaign PRs:

- archive and restore operations
- large refactors
- test-suite expansion
- docs consolidation waves
- structural module splits

The large PRs usually correspond to deliberate re-organization, not routine feature work.

---

## 4. Docs Waves Tend To Be Small But Frequent

The recent docs/article wave is a good example of how the repo uses small slices even for visible work.

The article PRs in the current wave are narrow enough to land independently:

- `#2224` `docs: add Five Eras of AI Development article`
- `#2225` `docs: add swarm methodology reference article`
- `#2226` `docs: add zero-panic reliability and security article`
- `#2227` `docs: add Parsing Perl challenges article`
- `#2228` `docs: add development curiosities and records`

Those are separate PRs, not one massive docs dump. The repo is batch-oriented, but it still keeps the batch subdivided into reviewable slices.

That is a useful distinction:

- the repo does work in waves
- but the waves are often composed of small PRs

---

## 5. What The Archive Suggests About The Operating Model

The PR archive points to a mixed strategy:

1. default to small bounded PRs
2. use medium slices for ordinary refactors and feature work
3. reserve large umbrella PRs for archive, migration, structural cleanup, or campaign-grade changes

That mix is consistent with the swarm model elsewhere in the archaeology:

- small slices let agents stay isolated
- larger campaign PRs let the repo absorb structural shifts
- docs waves can be split into many small PRs instead of one monolith
- cleanup and archival work can still legitimately be large when the target surface is large

So the repo is not "small PR only."
It is "small by default, large when the unit of change demands it."

---

## Evidence Pointers

- `gh pr list --state all --limit 300 --json number,title,headRefName,additions,deletions,changedFiles,createdAt,isDraft`
- `#1828` `Improve workspace discovery logging`
- `#1836` `Improve async LSP read scheduling`
- `#1994` `test(parser-core): add oneliner regression coverage`
- `#1911` `chore: remove abandoned agent directory iterations (agents2-6, agents-compat)`
- `#2004` `revert(claude): restore archived historical agent directories`
- `#2034` `test(vscode): add unit and integration tests (#1660)`
- `#2230` `docs: consolidate historical analyses under docs/articles`
- `#2224` through `#2228` article-wave PRs
