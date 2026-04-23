#!/usr/bin/env python3
"""Fix remaining find_symbol calls in mod.rs"""

mod_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/semantic/mod.rs'
with open(mod_path, 'r') as f:
    content = f.read()

# Find all find_symbol calls with SymbolKind::Subroutine that don't have Some()
import re

# Pattern to find find_symbol calls with SymbolKind::Subroutine
pattern = r'find_symbol\([^)]*SymbolKind::Subroutine[^)]*\)'

def fix_match(m):
    text = m.group(0)
    # Skip if already has Some()
    if 'Some(' in text:
        return text
    # Add Some() around SymbolKind::Subroutine
    text = text.replace('SymbolKind::Subroutine', 'Some(SymbolKind::Subroutine)')
    return text

content = re.sub(pattern, fix_match, content)

with open(mod_path, 'w') as f:
    f.write(content)

print("mod.rs fixed")

# Now check node_analysis.rs for remaining issues
node_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/semantic/node_analysis.rs'
with open(node_path, 'r') as f:
    content = f.read()

# Check for find_symbol calls with bare SymbolKind::Subroutine
pattern = r'find_symbol\([^)]*SymbolKind::Subroutine[^)]*\)'

def fix_match(m):
    text = m.group(0)
    # Skip if already has Some()
    if 'Some(' in text:
        return text
    # Add Some() around SymbolKind::Subroutine
    text = text.replace('SymbolKind::Subroutine', 'Some(SymbolKind::Subroutine)')
    return text

content = re.sub(pattern, fix_match, content)

with open(node_path, 'w') as f:
    f.write(content)

print("node_analysis.rs fixed")
