# Version-Bound Pipeline Receipts

Pipeline labels (`merge-ready`, `in-build`, `plan-reviewed`, etc.) enable orchestrator
routing but lack version information. When an artifact changes (PR pushed, issue updated),
stale labels persist without recording what version they evaluated.

Version-bound receipts solve this by recording **what version of the artifact each label
was evaluated against**, turning labels from historical stickers into versioned routing state.

## Problem

GitHub labels have no metadata fields. When a reviewer marks a PR `merge-ready` at
commit `abc123`, and the builder then pushes `def456`, the label persists but refers
to an old commit. Any agent that blindly trusts the label may route incorrectly.

## Solution: Label Receipt Comments

Each PR or issue gets a single **receipt comment** (created or updated atomically) that
tracks which version of the artifact each label was bound to. The comment uses an HTML
marker so it can be found and updated programmatically.

### Receipt Comment Format

```
<!-- LABEL_RECEIPT_v1 -->
{
  "schema_version": "1.0",
  "artifact_id": "pr-2645",
  "artifact_type": "pull_request",
  "current_version": {
    "sha": "def456789abc",
    "updated_at": "2026-03-21T11:05:30Z"
  },
  "label_bindings": [
    {
      "label": "in-review",
      "bound_at_version": "def456789abc",
      "bound_at_timestamp": "2026-03-21T10:30:00Z",
      "bound_by_agent": "reviewer",
      "valid": true
    },
    {
      "label": "merge-ready",
      "bound_at_version": "def456789abc",
      "bound_at_timestamp": "2026-03-21T11:00:00Z",
      "bound_by_agent": "pr-ready",
      "valid": true
    }
  ]
}
<!-- /LABEL_RECEIPT_v1 -->
```

### Field Definitions

| Field | Type | Description |
|-------|------|-------------|
| `schema_version` | string | Receipt schema version for forward compatibility |
| `artifact_id` | string | Identifier: `pr-<number>` or `issue-<number>` |
| `artifact_type` | enum | `pull_request` or `issue` |
| `current_version.sha` | string | HEAD SHA (PRs) or `n/a` (issues) |
| `current_version.updated_at` | ISO 8601 | Last update timestamp |
| `label_bindings[].label` | string | The pipeline label name |
| `label_bindings[].bound_at_version` | string | SHA or timestamp when label was set |
| `label_bindings[].bound_at_timestamp` | ISO 8601 | When the label was bound |
| `label_bindings[].bound_by_agent` | string | Which agent/skill set the label |
| `label_bindings[].valid` | boolean | Whether binding is still current |

### Freshness Check

A label binding is **fresh** when:
- For PRs: `bound_at_version` matches `current_version.sha`
- For issues: `bound_at_timestamp` is after the issue's last `updated_at`

A label binding is **stale** when the artifact has changed since the label was set.
Stale bindings have `valid: false`.

### How to Write a Receipt

Use the `/label-receipt-write` skill after any label change:

```bash
/label-receipt-write <artifact-type> <number> <label> <agent-name>
```

This will:
1. Get the current artifact version (HEAD SHA for PRs, updated_at for issues)
2. Find or create the receipt comment on the artifact
3. Add or update the label binding with the current version
4. Mark any bindings for the same label at older versions as invalid

### How to Validate a Receipt

Use the `/label-receipt-validate` skill before trusting a label:

```bash
/label-receipt-validate <artifact-type> <number> <label>
```

This will:
1. Get the current artifact version
2. Find the receipt comment
3. Check if the label binding matches the current version
4. Report: `FRESH` (safe to trust) or `STALE` (artifact changed since label was set)

## Integration Points

The following skills write pipeline labels and should call `/label-receipt-write`:

| Skill | Label Written | Agent |
|-------|--------------|-------|
| `/pr-ready` | `merge-ready` | pr-ready |
| `/plan-review-improve` | `plan-reviewed`, `builder-ready` | plan-reviewer |
| `/builder-read-spec` | `in-build` | builder |
| `/reviewer-read-handoff` | `in-review` | reviewer |
| `/reviewer-decide` | `needs-deep-review` | reviewer |
| `/ops-merge-batch` | (removes `merge-ready`) | ops |

## Design Decisions

1. **Comments over files**: Receipt comments live on the artifact itself, not in repo
   files. This avoids repo writes and keeps receipts co-located with the artifact.

2. **Single comment, updated atomically**: One receipt comment per artifact prevents
   comment thread noise. The HTML markers enable find-and-replace.

3. **Versioned schema**: `schema_version` allows future format changes without
   breaking existing receipts.

4. **Valid flag over deletion**: Marking bindings as `valid: false` preserves audit
   trail. Old bindings are not deleted, just invalidated.

## Related

- `.ci/receipt.schema.json` -- CI gate receipt format (different purpose: CI runs)
- `.ci/schemas/receipt.schema.yaml` -- Gate execution receipt schema
- Issue #2159 -- Receipt directory infrastructure
- PR #2638 -- Label-driven pipeline state machine
