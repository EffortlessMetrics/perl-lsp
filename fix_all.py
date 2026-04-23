#!/usr/bin/env python3
"""Fix ALL remaining find_symbol calls in all files."""

import re

# Fix mod.rs
mod_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/semantic/mod.rs'
with open(mod_path, 'r') as f:
    content = f.read()

# Replace all find_symbol calls with SymbolKind::XXX without Some()
# Pattern: find_symbol(..., SymbolKind::XXX) -> find_symbol(..., Some(SymbolKind::XXX))
# But skip matches! patterns

lines = content.split('\n')
new_lines = []
i = 0
while i < len(lines):
    line = lines[i]
    # Skip matches! lines
    if 'matches!' in line:
        new_lines.append(line)
        i += 1
        continue
    # Fix find_symbol calls
    if 'find_symbol' in line and 'SymbolKind::' in line and 'Some(' not in line:
        # Check for SymbolKind::scalar() or SymbolKind::Subroutine
        if 'SymbolKind::scalar()' in line:
            line = line.replace('SymbolKind::scalar()', 'Some(SymbolKind::scalar())')
        if 'SymbolKind::Subroutine' in line and 'Some(' not in line:
            line = line.replace('SymbolKind::Subroutine)', 'Some(SymbolKind::Subroutine))')
    new_lines.append(line)
    i += 1

content = '\n'.join(new_lines)

with open(mod_path, 'w') as f:
    f.write(content)

print("mod.rs fixed")

# Fix node_analysis.rs  
node_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/semantic/node_analysis.rs'
with open(node_path, 'r') as f:
    content = f.read()

lines = content.split('\n')
new_lines = []
for line in lines:
    if 'find_symbol' in line and 'SymbolKind::' in line and 'Some(' not in line:
        if 'SymbolKind::scalar()' in line:
            line = line.replace('SymbolKind::scalar()', 'Some(SymbolKind::scalar())')
        if 'SymbolKind::Subroutine' in line:
            line = line.replace('SymbolKind::Subroutine)', 'Some(SymbolKind::Subroutine))')
    new_lines.append(line)

content = '\n'.join(new_lines)

with open(node_path, 'w') as f:
    f.write(content)

print("node_analysis.rs fixed")
