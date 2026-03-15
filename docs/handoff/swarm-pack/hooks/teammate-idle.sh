#!/bin/bash
# TeammateIdle hook: keeps swarm teammates working
# Exit 2 = send feedback and keep teammate active
# Exit 0 = allow idle

echo "Check the task list for unclaimed tasks. If available, claim the next one that doesn't overlap with your active work. If empty, launch subagents to discover new work slices. If you are the merger, check for green PRs. If you are the janitor, check for stale worktrees."
exit 2
