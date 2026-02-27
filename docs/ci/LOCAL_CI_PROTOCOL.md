# Local CI Protocol

Canonical local CI workflow for this repository:

- **Every mergeable change**: run `just ci-gate`
- **Large or risky changes**: follow with `just ci-full`
- **Policy-only checks**: `just ci-policy`
- **Archive-level CI summary**: keep `docs/CI_STATUS_214.md` and `docs/ci/LOCAL_CI_SUMMARY.md` in sync with outcomes

For details and examples, use this protocol as the canonical source:
- `just ci-gate` is the required baseline merge gate
- `just ci-full` adds the broader, slower pass when release confidence is needed
