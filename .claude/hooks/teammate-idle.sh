#!/bin/bash
# TeammateIdle hook: keeps swarm teammates working
# Exit 2 = send feedback and keep teammate active
# Exit 0 = allow teammate to go idle

# Check if there are unclaimed tasks in the task list
# If so, nudge the teammate to claim one
echo "Check the task list for unclaimed tasks. If tasks are available, claim the next one that doesn't overlap with your active work. If no tasks available, launch subagents to discover new work: use Explore agents to find gaps, then create tasks from the results. If you are a merger, check for green PRs. If you are a janitor, check for stale worktrees."
exit 2
