# Task List — work-eb418345

## Implementation Tasks

- [ ] 1. Read current file state for `scout-parser.md` (verify lines match issue description)
- [ ] 2. Patch `scout-parser.md` — move `## Domain context` before `## Todo list`
- [ ] 3. Patch `scout-dap.md` — move `## Domain context` before `## Todo list`
- [ ] 4. Patch `accuracy-scout.md` — move `## Invocation` before `## Todo list`
- [ ] 5. Patch `scout-lsp.md` — move `## Domain context` and `## Write to think...` before `## Todo list`
- [ ] 6. Run verification script to confirm all 4 files end with `## Todo list`
- [ ] 7. Commit with message: `fix(agents): reorder definition sections so todo lists are final (#4387)`
- [ ] 8. Push branch to origin

## Verification Command
```bash
for file in scout-parser scout-dap accuracy-scout scout-lsp; do
  path=".claude/agents/$file.md"
  last_section=$(grep "^## " "$path" | tail -1)
  [ "$last_section" = "## Todo list" ] && echo "PASS: $file.md" || echo "FAIL: $file.md"
done
```
