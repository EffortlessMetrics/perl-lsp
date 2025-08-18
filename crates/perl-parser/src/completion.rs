//! Code completion provider for Perl
//!
//! This module provides intelligent code completion suggestions based on
//! context, including variables, functions, keywords, and more.

use crate::SourceLocation;
use crate::ast::Node;
use crate::symbol::{SymbolExtractor, SymbolKind, SymbolTable};
use std::collections::HashSet;

/// Type of completion item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionItemKind {
    /// Variable (scalar, array, hash)
    Variable,
    /// Function or method
    Function,
    /// Perl keyword
    Keyword,
    /// Package or module
    Module,
    /// File path
    File,
    /// Snippet with placeholders
    Snippet,
    /// Constant value
    Constant,
    /// Property or hash key
    Property,
}

/// A single completion suggestion
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// The text to insert
    pub label: String,
    /// Kind of completion
    pub kind: CompletionItemKind,
    /// Optional detail text
    pub detail: Option<String>,
    /// Optional documentation
    pub documentation: Option<String>,
    /// Text to insert (if different from label)
    pub insert_text: Option<String>,
    /// Sort priority (lower is better)
    pub sort_text: Option<String>,
    /// Filter text for matching
    pub filter_text: Option<String>,
    /// Additional text edits to apply
    pub additional_edits: Vec<(SourceLocation, String)>,
}

/// Context for completion
#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// The position where completion was triggered
    pub position: usize,
    /// The character that triggered completion (if any)
    pub trigger_character: Option<char>,
    /// Whether we're in a string literal
    pub in_string: bool,
    /// Whether we're in a regex
    pub in_regex: bool,
    /// Whether we're in a comment
    pub in_comment: bool,
    /// Current package context
    pub current_package: String,
    /// Prefix text before cursor
    pub prefix: String,
}

/// Completion provider
pub struct CompletionProvider {
    symbol_table: SymbolTable,
    keywords: HashSet<&'static str>,
    builtins: HashSet<&'static str>,
}

impl CompletionProvider {
    /// Create a new completion provider from parsed AST
    pub fn new(ast: &Node) -> Self {
        let symbol_table = SymbolExtractor::new().extract(ast);

        let keywords = [
            "my",
            "our",
            "local",
            "state",
            "sub",
            "package",
            "use",
            "require",
            "if",
            "elsif",
            "else",
            "unless",
            "while",
            "until",
            "for",
            "foreach",
            "do",
            "eval",
            "goto",
            "last",
            "next",
            "redo",
            "return",
            "die",
            "warn",
            "exit",
            "and",
            "or",
            "not",
            "xor",
            "eq",
            "ne",
            "lt",
            "le",
            "gt",
            "ge",
            "cmp",
            "defined",
            "undef",
            "ref",
            "blessed",
            "scalar",
            "wantarray",
            "__PACKAGE__",
            "__FILE__",
            "__LINE__",
            "BEGIN",
            "END",
            "CHECK",
            "INIT",
            "UNITCHECK",
            "DESTROY",
            "AUTOLOAD",
        ]
        .into_iter()
        .collect();

        let builtins = [
            // I/O
            "print",
            "printf",
            "say",
            "sprintf",
            "open",
            "close",
            "read",
            "write",
            "seek",
            "tell",
            "binmode",
            "eof",
            "fileno",
            "flock",
            // String
            "chomp",
            "chop",
            "chr",
            "ord",
            "lc",
            "uc",
            "lcfirst",
            "ucfirst",
            "length",
            "substr",
            "index",
            "rindex",
            "split",
            "join",
            "reverse",
            "sprintf",
            "quotemeta",
            // Array
            "push",
            "pop",
            "shift",
            "unshift",
            "splice",
            "grep",
            "map",
            "sort",
            "reverse",
            // Hash
            "keys",
            "values",
            "each",
            "exists",
            "delete",
            // Math
            "abs",
            "atan2",
            "cos",
            "sin",
            "exp",
            "log",
            "sqrt",
            "int",
            "rand",
            "srand",
            // File tests
            "-r",
            "-w",
            "-x",
            "-o",
            "-R",
            "-W",
            "-X",
            "-O",
            "-e",
            "-z",
            "-s",
            "-f",
            "-d",
            "-l",
            "-p",
            "-S",
            "-b",
            "-c",
            "-t",
            "-u",
            "-g",
            "-k",
            "-T",
            "-B",
            "-M",
            "-A",
            "-C",
            // System
            "system",
            "exec",
            "fork",
            "wait",
            "waitpid",
            "kill",
            "sleep",
            "alarm",
            "getpid",
            "getppid",
            // Time
            "time",
            "localtime",
            "gmtime",
            // Misc
            "caller",
            "die",
            "warn",
            "eval",
            "exit",
            "require",
            "use",
            "no",
            "import",
            "unimport",
            "bless",
            "ref",
            "tied",
            "untie",
            "pack",
            "unpack",
            "vec",
            "study",
            "pos",
            "qr",
        ]
        .into_iter()
        .collect();

        CompletionProvider { symbol_table, keywords, builtins }
    }

