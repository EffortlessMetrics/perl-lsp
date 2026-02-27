//! Mock implementations for test infrastructure validation
//!
//! Provides temporary mock responses and test doubles for validating
//! test scaffolding and infrastructure components without requiring
//! full LSP server functionality.
//!
//! Features:
//! - Mock LSP server responses for scaffolding validation
//! - Temporary Perl file analysis mocks
//! - Test harness validation helpers
//! - Performance simulation for timing tests
//! - Error injection for error handling validation

use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[cfg(test)]
pub struct MockLspServer {
    pub responses: HashMap<String, Value>,
    pub request_count: usize,
    pub start_time: Instant,
    pub simulate_latency: bool,
    pub error_rate: f32,
}

#[cfg(test)]
impl MockLspServer {
    pub fn new() -> Self {
        let mut responses = HashMap::new();

        // Initialize with basic mock responses
        responses.insert("initialize".to_string(), json!({
            "jsonrpc": "2.0",
            "result": {
                "capabilities": {
                    "textDocumentSync": 2,
                    "hoverProvider": true,
                    "completionProvider": {
                        "triggerCharacters": ["$", "@", "%"]
                    },
                    "definitionProvider": true,
                    "referencesProvider": true,
                    "documentSymbolProvider": true,
                    "executeCommandProvider": {
                        "commands": ["perl.runCritic"]
                    }
                },
                "serverInfo": {
                    "name": "perl-lsp-mock",
                    "version": "0.8.9-test"
                }
            }
        }));

        responses.insert("textDocument/documentSymbol".to_string(), json!({
            "jsonrpc": "2.0",
            "result": [
                {
                    "name": "mock_function",
                    "kind": 12,
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 5, "character": 1 }
                    }
                }
            ]
        }));

        responses.insert("textDocument/hover".to_string(), json!({
            "jsonrpc": "2.0",
            "result": {
                "contents": "Mock hover information"
            }
        }));

        responses.insert("textDocument/completion".to_string(), json!({
            "jsonrpc": "2.0",
            "result": {
                "items": [
                    {
                        "label": "$mock_var",
                        "kind": 6,
                        "insertText": "$mock_var"
                    }
                ]
            }
        }));

        responses.insert("workspace/executeCommand".to_string(), json!({
            "jsonrpc": "2.0",
            "result": {
                "status": "success",
                "analyzerUsed": "mock_analyzer",
                "violations": []
            }
        }));

        Self {
            responses,
            request_count: 0,
            start_time: Instant::now(),
            simulate_latency: false,
            error_rate: 0.0,
        }
    }

    pub fn with_latency(mut self, enable: bool) -> Self {
        self.simulate_latency = enable;
        self
    }

    pub fn with_error_rate(mut self, rate: f32) -> Self {
        self.error_rate = rate.clamp(0.0, 1.0);
        self
    }

    pub fn handle_request(&mut self, method: &str, id: Value) -> Value {
        self.request_count += 1;

        // Simulate latency if enabled
        if self.simulate_latency {
            std::thread::sleep(Duration::from_millis(50));
        }

        // Inject errors based on error rate
        if self.error_rate > 0.0 && rand::random::<f32>() < self.error_rate {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603,
                    "message": "Mock internal error",
                    "data": {
                        "request_count": self.request_count,
                        "method": method
                    }
                }
            });
        }

        // Return mock response
        if let Some(response) = self.responses.get(method) {
            let mut response = response.clone();
            if let Some(obj) = response.as_object_mut() {
                obj.insert("id".to_string(), id);
            }
            response
        } else {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                }
            })
        }
    }

    pub fn get_stats(&self) -> MockServerStats {
        MockServerStats {
            request_count: self.request_count,
            uptime_ms: self.start_time.elapsed().as_millis() as u64,
            error_rate: self.error_rate,
            latency_simulation: self.simulate_latency,
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct MockServerStats {
    pub request_count: usize,
    pub uptime_ms: u64,
    pub error_rate: f32,
    pub latency_simulation: bool,
}

/// Mock Perl file analyzer for testing without real parsing
#[cfg(test)]
pub struct MockPerlAnalyzer {
    pub file_cache: HashMap<String, MockFileAnalysis>,
    pub analysis_time_ms: u64,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct MockFileAnalysis {
    pub uri: String,
    pub syntax_valid: bool,
    pub symbol_count: usize,
    pub violation_count: usize,
    pub analysis_time_ms: u64,
    pub ast_node_count: usize,
}

#[cfg(test)]
impl MockPerlAnalyzer {
    pub fn new() -> Self {
        Self {
            file_cache: HashMap::new(),
            analysis_time_ms: 100,
        }
    }

    pub fn analyze_file(&mut self, uri: &str, _content: &str) -> MockFileAnalysis {
        // Simulate analysis time
        std::thread::sleep(Duration::from_millis(self.analysis_time_ms));

        // Generate deterministic mock results based on filename
        let syntax_valid = !uri.contains("syntax_error");
        let symbol_count = uri.len() % 20;
        let violation_count = if uri.contains("violations") { 5 } else { 0 };
        let ast_node_count = symbol_count * 3;

        let analysis = MockFileAnalysis {
            uri: uri.to_string(),
            syntax_valid,
            symbol_count,
            violation_count,
            analysis_time_ms: self.analysis_time_ms,
            ast_node_count,
        };

        self.file_cache.insert(uri.to_string(), analysis.clone());
        analysis
    }

    pub fn get_cached_analysis(&self, uri: &str) -> Option<&MockFileAnalysis> {
        self.file_cache.get(uri)
    }

    pub fn clear_cache(&mut self) {
        self.file_cache.clear();
    }
}

/// Mock workspace for testing cross-file navigation
#[cfg(test)]
pub struct MockWorkspace {
    pub files: HashMap<String, String>,
    pub symbol_index: HashMap<String, Vec<MockSymbol>>,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct MockSymbol {
    pub name: String,
    pub uri: String,
    pub line: u32,
    pub character: u32,
    pub symbol_type: MockSymbolType,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum MockSymbolType {
    Function,
    Variable,
    Package,
    Method,
}

#[cfg(test)]
impl MockWorkspace {
    pub fn new() -> Self {
        let mut workspace = Self {
            files: HashMap::new(),
            symbol_index: HashMap::new(),
        };

        // Add some mock files
        workspace.add_file("file:///test/main.pl", r#"
use strict;
use warnings;
use MyModule;

my $variable = MyModule::process_data("test");
print "Result: $variable\n";
"#);

        workspace.add_file("file:///test/MyModule.pm", r#"
package MyModule;

sub process_data {
    my ($input) = @_;
    return uc($input);
}

1;
"#);

        workspace
    }

    pub fn add_file(&mut self, uri: &str, content: &str) {
        self.files.insert(uri.to_string(), content.to_string());
        self.index_file(uri, content);
    }

    fn index_file(&mut self, uri: &str, content: &str) {
        let mut symbols = Vec::new();

        // Simple pattern matching for mock indexing
        for (line_num, line) in content.lines().enumerate() {
            if let Some(package_match) = extract_package_name(line) {
                symbols.push(MockSymbol {
                    name: package_match,
                    uri: uri.to_string(),
                    line: line_num as u32,
                    character: 0,
                    symbol_type: MockSymbolType::Package,
                });
            }

            if let Some(function_match) = extract_function_name(line) {
                symbols.push(MockSymbol {
                    name: function_match,
                    uri: uri.to_string(),
                    line: line_num as u32,
                    character: 0,
                    symbol_type: MockSymbolType::Function,
                });
            }

            if let Some(variable_match) = extract_variable_name(line) {
                symbols.push(MockSymbol {
                    name: variable_match,
                    uri: uri.to_string(),
                    line: line_num as u32,
                    character: 0,
                    symbol_type: MockSymbolType::Variable,
                });
            }
        }

        for symbol in symbols {
            self.symbol_index
                .entry(symbol.name.clone())
                .or_insert_with(Vec::new)
                .push(symbol);
        }
    }

    pub fn find_definition(&self, symbol: &str) -> Vec<&MockSymbol> {
        self.symbol_index
            .get(symbol)
            .map(|symbols| symbols.iter().collect())
            .unwrap_or_default()
    }

    pub fn find_references(&self, symbol: &str) -> Vec<&MockSymbol> {
        // For mock purposes, references are the same as definitions
        self.find_definition(symbol)
    }

    pub fn get_file_content(&self, uri: &str) -> Option<&String> {
        self.files.get(uri)
    }

    pub fn get_all_symbols(&self) -> Vec<&MockSymbol> {
        self.symbol_index.values().flatten().collect()
    }
}

/// Simple regex-based pattern extraction for mock indexing
#[cfg(test)]
fn extract_package_name(line: &str) -> Option<String> {
    if line.trim_start().starts_with("package ") {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let package_name = parts[1].trim_end_matches(';');
            return Some(package_name.to_string());
        }
    }
    None
}

#[cfg(test)]
fn extract_function_name(line: &str) -> Option<String> {
    if line.trim_start().starts_with("sub ") {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let function_name = parts[1].trim_end_matches('{').trim();
            return Some(function_name.to_string());
        }
    }
    None
}

#[cfg(test)]
fn extract_variable_name(line: &str) -> Option<String> {
    // Simple pattern for my $variable declarations
    if line.contains("my $") {
        let start = line.find("my $")?;
        let var_start = start + 3;
        let remaining = &line[var_start..];
        let var_end = remaining.find(' ').or_else(|| remaining.find('=')).unwrap_or(remaining.len());
        let var_name = &remaining[..var_end].trim();
        if !var_name.is_empty() {
            return Some(var_name.to_string());
        }
    }
    None
}

/// Performance test helpers
#[cfg(test)]
pub struct MockPerformanceTracker {
    start_times: HashMap<String, Instant>,
    measurements: Vec<PerformanceMeasurement>,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct PerformanceMeasurement {
    pub operation: String,
    pub duration_ms: u64,
    pub memory_kb: u64,
    pub timestamp: u64,
}

#[cfg(test)]
impl MockPerformanceTracker {
    pub fn new() -> Self {
        Self {
            start_times: HashMap::new(),
            measurements: Vec::new(),
        }
    }

    pub fn start_operation(&mut self, operation: &str) {
        self.start_times.insert(operation.to_string(), Instant::now());
    }

    pub fn end_operation(&mut self, operation: &str) -> Option<PerformanceMeasurement> {
        if let Some(start_time) = self.start_times.remove(operation) {
            let duration = start_time.elapsed();
            let measurement = PerformanceMeasurement {
                operation: operation.to_string(),
                duration_ms: duration.as_millis() as u64,
                memory_kb: get_mock_memory_usage(),
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
            };
            self.measurements.push(measurement.clone());
            Some(measurement)
        } else {
            None
        }
    }

    pub fn get_measurements(&self) -> &[PerformanceMeasurement] {
        &self.measurements
    }

    pub fn get_average_duration(&self, operation: &str) -> Option<f64> {
        let durations: Vec<u64> = self.measurements
            .iter()
            .filter(|m| m.operation == operation)
            .map(|m| m.duration_ms)
            .collect();

        if durations.is_empty() {
            None
        } else {
            Some(durations.iter().sum::<u64>() as f64 / durations.len() as f64)
        }
    }
}

#[cfg(test)]
fn get_mock_memory_usage() -> u64 {
    // Simulate memory usage between 1MB and 10MB
    1024 + (rand::random::<u64>() % 9216)
}

/// Test data generators for various scenarios
#[cfg(test)]
pub struct MockDataGenerator;

#[cfg(test)]
impl MockDataGenerator {
    pub fn generate_perl_file(name: &str, complexity: FileComplexity) -> String {
        match complexity {
            FileComplexity::Simple => format!(r#"#!/usr/bin/perl
use strict;
use warnings;

# Simple file: {}
my $variable = "test";
print "Hello from {}\n";
"#, name, name),

            FileComplexity::Medium => format!(r#"#!/usr/bin/perl
use strict;
use warnings;

# Medium complexity file: {}
package {};

sub process_data {{
    my ($input) = @_;
    my @results = map {{ $_ * 2 }} split /,/, $input;
    return join(",", @results);
}}

sub validate_input {{
    my ($data) = @_;
    return length($data) > 0;
}}

my $test_data = "1,2,3,4,5";
my $result = process_data($test_data) if validate_input($test_data);
print "Result: $result\n";

1;
"#, name, name),

            FileComplexity::Complex => format!(r#"#!/usr/bin/perl
use strict;
use warnings;
use feature 'signatures';
no warnings 'experimental::signatures';

# Complex file: {} with modern features
package {}::Advanced;

# Subroutine with signatures
sub complex_processing($data, $options = {{}}) {{
    state %cache;

    my $cache_key = join(":", sort keys %$options);
    return $cache{{$cache_key}} if exists $cache{{$cache_key}};

    my @processed = map {{
        my $item = $_;
        $item->{{value}} *= $options->{{multiplier}} // 1;
        $item->{{processed}} = 1;
        $item;
    }} grep {{
        $_->{{active}} && $_->{{value}} > 0
    }} @$data;

    $cache{{$cache_key}} = \@processed;
    return \@processed;
}}

# Error handling with try/catch (if available)
sub safe_operation($risky_data) {{
    eval {{
        die "Invalid data" unless ref $risky_data eq 'ARRAY';
        return complex_processing($risky_data, {{multiplier => 2}});
    }};
    if ($@) {{
        warn "Operation failed: $@";
        return [];
    }}
}}

# Object-oriented features
sub new($class, %args) {{
    my $self = {{
        data => $args{{data}} // [],
        config => $args{{config}} // {{}},
        stats => {{
            operations => 0,
            errors => 0
        }}
    }};
    return bless $self, $class;
}}

sub analyze($self) {{
    $self->{{stats}}->{{operations}}++;

    try {{
        my $results = $self->complex_processing(
            $self->{{data}},
            $self->{{config}}
        );
        return {{
            status => 'success',
            count => scalar @$results,
            data => $results
        }};
    }} catch {{
        $self->{{stats}}->{{errors}}++;
        return {{
            status => 'error',
            message => "Analysis failed"
        }};
    }}
}}

1;
"#, name, name),
        }
    }

    pub fn generate_test_workspace() -> MockWorkspace {
        let mut workspace = MockWorkspace::new();

        workspace.add_file(
            "file:///test/simple.pl",
            &Self::generate_perl_file("Simple", FileComplexity::Simple)
        );

        workspace.add_file(
            "file:///test/medium.pl",
            &Self::generate_perl_file("Medium", FileComplexity::Medium)
        );

        workspace.add_file(
            "file:///test/complex.pl",
            &Self::generate_perl_file("Complex", FileComplexity::Complex)
        );

        workspace
    }
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum FileComplexity {
    Simple,
    Medium,
    Complex,
}

// Add rand dependency for error simulation
#[cfg(test)]
mod rand {
    pub fn random<T>() -> T
    where
        T: From<u32>
    {
        // Simple mock random number generator
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        std::thread::current().id().hash(&mut hasher);
        let hash = hasher.finish();
        T::from((hash % 100) as u32)
    }
}

#[cfg(test)]
mod chrono {
    pub struct Utc;

    impl Utc {
        pub fn now() -> DateTime {
            DateTime
        }
    }

    pub struct DateTime;

    impl DateTime {
        pub fn timestamp_millis(&self) -> i64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64
        }
    }
}