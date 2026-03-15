# Patterns for Effective Agent Dispatch

Hard-won lessons from running agent swarms on real codebases.

## Core patterns

### 1. Worktree isolation

Always use `isolation: "worktree"` for any agent that writes code.

**Why**: Two agents editing the same git index will corrupt each other's work.
Worktrees give each agent a clean, independent working copy. They share the
same git object store, so they're cheap to create and fast to set up.

```
Agent(
  prompt: "Fix the date parsing bug",
  mode: "auto",
  isolation: "worktree",
  run_in_background: true,
  name: "fix-date-parsing"
)
```

**Anti-pattern**: Running multiple agents in the same checkout with
`isolation: "none"`. They will overwrite each other's changes.

### 2. Background dispatch for parallelism

Use `run_in_background: true` for agents that should run in parallel.

```
# Launch 4 agents simultaneously
Agent(prompt: "Task A", isolation: "worktree", run_in_background: true, name: "task-a")
Agent(prompt: "Task B", isolation: "worktree", run_in_background: true, name: "task-b")
Agent(prompt: "Task C", isolation: "worktree", run_in_background: true, name: "task-c")
Agent(prompt: "Task D", isolation: "worktree", run_in_background: true, name: "task-d")
```

Wait for all to complete before launching the PR wave. The orchestrator
receives a notification when each background agent finishes.

### 3. Name agents descriptively

Use the `name` parameter so you can track which agent is doing what.

Good names:
- `fix-null-pointer-in-tokenizer`
- `add-tests-for-auth-module`
- `update-api-docs-v3`
- `remove-dead-code-utils`

Bad names:
- `agent-1`
- `worker`
- `task`

### 4. PR pipeline: separate implementation from publication

Split the work into two phases:

**Phase 1 -- Implementation agents** write code:
- TDD fix agents
- Test coverage agents
- Documentation agents
- Cleanup agents

**Phase 2 -- PR agents** publish the results:
- Validate (fmt, lint, test)
- Create branch with conventional name
- Commit with descriptive message
- Push and create pull request

This separation means implementation agents can focus purely on correctness,
and PR agents apply a uniform publication process.

### 5. Wave pattern

Organize work in waves by category:

```
Wave 1: Bug fixes (highest priority, most likely to conflict with each other)
  -> Bulk PR -> Merge all -> Rebase remaining worktrees

Wave 2: Test coverage (builds on fixes, unlikely to conflict)
  -> Bulk PR -> Merge all

Wave 3: Documentation (depends on final code shape)
  -> Bulk PR -> Merge all

Wave 4: Cleanup (safe to do last)
  -> Bulk PR -> Merge all
```

**Why waves matter**: Bug fixes often touch the same files. By doing them
first and merging before the next wave, you avoid conflicts between fix agents
and test agents.

### 6. Quality ratcheting

After each merge wave, update your baselines:

```bash
# Example: test count ratchet
current_tests=$(grep -c "#\[test\]" src/**/*.rs)
echo "$current_tests" > .ci/test-count-baseline.txt
git add .ci/test-count-baseline.txt
git commit -m "ci: ratchet test baseline to $current_tests"
```

Ratcheting ensures that improvements from the swarm are permanent. The next
CI run will fail if someone drops below the new baseline.

Things worth ratcheting:
- Test count
- Lint warning count (should be zero)
- Code coverage percentage
- Clean file count in a corpus sweep
- Dead code count (should only decrease)

### 7. Orchestrator role

The main thread (orchestrator) should only:

- Decide what work needs doing
- Dispatch agents with clear, scoped prompts
- Track completions
- Launch PR waves
- Coordinate merges
- Update baselines

The orchestrator should never:

- Edit code directly
- Run tests (that's the agents' job)
- Make implementation decisions (agents decide how to fix)

## Advanced patterns

### Dependency chains

When one agent's output feeds another:

```
# Phase 1: Generate
Agent(prompt: "Create the API schema", isolation: "worktree", name: "schema")
# Wait for completion

# Phase 2: Consume (PR the schema first, then launch consumers)
Agent(prompt: "PR the schema worktree", name: "pr-schema")
# Wait, merge

# Phase 3: Parallel consumers on the merged result
Agent(prompt: "Generate client from schema", isolation: "worktree", run_in_background: true)
Agent(prompt: "Generate server stubs from schema", isolation: "worktree", run_in_background: true)
Agent(prompt: "Generate docs from schema", isolation: "worktree", run_in_background: true)
```

### Conflict resolution

When two agents touched the same file:

1. Merge the first PR
2. The second worktree needs a rebase:
   ```bash
   cd .claude/worktrees/agent-xyz
   git fetch origin
   git rebase origin/master
   ```
3. If the rebase has conflicts, dispatch a fix-up agent:
   ```
   Agent(
     prompt: "Resolve merge conflicts in this worktree and verify tests pass",
     isolation: "none",  # work in the existing worktree
     name: "resolve-conflicts"
   )
   ```

### Retry with narrower scope

If an agent fails on a broad task, split it:

```
# Too broad -- agent gets confused
Agent(prompt: "Fix all date handling bugs")

# Better -- one bug per agent
Agent(prompt: "Fix: parse_date returns None for ISO 8601 dates with timezone offset")
Agent(prompt: "Fix: date_diff panics when crossing DST boundary")
Agent(prompt: "Fix: format_date drops milliseconds for timestamps before epoch")
```

### Smoke test before PR wave

Before launching the PR wave, do a quick validation:

```bash
# Check which worktrees actually have meaningful changes
for d in .claude/worktrees/agent-*; do
  changes=$(cd "$d" && git diff --stat HEAD 2>/dev/null | tail -1)
  if [ -n "$changes" ]; then
    echo "READY: $(basename $d) | $changes"
  fi
done
```

Skip worktrees that only have debugging artifacts or incomplete work.

## Anti-patterns

### Letting agents coordinate with each other

Agents cannot communicate. Do not prompt one agent to "wait for agent X" or
"check what agent Y did." All coordination goes through the orchestrator.

### Too many agents at once

Diminishing returns set in around 5-8 parallel agents. Beyond that:
- System resources become constrained
- More conflicts to resolve
- Harder to track what's happening

Start with 3-4 agents per wave and scale up if your system handles it.

### Vague prompts

```
# Bad: agent has to guess what "improve" means
Agent(prompt: "Improve the auth module")

# Good: specific, measurable, verifiable
Agent(prompt: "Add unit tests for the JWT token refresh flow in auth/refresh.rs.
Cover: expired token, valid refresh, revoked refresh token, malformed token.")
```

### Mixing concerns in one agent

```
# Bad: too many concerns
Agent(prompt: "Fix the parser bug, add tests, update docs, and clean up dead code")

# Good: one concern per agent
Agent(prompt: "Fix: parser rejects valid heredoc with indented delimiter")
Agent(prompt: "Add heredoc edge case tests for indented and stripped delimiters")
Agent(prompt: "Update heredoc section in syntax-reference.md with new examples")
```
