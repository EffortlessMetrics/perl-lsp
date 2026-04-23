---
description: Write a version-bound label receipt on a PR or issue
argument-hint: "<pr|issue> <number> <label> <agent-name>"
---

# Label Receipt Write

Record a version-bound label binding on a PR or issue. This creates or updates a
receipt comment that tracks which version of the artifact the label was evaluated against.

Context: **$ARGUMENTS**

## Steps

### 1. Parse arguments

Extract from $ARGUMENTS:
- `artifact_type`: `pr` or `issue`
- `number`: PR or issue number
- `label`: the pipeline label being written (e.g., `merge-ready`, `in-build`)
- `agent_name`: the agent writing the label (e.g., `pr-ready`, `builder`, `reviewer`)

If arguments are missing, report usage and stop:
```
Usage: /label-receipt-write <pr|issue> <number> <label> <agent-name>
Example: /label-receipt-write pr 2645 merge-ready pr-ready
```

### 2. Get current artifact version

For PRs, get the HEAD SHA:
```bash
CURRENT_SHA=$(gh pr view $NUMBER --json headRefOid --jq '.headRefOid')
CURRENT_UPDATED=$(gh pr view $NUMBER --json updatedAt --jq '.updatedAt')
```

For issues, get the updated_at timestamp:
```bash
CURRENT_UPDATED=$(gh issue view $NUMBER --json updatedAt --jq '.updatedAt')
CURRENT_SHA="n/a"
```

### 3. Check for existing receipt comment

Search for the receipt comment marker. Use the `issues` endpoint for both PRs and
issues -- on GitHub's API, `gh pr comment` creates an issue-type comment, and the
`pulls/.../comments` endpoint only returns line-level review comments, not general comments.

```bash
EXISTING_COMMENT=$(gh api "repos/{owner}/{repo}/issues/$NUMBER/comments" \
  --jq '.[] | select(.body | contains("<!-- LABEL_RECEIPT_v1 -->")) | {id: .id, body: .body}' \
  | head -1)
```

### 4. Build the label binding

Create the new binding entry:
```json
{
  "label": "<label>",
  "bound_at_version": "<CURRENT_SHA>",
  "bound_at_timestamp": "<now ISO 8601>",
  "bound_by_agent": "<agent_name>",
  "valid": true
}
```

### 5. Create or update receipt comment

**If no existing receipt comment**, create one:

```bash
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
ARTIFACT_ID="<type>-$NUMBER"
ARTIFACT_TYPE_FULL=$( [ "$ARTIFACT_TYPE" = "pr" ] && echo "pull_request" || echo "issue" )

BODY=$(cat <<RECEIPT_EOF
<!-- LABEL_RECEIPT_v1 -->
\`\`\`json
{
  "schema_version": "1.0",
  "artifact_id": "$ARTIFACT_ID",
  "artifact_type": "$ARTIFACT_TYPE_FULL",
  "current_version": {
    "sha": "$CURRENT_SHA",
    "updated_at": "$CURRENT_UPDATED"
  },
  "label_bindings": [
    {
      "label": "$LABEL",
      "bound_at_version": "$CURRENT_SHA",
      "bound_at_timestamp": "$TIMESTAMP",
      "bound_by_agent": "$AGENT_NAME",
      "valid": true
    }
  ]
}
\`\`\`
<!-- /LABEL_RECEIPT_v1 -->
RECEIPT_EOF
)
```

For PRs:
```bash
gh pr comment $NUMBER --body "$BODY"
```

For issues:
```bash
gh issue comment $NUMBER --body "$BODY"
```

**If existing receipt comment**, update it:
1. Parse the existing JSON from the comment body
2. Update `current_version` to the latest
3. Invalidate any existing binding for the same label at a different version
   (set `valid: false`)
4. Append the new binding
5. Edit the comment:
```bash
gh api --method PATCH "repos/{owner}/{repo}/issues/comments/$COMMENT_ID" \
  -f body="$UPDATED_BODY"
```

Note: `issues/comments` endpoint works for both PR and issue comments on GitHub API.

### 6. Report

Output:
```
Receipt written: $LABEL on $ARTIFACT_TYPE #$NUMBER
  Version: $CURRENT_SHA
  Timestamp: $TIMESTAMP
  Agent: $AGENT_NAME
```

## Notes

- Receipt comments use `<!-- LABEL_RECEIPT_v1 -->` markers for programmatic discovery
- The JSON is wrapped in a code fence for readability on GitHub
- One receipt comment per artifact; multiple bindings in the same comment
- Old bindings for the same label are marked `valid: false`, not deleted
- This skill is called by other skills after label writes; it does NOT write the label itself
