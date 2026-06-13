# PR Evidence Summary

## Fast Gate

- status: `advisory`
- root: `.`
- base: `origin/main`
- head: `HEAD`
- changed files: 5

## RIPR

- changed-line comments: 0
- summary-only guidance: 0
- suppressed guidance: 0
- weakly_exposed: 2
- reachable_unrevealed: 0
- no_static_path: 0
- suppressed_by_policy: 0
- severe gaps: 2

## Targeted Mutation

- requires_targeted_mutation: true
- routing_reason: `ripr severe gap`

## Artifacts

| Artifact | Path | Scope | Available |
| --- | --- | --- | --- |
| PR evidence JSON | `target/ripr/pr/repo-exposure.json` | diff | true |
| PR evidence Markdown | `target/ripr/pr/repo-exposure.md` | diff | true |
| Analyzed PR diff | `target/ripr/pr/pr.diff` | diff | true |

_This packet is diff-scoped and advisory. Do not copy it into public badge state._
