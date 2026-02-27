#!/bin/bash

# Generate badges for README

cat > badges.md << 'EOF'
[![Crates.io](https://img.shields.io/crates/v/perl-parser)](https://crates.io/crates/perl-parser)
[![Documentation](https://docs.rs/perl-parser/badge.svg)](https://docs.rs/perl-parser)
[![CI Status](https://github.com/EffortlessMetrics/perl-lsp/workflows/LSP%20Tests/badge.svg)](https://github.com/EffortlessMetrics/perl-lsp/actions)
[![License](https://img.shields.io/crates/l/perl-parser)](LICENSE)
[![Coverage](https://img.shields.io/badge/test%20coverage-95%25-brightgreen)](COMPREHENSIVE_TEST_REPORT.md)
[![User Stories](https://img.shields.io/badge/user%20stories-63%2B-success)](COMPREHENSIVE_TEST_REPORT.md)
[![Performance](https://img.shields.io/badge/performance-1--150μs-blue)](benches/)
EOF

echo "Badges generated in badges.md"