    /// Get completions at a given position
    pub fn get_completions(&self, source: &str, position: usize) -> Vec<CompletionItem> {
        let context = self.analyze_context(source, position);

        if context.in_comment {
            return vec![];
        }

        let mut completions = Vec::new();

        // Determine what kind of completions to provide based on context
        if context.prefix.starts_with('$') {
            // Scalar variable completion
            self.add_variable_completions(&mut completions, &context, SymbolKind::ScalarVariable);
            self.add_special_variables(&mut completions, &context, "$");
        } else if context.prefix.starts_with('@') {
            // Array variable completion
            self.add_variable_completions(&mut completions, &context, SymbolKind::ArrayVariable);
            self.add_special_variables(&mut completions, &context, "@");
        } else if context.prefix.starts_with('%') {
            // Hash variable completion
            self.add_variable_completions(&mut completions, &context, SymbolKind::HashVariable);
            self.add_special_variables(&mut completions, &context, "%");
        } else if context.prefix.starts_with('&') {
            // Subroutine completion
            self.add_function_completions(&mut completions, &context);
        } else if context.trigger_character == Some(':') && context.prefix.ends_with("::") {
            // Package member completion
            self.add_package_completions(&mut completions, &context);
        } else if context.trigger_character == Some('>') && context.prefix.ends_with("->") {
            // Method completion
            self.add_method_completions(&mut completions, &context, source);
        } else if context.in_string {
            // String interpolation or file path
            if context.prefix.contains('/') {
                self.add_file_completions(&mut completions, &context);
            }
        } else {
            // General completion: keywords, functions, variables
            if context.prefix.is_empty() || self.could_be_keyword(&context.prefix) {
                self.add_keyword_completions(&mut completions, &context);
            }

            if context.prefix.is_empty() || self.could_be_function(&context.prefix) {
                self.add_builtin_completions(&mut completions, &context);
                self.add_function_completions(&mut completions, &context);
            }

            // Also suggest variables without sigils in some contexts
            self.add_all_variables(&mut completions, &context);
        }

        // Sort completions by relevance
        completions.sort_by(|a, b| {
            a.sort_text.as_ref().unwrap_or(&a.label).cmp(b.sort_text.as_ref().unwrap_or(&b.label))
        });

        completions
    }

    /// Analyze the context at the cursor position
    fn analyze_context(&self, source: &str, position: usize) -> CompletionContext {
        // Find the prefix (text before cursor on the same line)
        let line_start = source[..position].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let _prefix = source[line_start..position].to_string();

        // Find the word being typed
        // Special handling for method calls: include the -> and the receiver
        let word_prefix = if position >= 2 && &source[position.saturating_sub(2)..position] == "->"
        {
            // We're right after ->, find the receiver variable
            let receiver_start = source[..position.saturating_sub(2)]
                .rfind(|c: char| {
                    !c.is_alphanumeric() && c != '_' && c != '$' && c != '@' && c != '%'
                })
                .map(|p| p + 1)
                .unwrap_or(0);
            source[receiver_start..position].to_string()
        } else {
            let word_start = source[..position]
                .rfind(|c: char| {
                    !c.is_alphanumeric()
                        && c != '_'
                        && c != ':'
                        && c != '$'
                        && c != '@'
                        && c != '%'
                        && c != '&'
                })
                .map(|p| p + 1)
                .unwrap_or(0);
            source[word_start..position].to_string()
        };

        // Detect trigger character
        let trigger_character = if position > 0 { source.chars().nth(position - 1) } else { None };

        // Simple heuristics for context detection
        let in_string = self.is_in_string(source, position);
        let in_regex = self.is_in_regex(source, position);
        let in_comment = self.is_in_comment(source, position);

        CompletionContext {
            position,
            trigger_character,
            in_string,
            in_regex,
            in_comment,
            current_package: "main".to_string(), // TODO: Detect actual package
            prefix: word_prefix,
        }
    }

