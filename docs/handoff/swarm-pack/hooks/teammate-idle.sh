#!/bin/bash
# TeammateIdle hook: keeps swarm teammates working
# Exit 2 = send feedback and keep teammate active
# Exit 0 = allow idle

echo "Check the task list for unclaimed tasks. If available, claim the next one that doesn't overlap with your active work. If empty, launch subagents to discover new work slices. If you are ops, check for green PRs and CI drift. If you are the janitor helper, check for stale worktrees."
exit 2
