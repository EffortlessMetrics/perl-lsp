# Impacted Evidence

## Routing

- mutation_mode: `targeted`
- requires_targeted_mutation: true
- requires_full_owner_mutation: false
- ripr_severe_gap: true
- routing_reason: `ripr severe gap`

## Inputs

- PR evidence: `target/ripr/pr/repo-exposure.json`
- labels: `in-review`

## Artifacts

| Artifact | Path | Available |
| --- | --- | --- |
| impacted evidence JSON | `target/xtask/impacted-evidence/latest.json` | true |
| impacted evidence Markdown | `target/xtask/impacted-evidence/latest.md` | true |
| PR evidence JSON | `target/ripr/pr/repo-exposure.json` | true |

_This receipt routes verification work. It does not execute mutation._