    /// Add variable completions
    #[allow(clippy::ptr_arg)] // needs Vec for push operations
    fn add_variable_completions(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
        kind: SymbolKind,
    ) {
        let sigil = kind.sigil().unwrap_or("");
        let prefix_without_sigil = context.prefix.trim_start_matches(sigil);

        for (name, symbols) in &self.symbol_table.symbols {
            for symbol in symbols {
                if symbol.kind == kind && name.starts_with(prefix_without_sigil) {
                    let insert_text = format!("{}{}", sigil, name);

                    completions.push(CompletionItem {
                        label: insert_text.clone(),
                        kind: CompletionItemKind::Variable,
                        detail: Some(
                            format!(
                                "{} {}{}",
                                symbol.declaration.as_deref().unwrap_or(""),
                                sigil,
                                name
                            )
                            .trim()
                            .to_string(),
                        ),
                        documentation: symbol.documentation.clone(),
                        insert_text: Some(insert_text),
                        sort_text: Some(format!("1_{}", name)), // Variables have high priority
                        filter_text: Some(name.clone()),
                        additional_edits: vec![],
                    });
                }
            }
        }
    }

    /// Add special Perl variables
    #[allow(clippy::ptr_arg)] // needs Vec for push operations
    fn add_special_variables(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
        sigil: &str,
    ) {
        let special_vars = match sigil {
            "$" => vec![
                ("$_", "Default input and pattern-search space"),
                ("$.", "Current line number"),
                ("$,", "Output field separator"),
                ("$/", "Input record separator"),
                ("$\\", "Output record separator"),
                ("$!", "Current errno"),
                ("$@", "Last eval error"),
                ("$$", "Process ID"),
                ("$0", "Program name"),
                ("$1", "First capture group"),
                ("$&", "Last match"),
                ("$`", "Prematch"),
                ("$'", "Postmatch"),
                ("$+", "Last capture group"),
                ("$^O", "Operating system name"),
                ("$^V", "Perl version"),
            ],
            "@" => vec![
                ("@_", "Subroutine arguments"),
                ("@ARGV", "Command line arguments"),
                ("@INC", "Module search paths"),
                ("@ISA", "Base classes"),
                ("@EXPORT", "Exported symbols"),
            ],
            "%" => vec![
                ("%ENV", "Environment variables"),
                ("%INC", "Loaded modules"),
                ("%SIG", "Signal handlers"),
            ],
            _ => vec![],
        };

        for (var, description) in special_vars {
            if var.starts_with(&context.prefix) {
                completions.push(CompletionItem {
                    label: var.to_string(),
                    kind: CompletionItemKind::Variable,
                    detail: Some("special variable".to_string()),
                    documentation: Some(description.to_string()),
                    insert_text: Some(var.to_string()),
                    sort_text: Some(format!("0_{}", var)), // Special vars have highest priority
                    filter_text: Some(var.to_string()),
                    additional_edits: vec![],
                });
            }
        }
    }

    /// Add function completions
    #[allow(clippy::ptr_arg)] // needs Vec for push operations
    fn add_function_completions(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
    ) {
        let prefix_without_amp = context.prefix.trim_start_matches('&');

        for (name, symbols) in &self.symbol_table.symbols {
            for symbol in symbols {
                if symbol.kind == SymbolKind::Subroutine && name.starts_with(prefix_without_amp) {
                    completions.push(CompletionItem {
                        label: name.clone(),
                        kind: CompletionItemKind::Function,
                        detail: Some("sub".to_string()),
                        documentation: symbol.documentation.clone(),
                        insert_text: Some(format!("{}()", name)),
                        sort_text: Some(format!("2_{}", name)),
                        filter_text: Some(name.clone()),
                        additional_edits: vec![],
                    });
                }
            }
        }
    }

