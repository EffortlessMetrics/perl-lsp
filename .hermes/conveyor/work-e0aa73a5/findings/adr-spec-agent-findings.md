# ADR/Spec Agent Findings — work-e0aa73a5

## What This ADR Decides
Implement Phase 1 of schema migration support as file discovery ONLY, using a separate `is_migration_discovery_path()` function rather than polluting `PERL_SOURCE_EXTENSIONS`. SQL highlighting and document links are deferred to Phase 2 due to architectural complexity.

## Key Decision
Phase 1 scope is reduced to file discovery via a new `is_migration_discovery_path()` function that checks path components for migration directory patterns. `.sql` is NOT added to `PERL_SOURCE_EXTENSIONS`. SQL highlighting and document links are explicitly out of scope.

## Alternatives Considered
1. **Full SQL highlighting pipeline** — Rejected: too complex, SQL highlighting available natively in editors
2. **Add `.sql` to PERL_SOURCE_EXTENSIONS** — Rejected: semantic error, confuses Perl source concept
3. **Defer entirely** — Rejected: file discovery provides immediate value without complexity
4. **Global `.sql` discovery** — Rejected: over-breadth, indexes fixtures and documentation

## Consequences
- **Positive**: Architecturally sound, low risk, Perl source concept stays clean, phased approach
- **Negative**: Limited immediate value (no highlighting/navigation), fragile path matching
- **Tradeoff**: Accept fragile path component checking for workable Phase 1 MVP

## Acceptance Criteria
1. AC1: DeploymentHandler paths (`share/deploy/`, `share/upgrade/`, `share/revert/*.sql`) are discovered
2. AC2: Sqitch paths (`deploy/`, `verify/`, `revert/`, `sqitch.plan`) are discovered
3. AC3: Non-migration SQL paths are NOT discovered
4. AC4: Skip list compatibility verified
5. AC5: `.sql` NOT added to `PERL_SOURCE_EXTENSIONS`
6. AC6: No new feature entries in `features.toml`
