//! Comprehensive IDE features demonstration
//!
//! This example showcases all IDE features: diagnostics, code actions,
//! completion, signature help, rename, and more.

use perl_parser::{
    Parser,
    DiagnosticsProvider, DiagnosticSeverity,
    CodeActionsProvider, CodeActionKind,
    CompletionProvider,
    signature_help::SignatureHelpProvider,
    rename::{RenameProvider, RenameOptions, apply_rename_edits},
    LspServer,
    position::Position,
};

fn main() {
    println!("=== Comprehensive Perl IDE Demo ===\n");
    
    // Sample Perl code with various issues
    let source = r#"
# Missing 'use strict' and 'use warnings'

print $undefined_var;  # undefined variable

my $unused = 42;  # unused variable

if ($x = 5) {  # assignment in condition
    print "x is 5\n";
}

# Deprecated syntax
if (defined @array) {
    print "Array is defined\n";
}

# Numeric comparison with potential undef
if ($maybe_undef == 0) {
    print "Zero\n";
}

# Function calls for signature help
print("Hello, ", "World", "\n");
split(/,/, $string, 3);

# Code for completion demo
my $comp

# Code for rename demo
my $old_name = 10;
print $old_name * 2;
my $result = $old_name + 5;

# Extract variable demo
my $total = ($price * $quantity) + ($tax * $price * $quantity);

# Extract function demo
print "Starting process\n";
validate_input();
process_data();
save_results();
print "Process complete\n";
"#;

    // Parse the code
    let mut parser = Parser::new(source);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(e) => {
            println!("Parse error: {:?}", e);
            return;
        }
    };
    let parse_errors: Vec<ParseError> = vec![];
    
    println!("✓ Successfully parsed the code");
    println!("  Parse errors: {}\n", parse_errors.len());
    
    // 1. Diagnostics
    println!("1. DIAGNOSTICS");
    println!("==============");
    
    let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &parse_errors);
    
    for diag in &diagnostics {
        let severity = match diag.severity {
            DiagnosticSeverity::Error => "ERROR",
            DiagnosticSeverity::Warning => "WARNING",
            DiagnosticSeverity::Information => "INFO",
            DiagnosticSeverity::Hint => "HINT",
        };
        
        println!("[{}] {}", severity, diag.message);
        if let Some(code) = &diag.code {
            println!("  Code: {}", code);
        }
        println!("  Range: {:?}", diag.range);
        
        for related in &diag.related_information {
            println!("  Related: {}", related.message);
        }
        println!();
    }
    
    // 2. Code Actions
    println!("2. CODE ACTIONS");
    println!("===============");
    
    let code_actions_provider = CodeActionsProvider::new(source.to_string());
    let actions = code_actions_provider.get_code_actions(&ast, (0, source.len()), &diagnostics);
    
    println!("Available code actions: {}", actions.len());
    for (i, action) in actions.iter().enumerate() {
        let kind = match action.kind {
            CodeActionKind::QuickFix => "QuickFix",
            CodeActionKind::Refactor => "Refactor",
            CodeActionKind::RefactorExtract => "Extract",
            _ => "Other",
        };
        
        println!("{}. [{}] {}", i + 1, kind, action.title);
        if action.is_preferred {
            println!("   (Preferred)");
        }
        println!("   Fixes: {:?}", action.diagnostics);
        println!("   Edits: {} changes", action.edit.changes.len());
    }
    println!();
    
    // 3. Completion
    println!("3. CODE COMPLETION");
    println!("==================");
    
    let completion_provider = CompletionProvider::new(&ast);
    
    // Find position after "$comp"
    let comp_pos = source.find("my $comp").unwrap() + 8;
    let completions = completion_provider.get_completions(source, comp_pos);
    
    println!("Completions at position {} (after '$comp'):", comp_pos);
    for comp in completions.iter().take(10) {
        println!("  - {} ({:?})", comp.label, comp.kind);
        if let Some(detail) = &comp.detail {
            println!("    {}", detail);
        }
    }
    println!("  ... and {} more", completions.len().saturating_sub(10));
    println!();
    
    // 4. Signature Help
    println!("4. SIGNATURE HELP");
    println!("=================");
    
    let sig_help_provider = SignatureHelpProvider::new(&ast);
    
    // Find position inside print function
    let print_pos = source.find("print(\"Hello").unwrap() + 13;
    if let Some(help) = sig_help_provider.get_signature_help(source, print_pos) {
        println!("Signature help for 'print' at position {}:", print_pos);
        for sig in &help.signatures {
            println!("  {}", sig.label);
            if let Some(doc) = &sig.documentation {
                println!("  Doc: {}", doc);
            }
        }
        if let Some(active) = help.active_parameter {
            println!("  Active parameter: {}", active);
        }
    }
    println!();
    
    // 5. Rename
    println!("5. RENAME REFACTORING");
    println!("=====================");
    
    let rename_provider = RenameProvider::new(&ast, source.to_string());
    
    // Find position of $old_name
    let old_name_pos = source.find("my $old_name").unwrap() + 3;
    
    if let Some((range, name)) = rename_provider.prepare_rename(old_name_pos) {
        println!("Can rename '{}' at range {:?}", name, range);
        
        let result = rename_provider.rename(old_name_pos, "new_name", &RenameOptions::default());
        if result.is_valid {
            println!("Rename successful! {} edits", result.edits.len());
            for edit in &result.edits {
                println!("  Replace at {:?} with '{}'", edit.location, edit.new_text);
            }
            
            // Apply the rename
            let new_source = apply_rename_edits(source, &result.edits);
            println!("\nCode after rename:");
            println!("---");
            for line in new_source.lines().skip(31).take(3) {
                println!("{}", line);
            }
            println!("---");
        }
    }
    println!();
    
    // 6. Language Server Integration
    println!("6. LANGUAGE SERVER");
    println!("==================");
    
    let mut ls = LanguageServer::new();
    ls.open_document("file:///test.pl".to_string(), 1, source.to_string());
    
    // Get document symbols
    let symbols = ls.document_symbols("file:///test.pl");
    println!("Document symbols: {}", symbols.len());
    for sym in symbols.iter().take(5) {
        println!("  - {} ({:?})", sym.name, sym.kind);
    }
    
    // Find definition
    let old_name_line = source[..old_name_pos].lines().count() - 1;
    let old_name_col = source[..old_name_pos].lines().last().map(|l| l.len()).unwrap_or(0);
    let position = Position { line: old_name_line, character: old_name_col };
    if let Some(def_loc) = ls.goto_definition("file:///test.pl", position) {
        println!("\nDefinition of $old_name at: {:?}", def_loc);
    }
    
    // Find references
    let refs = ls.find_references("file:///test.pl", position, true);
    println!("\nReferences to $old_name: {}", refs.len());
    for r in &refs {
        println!("  - {:?}", r);
    }
    
    println!("\n✓ All IDE features demonstrated successfully!");
}