    /// Add built-in function completions
    #[allow(clippy::ptr_arg)] // needs Vec for push operations
    fn add_builtin_completions(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
    ) {
        for builtin in &self.builtins {
            if builtin.starts_with(&context.prefix) {
                let (insert_text, detail) = match *builtin {
                    "print" => ("print ", "print FILEHANDLE LIST"),
                    "open" => ("open(my $fh, '<', )", "open FILEHANDLE, MODE, FILENAME"),
                    "push" => ("push(@, )", "push ARRAY, LIST"),
                    "map" => ("map { } ", "map BLOCK LIST"),
                    "grep" => ("grep { } ", "grep BLOCK LIST"),
                    "sort" => ("sort { } ", "sort BLOCK LIST"),
                    _ => (*builtin, "built-in function"),
                };

                completions.push(CompletionItem {
                    label: builtin.to_string(),
                    kind: CompletionItemKind::Function,
                    detail: Some(detail.to_string()),
                    documentation: None,
                    insert_text: Some(insert_text.to_string()),
                    sort_text: Some(format!("3_{}", builtin)),
                    filter_text: Some(builtin.to_string()),
                    additional_edits: vec![],
                });
            }
        }
    }

    /// Add keyword completions
    #[allow(clippy::ptr_arg)] // needs Vec for push operations
    fn add_keyword_completions(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
    ) {
        for keyword in &self.keywords {
            if keyword.starts_with(&context.prefix) {
                let (insert_text, snippet) = match *keyword {
                    "sub" => ("sub ${1:name} {\n    $0\n}", true),
                    "if" => ("if ($1) {\n    $0\n}", true),
                    "elsif" => ("elsif ($1) {\n    $0\n}", true),
                    "else" => ("else {\n    $0\n}", true),
                    "unless" => ("unless ($1) {\n    $0\n}", true),
                    "while" => ("while ($1) {\n    $0\n}", true),
                    "for" => ("for (my $i = 0; $i < $1; $i++) {\n    $0\n}", true),
                    "foreach" => ("foreach my $${1:item} (@${2:array}) {\n    $0\n}", true),
                    "package" => ("package ${1:Name};\n\n$0", true),
                    "use" => ("use ${1:Module};\n$0", true),
                    _ => (*keyword, false),
                };

                completions.push(CompletionItem {
                    label: keyword.to_string(),
                    kind: if snippet {
                        CompletionItemKind::Snippet
                    } else {
                        CompletionItemKind::Keyword
                    },
                    detail: Some("keyword".to_string()),
                    documentation: None,
                    insert_text: Some(insert_text.to_string()),
                    sort_text: Some(format!("4_{}", keyword)),
                    filter_text: Some(keyword.to_string()),
                    additional_edits: vec![],
                });
            }
        }
    }

    /// Add package member completions
    #[allow(clippy::ptr_arg)] // might need Vec in future for push operations
    fn add_package_completions(
        &self,
        _completions: &mut Vec<CompletionItem>,
        _context: &CompletionContext,
    ) {
        // TODO: Implement package member completion
        // This would require analyzing package contents
    }

