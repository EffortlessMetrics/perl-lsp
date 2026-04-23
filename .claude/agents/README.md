# Agent Definitions

The swarm orchestrator calls agents, and these agent files are part of the
live swarm design.

Each agent file is responsible for the role-specific layer:

- lane ownership
- communication patterns
- startup checklist
- active todo shape
- dispatch or escalation rules

Those todos then call specific skills or commands for the procedural steps.
That keeps the split clean:

- agents provide the "who/what/when" context
- skills and commands provide the reusable "how" for each step

This minimizes how much the orchestrator has to re-encode on every spawn. The
orchestrator routes work to the right agent, and the agent file supplies the
role framing plus the todo list that names the required skills.

It also keeps the mechanical substep instructions loadable on demand. When a
specific step becomes relevant midway through the run, the agent can load the
matching skill then, rather than carrying every detailed procedure in the
initial agent prompt.

Legacy directories archived to `docs/reference/archive/` during architecture transition.

## What These Files Are Good For

These files are useful for:
- Understanding agent roles and lane ownership
- Seeing which skills each agent type invokes first
- Reconstructing how the swarm minimizes orchestrator burden by pushing role
  instructions into reusable agent definitions
