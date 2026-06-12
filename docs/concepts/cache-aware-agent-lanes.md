# Cache-Aware Agent Lanes

## The pattern

LLM inference costs vary dramatically depending on whether the input tokens are cached.
Cached tokens cost roughly one-tenth as much as fresh tokens (the exact ratio varies by
provider, but the order of magnitude is stable). A cache window stays warm for approximately
five minutes of inactivity.

This creates two distinct architectural modes for orchestrating agent work:

**Fan-out** — spawn N agents at the same moment to work in parallel. Each agent starts cold.
Best for breadth at a given point in time: when you need many independent subtasks completed
simultaneously and the tasks do not build on each other.

**Lane** — keep a single agent alive across a sequence of related tasks, re-feeding it with
each new task within the cache window. The first invocation is cold; subsequent invocations
within the window are warm and cost ~10x less. Best for depth over time: when tasks build on
each other and share a large common context (the code being analyzed, the spec, the prior
output).

## The economics

For a single agent with 100k tokens of shared context (code, spec, prior output):

- **Cold spawn (fan-out each time)**: 100k fresh tokens per invocation
- **Warm lane (re-fed within window)**: ~10k equivalent cost per invocation

If a pipeline has 6 sequential passes over the same context, a lane costs ~6x cheaper than
spawning 6 independent agents. The total cost is comparable to 1 cold spawn, not 6.

This is why long-running verification pipelines — a sequence of passes each building on
the previous one's output — benefit from the lane pattern rather than spawning one agent
per pass.

## When to use each mode

Use **fan-out** when:
- Tasks are independent (neither reads the other's output)
- Breadth is needed at a specific moment (e.g., exploring multiple candidate approaches)
- Tasks do not share a large common context

Use **lanes** when:
- Tasks are sequential and each builds on the previous output
- All tasks share a large common context (same codebase, same spec, same prior analysis)
- The sequence fits within a single session (or can be paced to stay within the cache window)
- Cost reduction matters and latency is not the primary constraint

## The caution

A lane left idle for more than five minutes loses its cache and the next invocation pays
full cold cost — negating the lane advantage. Do not hold a lane open "just in case"; feed
it with the next task promptly when the prior one completes.

A lane doing nothing to stay warm (synthetic keep-alive messages) wastes tokens without
producing value. The cache window is earned by productive work, not by idle presence.

If a lane's shared context is small (under ~10k tokens), the per-invocation savings are
modest. Fan-out may be simpler and equally economical in those cases.

## Sequencing for cache efficiency

When a pipeline has a fixed sequence of passes, structure the orchestration to complete one
pass and immediately feed the next within the same agent session rather than closing and
re-opening. Completion events should trigger the next step, not a cold re-spawn.

Batching related tasks that share context into a single agent session — rather than spawning
a new agent for each task — is the primary lever for cost reduction in high-volume pipelines.

## Relation to other patterns

- **Serialize merges** (`serialize-merges-and-cancellation.md`) — independent concern;
  serialization prevents CI cancellation cascades, not a cost optimization.
- **Multi-angle early spec** (`multi-angle-haiku-early-spec.md`) — a fan-out pattern applied
  to spec construction; each angle is independent so fan-out is correct there.