    /// Add method completions
    #[allow(clippy::ptr_arg)] // needs Vec for push operations
    fn add_method_completions(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
        source: &str,
    ) {
        // Try to infer the receiver type from context
        let receiver_type = self.infer_receiver_type(context, source);

        // DBI database handle methods
        const DBI_DB_METHODS: &[(&str, &str)] = &[
            ("do", "Execute a single SQL statement"),
            ("prepare", "Prepare a SQL statement"),
            ("prepare_cached", "Prepare and cache a SQL statement"),
            ("selectrow_array", "Execute and fetch a single row as array"),
            ("selectrow_arrayref", "Execute and fetch a single row as arrayref"),
            ("selectrow_hashref", "Execute and fetch a single row as hashref"),
            ("selectall_arrayref", "Execute and fetch all rows as arrayref"),
            ("selectall_hashref", "Execute and fetch all rows as hashref"),
            ("begin_work", "Begin a database transaction"),
            ("commit", "Commit the current transaction"),
            ("rollback", "Rollback the current transaction"),
            ("disconnect", "Disconnect from the database"),
            ("last_insert_id", "Get the last inserted row ID"),
            ("quote", "Quote a string for SQL"),
            ("quote_identifier", "Quote an identifier for SQL"),
            ("ping", "Check if database connection is alive"),
        ];

        // DBI statement handle methods
        const DBI_ST_METHODS: &[(&str, &str)] = &[
            ("bind_param", "Bind a parameter to the statement"),
            ("bind_param_inout", "Bind an in/out parameter"),
            ("execute", "Execute the prepared statement"),
            ("fetch", "Fetch the next row as arrayref"),
            ("fetchrow_array", "Fetch the next row as array"),
            ("fetchrow_arrayref", "Fetch the next row as arrayref"),
            ("fetchrow_hashref", "Fetch the next row as hashref"),
            ("fetchall_arrayref", "Fetch all remaining rows as arrayref"),
            ("fetchall_hashref", "Fetch all remaining rows as hashref of hashrefs"),
            ("finish", "Finish the statement handle"),
            ("rows", "Get the number of rows affected"),
        ];

        // Choose methods based on inferred type
        let methods: Vec<(&str, &str)> = match receiver_type.as_deref() {
            Some("DBI::db") => DBI_DB_METHODS.to_vec(),
            Some("DBI::st") => DBI_ST_METHODS.to_vec(),
            _ => {
                // Default common object methods
                vec![
                    ("new", "Constructor"),
                    ("isa", "Check if object is of given class"),
                    ("can", "Check if object can call method"),
                    ("DOES", "Check if object does role"),
                    ("VERSION", "Get version"),
                ]
            }
        };

        for (method, desc) in methods {
            completions.push(CompletionItem {
                label: method.to_string(),
                kind: CompletionItemKind::Function,
                detail: Some("method".to_string()),
                documentation: Some(desc.to_string()),
                insert_text: Some(format!("{}()", method)),
                sort_text: Some(format!("2_{}", method)),
                filter_text: Some(method.to_string()),
                additional_edits: vec![],
            });
        }

        // If we have a DBI type, also add common methods at lower priority
        if receiver_type.as_deref() == Some("DBI::db")
            || receiver_type.as_deref() == Some("DBI::st")
        {
            for (method, desc) in [
                ("isa", "Check if object is of given class"),
                ("can", "Check if object can call method"),
            ] {
                completions.push(CompletionItem {
                    label: method.to_string(),
                    kind: CompletionItemKind::Function,
                    detail: Some("method".to_string()),
                    documentation: Some(desc.to_string()),
                    insert_text: Some(format!("{}()", method)),
                    sort_text: Some(format!("9_{}", method)), // Lower priority
                    filter_text: Some(method.to_string()),
                    additional_edits: vec![],
                });
            }
        }
    }

    /// Infer receiver type from context (for DBI method completion)
    fn infer_receiver_type(&self, context: &CompletionContext, source: &str) -> Option<String> {
        // Look backwards from the position to find the receiver
        let prefix = context.prefix.trim_end_matches("->");

        // Simple heuristics for DBI types based on variable name
        if prefix.ends_with("$dbh") {
            return Some("DBI::db".to_string());
        }
        if prefix.ends_with("$sth") {
            return Some("DBI::st".to_string());
        }

        // Look at the broader context - check if variable was assigned from DBI->connect
        if let Some(var_pos) = source.rfind(prefix) {
            // Look backwards for assignment
            let before_var = &source[..var_pos];
            if let Some(assign_pos) = before_var.rfind('=') {
                let assignment = &source[assign_pos..var_pos + prefix.len()];

                // Check if this looks like DBI->connect result
                if assignment.contains("DBI") && assignment.contains("connect") {
                    return Some("DBI::db".to_string());
                }

                // Check if this looks like prepare/prepare_cached result
                if assignment.contains("prepare") {
                    return Some("DBI::st".to_string());
                }
            }
        }

        None
    }

