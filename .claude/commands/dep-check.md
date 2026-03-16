---
description: Analyze dependencies — unused deps, security advisories, license compliance, supply chain health
argument-hint: ""
---

# Dependency Check

Analyze workspace dependencies for health, security, and compliance.

## Steps

1. **Check for unused dependencies**:
   ```bash
   cargo machete
   ```

2. **Check for security advisories**:
   ```bash
   cargo audit
   just security-audit
   ```

3. **Check SemVer compliance**:
   ```bash
   just semver-check
   ```

4. **Review supply chain policy** in `deny.toml`:
   - License allow-list is appropriate
   - No policy gaps for new dependencies

5. **Check for duplicates** in the transitive dependency tree:
   - Look for multiple versions of the same crate
   - Identify consolidation opportunities

6. **Check version freshness**:
   - Any dependencies more than one major version behind?
   - Any dependencies with known end-of-life?

## Key Files

- `deny.toml` — supply chain policy
- `Cargo.lock` — pinned versions
- `docs/reference/SUPPLY_CHAIN_SECURITY.md` — security docs

## Generate SBOM (if needed)

```bash
just sbom
```
