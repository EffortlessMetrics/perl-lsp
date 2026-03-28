---
description: Scout for security issues — banned constructs, unsafe blocks, dep vulnerabilities, supply chain
argument-hint: ""
---

# Security Scout

Scout for security issues across the codebase. READ ONLY — returns findings, does not modify code.

## Steps

1. **Run automated checks**:
   ```bash
   cargo audit 2>&1                       # Known vulnerabilities
   cargo machete 2>&1                     # Unused deps (attack surface reduction)
   ```

2. **Grep for banned constructs** in production code (exclude tests):
   - `unwrap()/expect()` in production code
   - `unsafe` blocks without justification
   - `std::process::exit()` outside `bin/` and `lifecycle.rs`
   - `std::process::abort()` anywhere
   - `dbg!()` anywhere

3. **Check for common vulnerabilities**:
   - Path traversal risks in file handling
   - Hardcoded secrets or credentials
   - Outdated deps with known CVEs

4. **Review supply chain policy**:
   - Check `deny.toml` for policy gaps
   - Verify SBOM is current

5. **Return findings** with severity (critical/high/medium/low) for each issue

## Output

Use the **Full Scout Report** variant from `/scout-issue`. Add a `## Severity` section immediately after Problem, with one line per finding:

```
## Severity

- **[critical/high/medium/low]** <finding name> — <one-line justification>
```