    /// Add file path completions
    #[allow(clippy::ptr_arg)] // might need Vec in future for push operations
    fn add_file_completions(
        &self,
        _completions: &mut Vec<CompletionItem>,
        _context: &CompletionContext,
    ) {
        // TODO: Implement file path completion
        // This would require filesystem access
    }

    /// Add all variables without sigils (for interpolation contexts)
    #[allow(clippy::ptr_arg)] // needs Vec for push operations
    fn add_all_variables(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
    ) {
        // Only add if the prefix doesn't already have a sigil
        if !context.prefix.starts_with(['$', '@', '%', '&']) {
            for (name, symbols) in &self.symbol_table.symbols {
                for symbol in symbols {
                    if matches!(
                        symbol.kind,
                        SymbolKind::ScalarVariable
                            | SymbolKind::ArrayVariable
                            | SymbolKind::HashVariable
                    ) && name.starts_with(&context.prefix)
                    {
                        let sigil = symbol.kind.sigil().unwrap_or("");
                        completions.push(CompletionItem {
                            label: format!("{}{}", sigil, name),
                            kind: CompletionItemKind::Variable,
                            detail: Some(format!(
                                "{} variable",
                                symbol.declaration.as_deref().unwrap_or("")
                            )),
                            documentation: symbol.documentation.clone(),
                            insert_text: Some(format!("{}{}", sigil, name)),
                            sort_text: Some(format!("5_{}", name)),
                            filter_text: Some(name.clone()),
                            additional_edits: vec![],
                        });
                    }
                }
            }
        }
    }

    /// Check if prefix could be a keyword
    fn could_be_keyword(&self, prefix: &str) -> bool {
        self.keywords.iter().any(|k| k.starts_with(prefix))
    }

    /// Check if prefix could be a function
    fn could_be_function(&self, prefix: &str) -> bool {
        // Check builtins
        if self.builtins.iter().any(|b| b.starts_with(prefix)) {
            return true;
        }

        // Check user-defined functions
        for (name, symbols) in &self.symbol_table.symbols {
            for symbol in symbols {
                if symbol.kind == SymbolKind::Subroutine && name.starts_with(prefix) {
                    return true;
                }
            }
        }

        false
    }

    /// Simple heuristic to check if position is in a string
    fn is_in_string(&self, source: &str, position: usize) -> bool {
        let before = &source[..position];
        let single_quotes = before.matches('\'').count();
        let double_quotes = before.matches('"').count();

        // Very simple: odd number of quotes means we're inside
        single_quotes % 2 == 1 || double_quotes % 2 == 1
    }

    /// Simple heuristic to check if position is in a regex
    fn is_in_regex(&self, source: &str, position: usize) -> bool {
        // Look for regex patterns before position
        let before = &source[..position];
        if let Some(last_slash) = before.rfind('/') {
            if last_slash > 0 {
                let prev_char = before.chars().nth(last_slash - 1);
                matches!(prev_char, Some('~' | '!'))
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Simple heuristic to check if position is in a comment
    fn is_in_comment(&self, source: &str, position: usize) -> bool {
        let line_start = source[..position].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let line = &source[line_start..position];
        line.contains('#')
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_variable_completion() {
        let code = r#"
my $count = 42;
my $counter = 0;
my @items = ();

$c
"#;

        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap();

        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len() - 1);

        assert!(completions.iter().any(|c| c.label == "$count"));
        assert!(completions.iter().any(|c| c.label == "$counter"));
    }

    #[test]
    fn test_function_completion() {
        let code = r#"
sub process_data {
    # ...
}

sub process_items {
    # ...
}

proc
"#;

        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap();

        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len() - 1);

        assert!(completions.iter().any(|c| c.label == "process_data"));
        assert!(completions.iter().any(|c| c.label == "process_items"));
    }

    #[test]
    fn test_builtin_completion() {
        let code = "pr";

        let mut parser = Parser::new(""); // Empty AST
        let ast = parser.parse().unwrap();

        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        assert!(completions.iter().any(|c| c.label == "print"));
        assert!(completions.iter().any(|c| c.label == "printf"));
    }
}
