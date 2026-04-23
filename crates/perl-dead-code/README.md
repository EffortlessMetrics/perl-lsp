# perl-dead-code

Dead code detection for Perl source code. Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## Features

- **Dead code detection** -- `DeadCodeDetector` identifies unused subroutines, variables, constants, packages, modules and unreachable code.

## Dependencies

Builds on `perl-workspace` (cross-file references).

## Usage

```rust
use perl_dead_code::DeadCodeDetector;
use perl_workspace::workspace_index::WorkspaceIndex;
use std::path::PathBuf;

let mut index = WorkspaceIndex::new();
// ... populate index ...

let mut detector = DeadCodeDetector::new(index);
detector.add_entry_point(PathBuf::from("script.pl"));

let analysis = detector.analyze_workspace();
println!("Unused subroutines: {}", analysis.stats.unused_subroutines);
```

## License

Licensed under MIT OR Apache-2.0 at your option.
