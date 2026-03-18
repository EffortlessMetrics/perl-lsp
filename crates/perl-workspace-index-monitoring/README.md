# Perl Workspace Index Monitoring

This microcrate provides the narrow monitoring/configuration layer for workspace
index coordination:

- resource-limit configuration
- performance-cap budgets
- pending-parse metrics
- lifecycle instrumentation snapshots and counters

It exists to keep `perl-workspace-index` focused on symbol indexing and query
behavior while preserving reusable coordinator primitives.
