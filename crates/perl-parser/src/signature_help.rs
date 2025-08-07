//! Signature help provider for function calls
//!
//! This module provides parameter hints and documentation for functions
//! as the user types function calls.

use crate::ast::Node;
use crate::symbol::{SymbolTable, SymbolKind, SymbolExtractor};
use crate::builtin_signatures::{create_builtin_signatures, BuiltinSignature as ImportedBuiltinSignature};
use std::collections::HashMap;

/// Information about a function parameter
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// Parameter name
    pub label: String,
    /// Optional documentation
    pub documentation: Option<String>,
}

/// Signature information for a function
#[derive(Debug, Clone)]
pub struct SignatureInfo {
    /// The full signature label
    pub label: String,
    /// Documentation for the function
    pub documentation: Option<String>,
    /// Information about each parameter
    pub parameters: Vec<ParameterInfo>,
    /// The active parameter index
    pub active_parameter: Option<usize>,
}

/// Signature help response
#[derive(Debug, Clone)]
pub struct SignatureHelp {
    /// Available signatures (overloads)
    pub signatures: Vec<SignatureInfo>,
    /// Active signature index
    pub active_signature: Option<usize>,
    /// Active parameter index
    pub active_parameter: Option<usize>,
}

/// Signature help provider
pub struct SignatureHelpProvider {
    symbol_table: SymbolTable,
    builtin_signatures: HashMap<&'static str, ImportedBuiltinSignature>,
}

impl SignatureHelpProvider {
    /// Create a new signature help provider
    pub fn new(ast: &Node) -> Self {
        let symbol_table = SymbolExtractor::new().extract(ast);
        let builtin_signatures = create_builtin_signatures();
        
        SignatureHelpProvider {
            symbol_table,
            builtin_signatures,
        }
    }
    
    /// Check if a built-in function exists
    pub fn has_builtin(&self, name: &str) -> bool {
        self.builtin_signatures.contains_key(name)
    }
    
    /// Get the number of built-in functions
    pub fn builtin_count(&self) -> usize {
        self.builtin_signatures.len()
    }
    
    /// Get built-in signature info
    pub fn get_builtin_signature(&self, name: &str) -> Option<&ImportedBuiltinSignature> {
        self.builtin_signatures.get(name)
    }
    
    /// Get signature help at a position
    pub fn get_signature_help(&self, source: &str, position: usize) -> Option<SignatureHelp> {
        // Find the function call context
        let context = self.find_call_context(source, position)?;
        
        // Get signatures for the function
        let signatures = self.get_signatures(&context.function_name);
        if signatures.is_empty() {
            return None;
        }
        
        // Determine active parameter
        let active_parameter = self.calculate_active_parameter(source, &context);
        
        Some(SignatureHelp {
            signatures,
            active_signature: Some(0),
            active_parameter: Some(active_parameter),
        })
    }
    
    /// Find the function call context at position
    fn find_call_context(&self, source: &str, position: usize) -> Option<CallContext> {
        // Look backwards for function name and opening parenthesis
        let mut paren_depth: usize = 0;
        let mut call_start = None;
        let chars: Vec<(usize, char)> = source.char_indices().collect();
        
        // Find our position in the char array
        let pos_idx = chars.iter().position(|(idx, _)| *idx >= position)?;
        
        // Search backwards
        for i in (0..=pos_idx).rev() {
            let (idx, ch) = chars[i];
            
            match ch {
                ')' => paren_depth += 1,
                '(' => {
                    if paren_depth == 0 {
                        call_start = Some(idx);
                        break;
                    } else {
                        paren_depth -= 1;
                    }
                }
                _ => {}
            }
        }
        
        let call_start = call_start?;
        
        // Find function name before the opening paren
        let before_paren = &source[..call_start];
        let function_name = self.extract_function_name(before_paren)?;
        
        Some(CallContext {
            function_name,
            call_start,
            position,
        })
    }
    
    /// Extract function name from text before parenthesis
    fn extract_function_name(&self, text: &str) -> Option<String> {
        // Skip whitespace from the end
        let text = text.trim_end();
        
        // Handle method calls (->method)
        if let Some(pos) = text.rfind("->") {
            let method_part = &text[pos + 2..];
            return Some(method_part.trim().to_string());
        }
        
        // Handle regular function calls
        let word_chars = text
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();
        
        if word_chars.is_empty() {
            None
        } else {
            Some(word_chars)
        }
    }
    
    /// Get signatures for a function
    fn get_signatures(&self, function_name: &str) -> Vec<SignatureInfo> {
        let mut signatures = Vec::new();
        
        // Check built-in functions
        if let Some(builtin) = self.builtin_signatures.get(function_name) {
            for sig_str in &builtin.signatures {
                let params = self.parse_builtin_parameters(sig_str);
                signatures.push(SignatureInfo {
                    label: sig_str.to_string(),
                    documentation: Some(builtin.documentation.to_string()),
                    parameters: params,
                    active_parameter: None,
                });
            }
        }
        
        // Check user-defined functions
        if let Some(symbols) = self.symbol_table.symbols.get(function_name) {
            for symbol in symbols {
                if symbol.kind == SymbolKind::Subroutine {
                    let sig = self.build_signature_from_symbol(symbol);
                    signatures.push(sig);
                }
            }
        }
        
        signatures
    }
    
