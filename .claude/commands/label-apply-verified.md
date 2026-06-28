---
description: Apply a pipeline label with read-back verification and retry — stops silent label-apply failures
argument-hint: "<pr|issue> <number> <label>"
---

# Label Apply Verified

Apply a single pipeline label to a PR or issue, then **verify it actually
landed** before reporting success.

Bare `gh ... --add-label` writes issued from an agent skill layer fail silently
far more often than not (~80% miss rate per issue #807): the command exits `0`
but the label never lands, leaving the PR/issue in a wrong pipeline state.
Labels are the authoritative state of the orchestration, so a silent miss
corrupts routing and forces an operator to re-apply labels by hand.

This skill closes that gap: **apply → wait for eventual consistency → read the
label back → retry on miss (max 3×) → emit a hard error if it never lands.**

Context: **$ARGUMENTS**

## When to use

Use this **instead of a bare `gh issue edit --add-label` / `gh pr edit
--add-label`** anywhere a skill applies a sign-off, routing, or state label.

Removing a label (`--remove-label`) does **not** need this wrapper — a failed
removal only leaves a stale queue-filter label, not a missing sign-off.

## Steps

### 1. Parse arguments

Extract from $ARGUMENTS into shell variables:
- `ARTIFACT_TYPE`: `pr` or `issue`
- `NUMBER`: the PR or issue number
- `LABEL`: the single label to apply (e.g. `review-reviewed`, `needs-ci-fix`)

If any are missing, report usage and stop:
```
Usage: /label-apply-verified <pr|issue> <number> <label>
Example: /label-apply-verified pr 2645 review-reviewed
```

### 2. Apply with verification and retry

Run the verified-apply procedure. It applies the label, waits ~2s for GitHub's
eventual-consistency window, reads the label back with `--json labels`, and
retries up to 3 times with a 1s backoff. It returns non-zero (and prints an
`ERROR:` line) if the label never lands.

```bash
apply_label_verified() {
  # usage: apply_label_verified <pr|issue> <number> <label>
  local kind="$1" num="$2" label="$3" attempt
  local edit_cmd view_cmd
  case "$kind" in
    pr)    edit_cmd="gh pr edit";    view_cmd="gh pr view" ;;
    issue) edit_cmd="gh issue edit"; view_cmd="gh issue view" ;;
    *) echo "ERROR: artifact_type must be 'pr' or 'issue', got '$kind'" >&2; return 2 ;;
  esac

  for attempt in 1 2 3; do
    $edit_cmd "$num" --add-label "$label" >/dev/null 2>&1 || true
    sleep 2  # GitHub eventual-consistency window
    if $view_cmd "$num" --json labels --jq '.labels[].name' | grep -Fxq "$label"; then
      echo "label-apply OK: '$label' landed on $kind #$num (attempt $attempt/3)"
      return 0
    fi
    echo "label-apply MISS: '$label' not yet on $kind #$num (attempt $attempt/3); retrying..." >&2
    sleep 1  # backoff before the next attempt
  done

  echo "ERROR: label-apply FAILED — '$label' did not land on $kind #$num after 3 attempts. Apply it manually and re-verify." >&2
  return 1
}

apply_label_verified "$ARTIFACT_TYPE" "$NUMBER" "$LABEL"
```

> **MCP alternative (web / no-`gh` sessions):** read current labels with
> `mcp__github__pull_request_read(method:"get", pullNumber:<number>)` (or the
> issue equivalent), write the union with
> `mcp__github__issue_write(method:"update", issue_number:<number>,
> labels:[...current + "<label>"])`, then **read the labels back** to confirm
> `<label>` is present. `issue_write` replaces the full label list, so always
> read-then-union-then-verify.

### 3. On hard failure

If `apply_label_verified` returns non-zero after all retries, do **not** report
the label as applied. Surface the explicit `ERROR:` line to the operator (in
your agent wrap-up / final message) so the miss is visible, not silent — the
pipeline state is wrong until the label lands.

## Rules

- **Apply the comment/work first, then the label.** The label is the completion
  signal — never set it before the work it certifies is done.
- **One label per call.** For multiple labels, invoke this skill once per label
  so each is verified independently.
- **Never report success on a miss.** A bare `gh ... --add-label` exit code is
  NOT proof the label landed (issue #807). Only the read-back verification is.
- **Removals stay bare.** `--remove-label` does not need this wrapper.
- If the label may not exist yet, create it first
  (`gh label create "<label>" ... 2>/dev/null || true`) — a write to a
  nonexistent label is one of the silent-miss causes.

## Output

```
label-apply OK: '<label>' landed on <pr|issue> #<number> (attempt N/3)
```
or, on hard failure:
```
ERROR: label-apply FAILED — '<label>' did not land on <pr|issue> #<number> after 3 attempts.
```
