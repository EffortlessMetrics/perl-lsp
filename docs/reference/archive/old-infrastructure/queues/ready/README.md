# Ready-to-Build Queue

Branch-ready work packets. Builders pull from here when CI capacity is available.

## Packet Format
Each file: `<branch-name>.md`

```yaml
target: <crate or subsystem>
files:
  - <path/to/file1.rs>
  - <path/to/file2.rs>
verification: cargo fmt && cargo clippy -p <crate> --tests && cargo test -p <crate>
pr_title: "<conventional commit title>"
ci_cost: none | low | normal | heavy
depends_on: []  # other packets or PRs that must land first
risk: <low | medium | high>
```

## Body
Problem statement, approach, test template, known pitfalls.
