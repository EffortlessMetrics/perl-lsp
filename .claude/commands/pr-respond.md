---
description: Address PR review comments — read feedback, fix issues, push updates
argument-hint: "<PR number>"
---

# PR Respond

Read and address review comments on PR **$ARGUMENTS**.

## Process

### 1. Read the review feedback
```bash
gh pr view $ARGUMENTS --comments
gh api repos/:owner/:repo/pulls/$ARGUMENTS/reviews --jq '.[] | "\(.user.login) (\(.state)): \(.body)"'
gh api repos/:owner/:repo/pulls/$ARGUMENTS/comments --jq '.[] | "\(.path):\(.line) \(.body)"'
```

### 2. Categorize each comment
- **Blocking** (changes requested): must fix before merge
- **Suggestion**: apply if it's an improvement, explain if you disagree
- **Question**: answer in a reply comment
- **Nitpick**: fix if trivial, skip if subjective

### 3. Address blocking comments
For each blocking comment:
1. Read the file and understand the concern
2. Make the fix
3. Commit: `fix(review): address <reviewer> feedback — <what>`

### 4. Reply to comments
```bash
gh api repos/:owner/:repo/pulls/$ARGUMENTS/comments/<comment-id>/replies \
  -f body="Fixed in <commit-hash>. <brief explanation>"
```

Or for general review comments:
```bash
gh pr comment $ARGUMENTS --body "Addressed review feedback:
- <comment 1>: fixed in <hash>
- <comment 2>: <explanation>
"
```

### 5. Re-verify
```bash
cargo xtask fmt
cargo clippy -p <crate> --tests -- -D warnings
cargo test -p <crate>
```

### 6. Push and request re-review
```bash
git push
gh pr edit $ARGUMENTS --add-reviewer <original-reviewer> 2>/dev/null || true
```
