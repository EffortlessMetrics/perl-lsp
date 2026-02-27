# Perl Workspace Index SLO

This microcrate provides production-readiness SLO tracking for workspace index operations:

- operation latency histograms and percentiles
- error-rate tracking
- coarse SLO compliance reporting
- per-operation timing context management

It is intentionally narrow in scope so it can be used independently of the broader
workspace indexing engine.
