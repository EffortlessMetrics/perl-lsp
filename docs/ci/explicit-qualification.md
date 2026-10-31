# Explicit PR qualification

Run the repository's checks in the coding session before requesting merge.
Adding `ci:qualify` to a non-draft PR requests the applicable PR qualification
workflows. Opening, updating, reopening, or marking a PR ready does not request
those workflows. Other labels do not request this qualification. Optional
`ci:*` labels can request their documented deeper lanes explicitly.

If the head changes, remove and reapply `ci:qualify` to request checks on the
new head. A label is not a passing check: inspect the completed checks and bind
evidence to the current head and base before queueing. GitHub's merge queue
runs its configured integration checks on the composed candidate.

Publication Sync Contract remains a separate required PR check, including on
body edits, because the PR body is part of its contract. Its merge-group
counterpart must be established before enabling a queue on the protected branch.
PR labels are unavailable to merge-group events; methodology's PR-label
enforcement is performed during explicit PR qualification and must not be
represented as queue-side label validation.
