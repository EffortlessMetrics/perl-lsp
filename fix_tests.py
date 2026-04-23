#!/usr/bin/env python3
"""Fix ALL remaining find_symbol calls in test files."""

import os
import re

test_files = [
    '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/tests/real_world_patterns.rs',
    '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/tests/extended_unit_tests.rs',
    '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/tests/scope_and_symbol_tests.rs',
]

for filepath in test_files:
    if not os.path.exists(filepath):
        print(f"File not found: {filepath}")
        continue
    
    with open(filepath, 'r') as f:
        content = f.read()
    
    original = content
    
    # Replace SymbolKind::scalar() with Some(SymbolKind::scalar())
    # But NOT if already has Some()
    if 'SymbolKind::scalar()' in content and 'Some(SymbolKind::scalar())' not in content:
        content = content.replace('SymbolKind::scalar()', 'Some(SymbolKind::scalar())')
    
    # Replace SymbolKind::Subroutine with Some(SymbolKind::Subroutine)
    # But NOT if already has Some()
    if 'SymbolKind::Subroutine' in content and 'Some(SymbolKind::Subroutine)' not in content:
        content = content.replace('SymbolKind::Subroutine)', 'Some(SymbolKind::Subroutine))')
    
    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"Fixed: {filepath}")
    else:
        print(f"No changes needed: {filepath}")

print("\nDone!")
