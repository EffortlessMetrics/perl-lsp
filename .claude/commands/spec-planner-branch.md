---
description: Spec planner step 4 — create branch, commit spec files, push
user-invocable: false
---

# Spec Planner: Branch

Create the implementation branch and commit the spec files.

## Steps

1. Create branch from master:
   ```bash
   git checkout -b impl/<issue#>-<specslug> origin/master
   ```

2. Create the spec directory:
   ```bash
   mkdir -p .spec/<issue#>-<specslug>
   ```

3. Write the three spec files (from `/spec-planner-plan` output):
   - `.spec/<issue#>-<specslug>/checklist.md`
   - `.spec/<issue#>-<specslug>/acceptance.md`
   - `.spec/<issue#>-<specslug>/context.md`

4. Commit:
   ```bash
   git add .spec/<issue#>-<specslug>/
   git commit -m "$(cat <<'EOF'
   plan(<crate>): add implementation spec for #<issue>

   Checklist, acceptance criteria, and context for the red TDD builder
   and builder to reference. See .spec/<issue#>-<specslug>/ for details.
   EOF
   )"
   ```

5. Push:
   ```bash
   git push -u origin impl/<issue#>-<specslug>
   ```

## Notes

- The branch name `impl/<issue#>-<specslug>` is the convention. Don't deviate.
- If the branch already exists (from a previous attempt), check it out and rebase on master instead of creating fresh.
- The spec files stay in the repo permanently — they're cheap historical context.
