#!/usr/bin/env python3
"""Apply all changes for Const::Fast readonly semantic tokens feature."""

# ====== symbol.rs ======
symbol_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/symbol.rs'
with open(symbol_path, 'r') as f:
    content = f.read()

# Check if already modified
if 'kind_filter: Option<SymbolKind>' in content:
    print("symbol.rs already has kind_filter - skipping")
else:
    # Replace the function signature
    old_sig = 'pub fn find_symbol(&self, name: &str, from_scope: ScopeId, kind: SymbolKind) -> Vec<&Symbol> {'
    new_sig = 'pub fn find_symbol(&self, name: &str, from_scope: ScopeId, kind_filter: Option<SymbolKind>) -> Vec<&Symbol> {'
    content = content.replace(old_sig, new_sig)
    
    # Replace the first condition check
    content = content.replace(
        'if symbol.scope_id == scope_id && symbol.kind == kind {',
        'if symbol.scope_id == scope_id && kind_filter.map_or(true, |k| symbol.kind == k) {'
    )
    
    # Replace the 'our' variable condition check
    content = content.replace(
        'if symbol.declaration.as_deref() == Some("our") && symbol.kind == kind {',
        'if symbol.declaration.as_deref() == Some("our") && kind_filter.map_or(true, |k| symbol.kind == k) {'
    )
    
    with open(symbol_path, 'w') as f:
        f.write(content)
    print("symbol.rs updated")

# ====== references.rs ======
ref_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/semantic/references.rs'
with open(ref_path, 'r') as f:
    content = f.read()

# Check if already modified
if 'Some(reference.kind),' in content and 'Some(Some(reference.kind))' not in content:
    print("references.rs already has Some(reference.kind) - skipping")
else:
    # Fix double Some first
    content = content.replace('Some(Some(reference.kind))', 'Some(reference.kind)')
    
    # Then add Some where needed - be careful to only replace bare reference.kind
    # The pattern is: find_symbol(..., reference.kind,) or find_symbol(..., reference.kind)
    # We need to replace reference.kind at find_symbol call sites only
    
    lines = content.split('\n')
    new_lines = []
    i = 0
    while i < len(lines):
        line = lines[i]
        # Check if this is a find_symbol call line with reference.kind
        if 'find_symbol' in line and 'reference.kind' in line and 'Some' not in line:
            # This is a line that needs fixing
            if 'reference.kind,' in line:
                line = line.replace('reference.kind,', 'Some(reference.kind),')
            elif 'reference.kind)' in line:
                line = line.replace('reference.kind)', 'Some(reference.kind))')
        new_lines.append(line)
        i += 1
    
    content = '\n'.join(new_lines)
    
    with open(ref_path, 'w') as f:
        f.write(content)
    print("references.rs updated")

# ====== mod.rs ======
mod_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/semantic/mod.rs'
with open(mod_path, 'r') as f:
    content = f.read()

# Check if already modified
if 'Some(crate::symbol::SymbolKind::Subroutine))' in content:
    print("mod.rs already has Some() wrappers - skipping")
else:
    # Add Some() wrappers to find_symbol calls for SymbolKind::Subroutine
    # But NOT the matches! lines
    lines = content.split('\n')
    new_lines = []
    i = 0
    while i < len(lines):
        line = lines[i]
        # Skip matches! lines
        if 'matches!(s.kind,' in line:
            new_lines.append(line)
            i += 1
            continue
        # Fix find_symbol calls with SymbolKind::Subroutine
        if 'find_symbol' in line and 'SymbolKind::Subroutine' in line:
            # Multi-line case
            if line.strip().endswith('SymbolKind::Subroutine)'):
                line = line.replace('SymbolKind::Subroutine)', 'Some(SymbolKind::Subroutine))')
            else:
                # Single line case with SymbolKind::Subroutine as argument
                line = line.replace('crate::symbol::SymbolKind::Subroutine)', 'Some(crate::symbol::SymbolKind::Subroutine))')
        new_lines.append(line)
        i += 1
    
    content = '\n'.join(new_lines)
    
    with open(mod_path, 'w') as f:
        f.write(content)
    print("mod.rs updated")