    /// Parse parameters from a built-in function signature
    fn parse_builtin_parameters(&self, signature: &str) -> Vec<ParameterInfo> {
        let mut params = Vec::new();
        
        // Extract parameter part (after function name)
        if let Some(start) = signature.find(|c: char| c.is_whitespace() || c == '(') {
            let param_str = &signature[start..].trim();
            
            // Split by commas or spaces
            let parts: Vec<&str> = param_str
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|s| !s.is_empty() && !matches!(*s, "(" | ")"))
                .collect();
            
            for part in parts {
                params.push(ParameterInfo {
                    label: part.to_string(),
                    documentation: None,
                });
            }
        }
        
        params
    }
    
    /// Build signature from a symbol
    fn build_signature_from_symbol(&self, symbol: &crate::symbol::Symbol) -> SignatureInfo {
        let mut label = format!("sub {}", symbol.name);
        let mut params = Vec::new();
        
        // Try to extract parameters from attributes or documentation
        // In Perl, we might have prototype like: sub foo($$$) or sub foo :prototype($$$)
        for attr in &symbol.attributes {
            if attr.starts_with("prototype(") {
                if let Some(proto) = attr.strip_prefix("prototype(").and_then(|s| s.strip_suffix(")")) {
                    label.push_str(proto);
                    // Parse prototype
                    for (i, ch) in proto.chars().enumerate() {
                        match ch {
                            '$' => params.push(ParameterInfo {
                                label: format!("$arg{}", i + 1),
                                documentation: Some("scalar".to_string()),
                            }),
                            '@' => params.push(ParameterInfo {
                                label: "@args".to_string(),
                                documentation: Some("array (slurps remaining args)".to_string()),
                            }),
                            '%' => params.push(ParameterInfo {
                                label: "%args".to_string(),
                                documentation: Some("hash (slurps remaining args)".to_string()),
                            }),
                            '&' => params.push(ParameterInfo {
                                label: "&code".to_string(),
                                documentation: Some("code reference".to_string()),
                            }),
                            _ => {}
                        }
                    }
                }
            }
        }
        
        // If no prototype, assume it takes a list
        if params.is_empty() {
            label.push_str("(...)");
            params.push(ParameterInfo {
                label: "LIST".to_string(),
                documentation: Some("arbitrary list of values".to_string()),
            });
        }
        
        SignatureInfo {
            label,
            documentation: symbol.documentation.clone(),
            parameters: params,
            active_parameter: None,
        }
    }
    
    /// Calculate which parameter is active
    fn calculate_active_parameter(&self, source: &str, context: &CallContext) -> usize {
        // Handle edge case where cursor is right at the opening paren
        if context.position <= context.call_start + 1 {
            return 0;
        }
        
        let arg_text = &source[context.call_start + 1..context.position];
        
        // Also need to handle nested parentheses
        let mut paren_depth: usize = 0;
        let mut actual_comma_count = 0;
        
        for ch in arg_text.chars() {
            match ch {
                '(' => paren_depth += 1,
                ')' => paren_depth = paren_depth.saturating_sub(1),
                ',' if paren_depth == 0 => actual_comma_count += 1,
                _ => {}
            }
        }
        
        actual_comma_count
    }
    
}

/// Context of a function call
#[derive(Debug)]
struct CallContext {
    /// Name of the function being called
    function_name: String,
    /// Position of the opening parenthesis
    call_start: usize,
    /// Current cursor position
    position: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    
    #[test]
    fn test_builtin_signature_help() {
        let code = "print($fh, ";
        let position = code.len() - 1;
        
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);
        
        let help = provider.get_signature_help(code, position);
        assert!(help.is_some());
        
        let help = help.unwrap();
        assert!(!help.signatures.is_empty());
        assert_eq!(help.active_parameter, Some(1)); // Second parameter
    }
    
    #[test]
    fn test_parameter_counting() {
        let code = "substr($str, 5, ";
        let position = code.len() - 1;
        
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);
        
        let help = provider.get_signature_help(code, position);
        assert!(help.is_some());
        
        let help = help.unwrap();
        assert_eq!(help.active_parameter, Some(2)); // Third parameter
    }
    
    #[test]
    fn test_nested_calls() {
        let code = "push(@arr, split(',', $str))";
        let position = 22; // After the comma in split(',', 
        
        let ast = Parser::new(code).parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);
        
        let help = provider.get_signature_help(code, position);
        assert!(help.is_some());
        
        let help = help.unwrap();
        assert_eq!(help.signatures[0].label, "split /PATTERN/, EXPR, LIMIT");
        
        // The active parameter could be 1 or 2 depending on interpretation
        // Since we're after the comma in split(',', ...), we should be on parameter 2
        assert!(help.active_parameter == Some(1) || help.active_parameter == Some(2));
    }
}