# ====== comprehensive_unit_tests.rs ======
test_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/tests/comprehensive_unit_tests.rs'
with open(test_path, 'r') as f:
    content = f.read()

# Check if already modified
if 'Some(SymbolKind::scalar()))' in content:
    print("comprehensive_unit_tests.rs already has Some() wrappers - skipping")
else:
    content = content.replace(
        'SymbolKind::scalar()))',
        'Some(SymbolKind::scalar()))'
    )
    content = content.replace(
        'SymbolKind::Subroutine)',
        'Some(SymbolKind::Subroutine))'
    )
    
    with open(test_path, 'w') as f:
        f.write(content)
    print("comprehensive_unit_tests.rs updated")

# ====== node_analysis.rs ======
node_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/semantic/node_analysis.rs'
with open(node_path, 'r') as f:
    content = f.read()

# First, fix find_symbol calls to use Some(kind)
# Be careful to not introduce double Some
if 'Some(kind))' in content and 'Some(Some(kind))' not in content:
    print("node_analysis.rs find_symbol calls already have Some() - skipping")
else:
    # Fix double Some first
    content = content.replace('Some(Some(kind))', 'Some(kind)')
    
    # Then fix bare kind at find_symbol call sites
    lines = content.split('\n')
    new_lines = []
    for line in lines:
        if 'find_symbol' in line and 'kind)' in line and 'Some' not in line:
            # This line has find_symbol with bare kind
            if 'kind)' in line:
                line = line.replace('kind)', 'Some(kind))')
        new_lines.append(line)
    
    content = '\n'.join(new_lines)
    
    with open(node_path, 'w') as f:
        f.write(content)
    print("node_analysis.rs find_symbol calls updated")

# Now add the VariableReadonly semantic token logic
# This is more complex - we need to replace specific code blocks

# Check if already modified
if 'VariableReadonly' in content:
    print("node_analysis.rs already has VariableReadonly - skipping")
else:
    # Replace Variable reference handling
    old_var_ref = '''            NodeKind::Variable { sigil, name } => {
                let kind = match sigil.as_str() {
                    "$" => SymbolKind::scalar(),
                    "@" => SymbolKind::array(),
                    "%" => SymbolKind::hash(),
                    _ => return,
                };

                // Find the symbol definition
                let symbols = self.symbol_table.find_symbol(name, scope_id, Some(kind));

                let token_type = if let Some(symbol) = symbols.first() {
                    match symbol.declaration.as_deref() {
                        Some("my") | Some("state") => SemanticTokenType::Variable,
                        Some("our") => SemanticTokenType::Variable,
                        _ => SemanticTokenType::Variable,
                    }
                } else {
                    // Undefined variable
                    SemanticTokenType::Variable
                };

                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type,
                    modifiers: vec![],
                });

                // Add hover info if we found the symbol'''

    new_var_ref = '''            NodeKind::Variable { sigil, name } => {
                let kind = match sigil.as_str() {
                    "$" => SymbolKind::scalar(),
                    "@" => SymbolKind::array(),
                    "%" => SymbolKind::hash(),
                    _ => return,
                };

                // Find the symbol definition - first try with sigil-derived kind
                let symbols = self.symbol_table.find_symbol(name, scope_id, Some(kind));

                // If not found, retry without kind filter to find constants
                // (Const::Fast/Readonly variables are stored with kind=Constant)
                let symbols = if symbols.is_empty() {
                    self.symbol_table.find_symbol(name, scope_id, None)
                } else {
                    symbols
                };

                // Determine token type and modifiers based on symbol
                let (token_type, modifiers) = if let Some(symbol) = symbols.first() {
                    match symbol.declaration.as_deref() {
                        // Const::Fast and Readonly module variables emit VariableReadonly
                        Some("const") | Some("Readonly") => (
                            SemanticTokenType::VariableReadonly,
                            vec![SemanticTokenModifier::Readonly],
                        ),
                        Some("my") | Some("state") => (SemanticTokenType::Variable, vec![]),
                        Some("our") => (SemanticTokenType::Variable, vec![]),
                        _ => (SemanticTokenType::Variable, vec![]),
                    }
                } else {
                    // Undefined variable
                    (SemanticTokenType::Variable, vec![])
                };

                self.semantic_tokens.push(SemanticToken {
                    location: node.location,
                    token_type,
                    modifiers,
                });

                // Add hover info if we found the symbol'''

    if old_var_ref in content:
        content = content.replace(old_var_ref, new_var_ref)
        print("node_analysis.rs Variable reference handling updated")
    else:
        print("WARNING: Could not find Variable reference handling to replace")

    # Replace VariableDeclaration handling
    old_var_decl = '''            NodeKind::VariableDeclaration { declarator, variable, attributes, initializer } => {
                // Add semantic token for declaration
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    let token_type = match declarator.as_str() {
                        "my" | "state" => SemanticTokenType::VariableDeclaration,
                        "our" => SemanticTokenType::Variable,
                        "local" => SemanticTokenType::Variable,
                        _ => SemanticTokenType::Variable,
                    };

                    let mut modifiers = vec![SemanticTokenModifier::Declaration];
                    if declarator == "state" || attributes.iter().any(|a| a == ":shared") {
                        modifiers.push(SemanticTokenModifier::Static);
                    }

                    self.semantic_tokens.push(SemanticToken {
                        location: variable.location,
                        token_type,
                        modifiers,
                    });

                    // Add hover info
                    let hover = HoverInfo {
                        signature: format!("{} {}{}", declarator, sigil, name),
                        documentation: self.extract_documentation(node.location.start),
                        details: if attributes.is_empty() {
                            vec![]
                        } else {
                            vec![format!("Attributes: {}", attributes.join(", "))]
                        },
                    };

                    self.hover_info.insert(variable.location, hover);
                }

                if let Some(init) = initializer {
                    self.analyze_node(init, scope_id);
                }
            }'''

    new_var_decl = '''            NodeKind::VariableDeclaration { declarator, variable, attributes, initializer } => {
                // Add semantic token for declaration
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    // Look up the symbol to check if it's a Const::Fast/Readonly constant
                    let symbols = self.symbol_table.find_symbol(name, scope_id, None);
                    let is_constant = symbols.first().map_or(false, |s| {
                        s.kind == SymbolKind::Constant
                            && matches!(s.declaration.as_deref(), Some("const") | Some("Readonly"))
                    });

                    let (token_type, modifiers) = if is_constant {
                        // Const::Fast/Readonly variables emit VariableReadonly
                        (
                            SemanticTokenType::VariableReadonly,
                            vec![SemanticTokenModifier::Readonly, SemanticTokenModifier::Declaration],
                        )
                    } else {
                        match declarator.as_str() {
                            "my" | "state" => (
                                SemanticTokenType::VariableDeclaration,
                                {
                                    let mut m = vec![SemanticTokenModifier::Declaration];
                                    if declarator == "state" || attributes.iter().any(|a| a == ":shared") {
                                        m.push(SemanticTokenModifier::Static);
                                    }
                                    m
                                },
                            ),
                            "our" | "local" => (
                                SemanticTokenType::Variable,
                                vec![SemanticTokenModifier::Declaration],
                            ),
                            _ => (
                                SemanticTokenType::Variable,
                                vec![SemanticTokenModifier::Declaration],
                            ),
                        }
                    };

                    self.semantic_tokens.push(SemanticToken {
                        location: variable.location,
                        token_type,
                        modifiers,
                    });

                    // Add hover info
                    let hover = HoverInfo {
                        signature: format!("{} {}{}", declarator, sigil, name),
                        documentation: self.extract_documentation(node.location.start),
                        details: if attributes.is_empty() {
                            vec![]
                        } else {
                            vec![format!("Attributes: {}", attributes.join(", "))]
                        },
                    };

                    self.hover_info.insert(variable.location, hover);
                }

                if let Some(init) = initializer {
                    self.analyze_node(init, scope_id);
                }
            }'''

    if old_var_decl in content:
        content = content.replace(old_var_decl, new_var_decl)
        print("node_analysis.rs VariableDeclaration handling updated")
    else:
        print("WARNING: Could not find VariableDeclaration handling to replace")

    # Replace VariableListDeclaration handling
    old_var_list_decl = '''            // Phase 1: Critical LSP Features (Issue #188)
            NodeKind::VariableListDeclaration {
                declarator,
                variables,
                attributes,
                initializer,
            } => {
                // Handle multi-variable declarations like: my ($x, $y, $z) = (1, 2, 3);
                for var in variables {
                    if let NodeKind::Variable { sigil, name } = &var.kind {
                        let token_type = match declarator.as_str() {
                            "my" | "state" => SemanticTokenType::VariableDeclaration,
                            "our" => SemanticTokenType::Variable,
                            "local" => SemanticTokenType::Variable,
                            _ => SemanticTokenType::Variable,
                        };

                        let mut modifiers = vec![SemanticTokenModifier::Declaration];
                        if declarator == "state" || attributes.iter().any(|a| a == ":shared") {
                            modifiers.push(SemanticTokenModifier::Static);
                        }

                        self.semantic_tokens.push(SemanticToken {
                            location: var.location,
                            token_type,
                            modifiers,
                        });

                        // Add hover info
                        let hover = HoverInfo {
                            signature: format!("{} {}{}", declarator, sigil, name),
                            documentation: self.extract_documentation(var.location.start),
                            details: if attributes.is_empty() {
                                vec![]
                            } else {
                                vec![format!("Attributes: {}", attributes.join(", "))]
                            },
                        };

                        self.hover_info.insert(var.location, hover);
                    }
                }

                if let Some(init) = initializer {
                    self.analyze_node(init, scope_id);
                }
            }'''

    new_var_list_decl = '''            // Phase 1: Critical LSP Features (Issue #188)
            NodeKind::VariableListDeclaration {
                declarator,
                variables,
                attributes,
                initializer,
            } => {
                // Handle multi-variable declarations like: my ($x, $y, $z) = (1, 2, 3);
                for var in variables {
                    if let NodeKind::Variable { sigil, name } = &var.kind {
                        // Look up the symbol to check if it's a Const::Fast/Readonly constant
                        let symbols = self.symbol_table.find_symbol(name, scope_id, None);
                        let is_constant = symbols.first().map_or(false, |s| {
                            s.kind == SymbolKind::Constant
                                && matches!(s.declaration.as_deref(), Some("const") | Some("Readonly"))
                        });

                        let (token_type, modifiers) = if is_constant {
                            // Const::Fast/Readonly variables emit VariableReadonly
                            (
                                SemanticTokenType::VariableReadonly,
                                vec![SemanticTokenModifier::Readonly, SemanticTokenModifier::Declaration],
                            )
                        } else {
                            match declarator.as_str() {
                                "my" | "state" => (
                                    SemanticTokenType::VariableDeclaration,
                                    {
                                        let mut m = vec![SemanticTokenModifier::Declaration];
                                        if declarator == "state" || attributes.iter().any(|a| a == ":shared") {
                                            m.push(SemanticTokenModifier::Static);
                                        }
                                        m
                                    },
                                ),
                                "our" | "local" => (
                                    SemanticTokenType::Variable,
                                    vec![SemanticTokenModifier::Declaration],
                                ),
                                _ => (
                                    SemanticTokenType::Variable,
                                    vec![SemanticTokenModifier::Declaration],
                                ),
                            }
                        };

                        self.semantic_tokens.push(SemanticToken {
                            location: var.location,
                            token_type,
                            modifiers,
                        });

                        // Add hover info
                        let hover = HoverInfo {
                            signature: format!("{} {}{}", declarator, sigil, name),
                            documentation: self.extract_documentation(var.location.start),
                            details: if attributes.is_empty() {
                                vec![]
                            } else {
                                vec![format!("Attributes: {}", attributes.join(", "))]
                            },
                        };

                        self.hover_info.insert(var.location, hover);
                    }
                }

                if let Some(init) = initializer {
                    self.analyze_node(init, scope_id);
                }
            }'''

    if old_var_list_decl in content:
        content = content.replace(old_var_list_decl, new_var_list_decl)
        print("node_analysis.rs VariableListDeclaration handling updated")
    else:
        print("WARNING: Could not find VariableListDeclaration handling to replace")

    with open(node_path, 'w') as f:
        f.write(content)

print("\nAll changes complete!")
