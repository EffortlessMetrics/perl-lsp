//! Parser test corpus with dual indexing validation scenarios
//!
//! Comprehensive test corpus for validating the dual indexing strategy where
//! functions are indexed under both qualified (Package::function) and bare (function) names
//! for 98% reference coverage in cross-file navigation.
//!
//! Features:
//! - Property-based testing data with comprehensive Perl syntax coverage
//! - Cross-file navigation scenarios with package boundaries
//! - Method call resolution with dual pattern matching
//! - Import/export analysis with qualified vs. bare usage tracking
//! - Edge case validation for complex inheritance hierarchies

use serde_json::{json, Value};
use std::collections::HashMap;

#[cfg(test)]
pub struct DualIndexingCorpusEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub perl_files: Vec<(&'static str, &'static str)>, // (filename, content)
    pub expected_qualified_refs: Vec<QualifiedReference>,
    pub expected_bare_refs: Vec<BareReference>,
    pub cross_file_links: Vec<CrossFileLink>,
    pub indexing_efficiency: f32, // Expected coverage percentage
    pub test_category: DualIndexingCategory,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum DualIndexingCategory {
    BasicPackageResolution,
    CrossFileNavigation,
    MethodCallResolution,
    ImportExportAnalysis,
    ComplexInheritance,
    EdgeCases,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct QualifiedReference {
    pub symbol: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub reference_type: ReferenceType,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct BareReference {
    pub symbol: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub reference_type: ReferenceType,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct CrossFileLink {
    pub from_file: String,
    pub to_file: String,
    pub symbol: String,
    pub link_type: LinkType,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceType {
    Definition,
    Call,
    Import,
    Export,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum LinkType {
    SubroutineCall,
    MethodCall,
    PackageImport,
    Inheritance,
}

/// Basic package resolution corpus with dual indexing
#[cfg(test)]
pub fn basic_package_resolution_corpus() -> DualIndexingCorpusEntry {
    DualIndexingCorpusEntry {
        name: "basic_package_resolution",
        description: "Basic package-qualified and bare function resolution scenarios",
        perl_files: vec![
            ("main.pl", r#"#!/usr/bin/perl
use strict;
use warnings;

use lib '.';
use MyModule::Utils;

# Qualified function calls (should be indexed under qualified name)
my $result1 = MyModule::Utils::process_data("test data");
my $result2 = MyModule::Utils::validate_input("input");

# Bare function calls (should also resolve via dual indexing)
my $result3 = process_data("test data");
my $result4 = validate_input("input");

# Mixed usage patterns
if (MyModule::Utils::is_valid($result1)) {
    print "Result 1 is valid\n";
}

if (is_valid($result3)) {
    print "Result 3 is valid\n";
}

print "Processing complete\n";
"#),
            ("MyModule/Utils.pm", r#"package MyModule::Utils;
use strict;
use warnings;

# Qualified subroutine definitions
sub MyModule::Utils::process_data {
    my ($data) = @_;
    return validate_input($data) ? transform_data($data) : undef;
}

# Bare subroutine definitions (dual indexing targets)
sub validate_input {
    my ($input) = @_;
    return defined $input && length($input) > 0;
}

sub transform_data {
    my ($data) = @_;
    return uc($data);
}

sub is_valid {
    my ($value) = @_;
    return defined $value;
}

# Export functions for external use
sub exported_function {
    my ($param) = @_;
    return process_data($param);
}

1;
"#),
        ],
        expected_qualified_refs: vec![
            QualifiedReference {
                symbol: "MyModule::Utils::process_data".to_string(),
                file: "main.pl".to_string(),
                line: 8,
                column: 15,
                reference_type: ReferenceType::Call,
            },
            QualifiedReference {
                symbol: "MyModule::Utils::validate_input".to_string(),
                file: "main.pl".to_string(),
                line: 9,
                column: 15,
                reference_type: ReferenceType::Call,
            },
            QualifiedReference {
                symbol: "MyModule::Utils::is_valid".to_string(),
                file: "main.pl".to_string(),
                line: 15,
                column: 4,
                reference_type: ReferenceType::Call,
            },
        ],
        expected_bare_refs: vec![
            BareReference {
                symbol: "process_data".to_string(),
                file: "main.pl".to_string(),
                line: 12,
                column: 15,
                reference_type: ReferenceType::Call,
            },
            BareReference {
                symbol: "validate_input".to_string(),
                file: "main.pl".to_string(),
                line: 13,
                column: 15,
                reference_type: ReferenceType::Call,
            },
            BareReference {
                symbol: "is_valid".to_string(),
                file: "main.pl".to_string(),
                line: 19,
                column: 4,
                reference_type: ReferenceType::Call,
            },
        ],
        cross_file_links: vec![
            CrossFileLink {
                from_file: "main.pl".to_string(),
                to_file: "MyModule/Utils.pm".to_string(),
                symbol: "process_data".to_string(),
                link_type: LinkType::SubroutineCall,
            },
            CrossFileLink {
                from_file: "main.pl".to_string(),
                to_file: "MyModule/Utils.pm".to_string(),
                symbol: "validate_input".to_string(),
                link_type: LinkType::SubroutineCall,
            },
        ],
        indexing_efficiency: 98.5,
        test_category: DualIndexingCategory::BasicPackageResolution,
    }
}

/// Complex cross-file navigation corpus
#[cfg(test)]
pub fn cross_file_navigation_corpus() -> DualIndexingCorpusEntry {
    DualIndexingCorpusEntry {
        name: "cross_file_navigation_complex",
        description: "Complex cross-file navigation with multiple packages and nested calls",
        perl_files: vec![
            ("application.pl", r#"#!/usr/bin/perl
use strict;
use warnings;

use Data::Processor;
use Config::Manager;
use Logger::Utils;

# Complex navigation scenario with multiple packages
my $config = Config::Manager::load_config("app.conf");
my $logger = Logger::Utils::create_logger($config);

# Processing data with cross-package calls
my $processor = Data::Processor->new($config, $logger);
my $data = $processor->load_data("input.dat");
my $result = $processor->process($data);

# Bare function calls that should resolve via dual indexing
my $validated = validate_data($data);
my $formatted = format_output($result);

# Mixed qualified and bare usage
if (Data::Processor::is_processable($data)) {
    Log::info("Data is processable");
    my $processed = process_with_validation($data);
}

print "Application completed\n";
"#),
            ("Data/Processor.pm", r#"package Data::Processor;
use strict;
use warnings;

use Logger::Utils;

sub new {
    my ($class, $config, $logger) = @_;
    return bless {
        config => $config,
        logger => $logger
    }, $class;
}

sub load_data {
    my ($self, $filename) = @_;
    Logger::Utils::debug("Loading data from $filename");
    return read_file_content($filename);
}

sub process {
    my ($self, $data) = @_;
    return transform_data($data) if is_processable($data);
    return undef;
}

# Static methods (dual indexing targets)
sub is_processable {
    my ($data) = @_;
    return defined $data && ref $data eq 'HASH';
}

sub validate_data {
    my ($data) = @_;
    return is_processable($data) && keys %$data > 0;
}

sub transform_data {
    my ($data) = @_;
    my %transformed = map { $_ => uc($data->{$_}) } keys %$data;
    return \%transformed;
}

sub process_with_validation {
    my ($data) = @_;
    return validate_data($data) ? transform_data($data) : {};
}

1;
"#),
            ("Config/Manager.pm", r#"package Config::Manager;
use strict;
use warnings;

sub load_config {
    my ($filename) = @_;
    return parse_config_file($filename);
}

sub parse_config_file {
    my ($filename) = @_;
    # Simplified config parsing
    return {
        log_level => 'debug',
        data_dir => '/tmp/data'
    };
}

sub get_setting {
    my ($config, $key) = @_;
    return $config->{$key};
}

1;
"#),
            ("Logger/Utils.pm", r#"package Logger::Utils;
use strict;
use warnings;

sub create_logger {
    my ($config) = @_;
    return bless { level => $config->{log_level} }, __PACKAGE__;
}

sub debug {
    my ($message) = @_;
    print "[DEBUG] $message\n";
}

sub info {
    my ($message) = @_;
    print "[INFO] $message\n";
}

sub Log::info {
    my ($message) = @_;
    info($message);  # Delegate to bare function
}

1;
"#),
        ],
        expected_qualified_refs: vec![
            QualifiedReference {
                symbol: "Config::Manager::load_config".to_string(),
                file: "application.pl".to_string(),
                line: 10,
                column: 13,
                reference_type: ReferenceType::Call,
            },
            QualifiedReference {
                symbol: "Logger::Utils::create_logger".to_string(),
                file: "application.pl".to_string(),
                line: 11,
                column: 14,
                reference_type: ReferenceType::Call,
            },
            QualifiedReference {
                symbol: "Data::Processor::is_processable".to_string(),
                file: "application.pl".to_string(),
                line: 22,
                column: 4,
                reference_type: ReferenceType::Call,
            },
        ],
        expected_bare_refs: vec![
            BareReference {
                symbol: "validate_data".to_string(),
                file: "application.pl".to_string(),
                line: 19,
                column: 17,
                reference_type: ReferenceType::Call,
            },
            BareReference {
                symbol: "format_output".to_string(),
                file: "application.pl".to_string(),
                line: 20,
                column: 16,
                reference_type: ReferenceType::Call,
            },
            BareReference {
                symbol: "process_with_validation".to_string(),
                file: "application.pl".to_string(),
                line: 24,
                column: 20,
                reference_type: ReferenceType::Call,
            },
        ],
        cross_file_links: vec![
            CrossFileLink {
                from_file: "application.pl".to_string(),
                to_file: "Data/Processor.pm".to_string(),
                symbol: "validate_data".to_string(),
                link_type: LinkType::SubroutineCall,
            },
            CrossFileLink {
                from_file: "application.pl".to_string(),
                to_file: "Config/Manager.pm".to_string(),
                symbol: "load_config".to_string(),
                link_type: LinkType::SubroutineCall,
            },
            CrossFileLink {
                from_file: "Data/Processor.pm".to_string(),
                to_file: "Logger/Utils.pm".to_string(),
                symbol: "debug".to_string(),
                link_type: LinkType::SubroutineCall,
            },
        ],
        indexing_efficiency: 97.8,
        test_category: DualIndexingCategory::CrossFileNavigation,
    }
}

/// Method call resolution corpus
#[cfg(test)]
pub fn method_call_resolution_corpus() -> DualIndexingCorpusEntry {
    DualIndexingCorpusEntry {
        name: "method_call_resolution",
        description: "Object-oriented method call resolution with inheritance",
        perl_files: vec![
            ("oop_test.pl", r#"#!/usr/bin/perl
use strict;
use warnings;

use Animal::Dog;
use Animal::Cat;
use Vehicle::Car;

# Object creation and method calls
my $dog = Animal::Dog->new("Buddy");
my $cat = Animal::Cat->new("Whiskers");
my $car = Vehicle::Car->new("Toyota", "Camry");

# Instance method calls (should be indexed)
$dog->speak();
$cat->speak();
$dog->fetch("ball");

# Class method calls
my $dog_sound = Animal::Dog->get_species_sound();
my $cat_sound = Animal::Cat->get_species_sound();

# Polymorphic method calls
my @animals = ($dog, $cat);
foreach my $animal (@animals) {
    $animal->speak();  # Polymorphic call
    $animal->move() if $animal->can('move');
}

# Chained method calls
$car->start()->accelerate(60)->brake();

# Mixed static and instance calls
if (Animal::Dog::is_domestic()) {
    $dog->train("sit");
}

print "OOP test completed\n";
"#),
            ("Animal/Dog.pm", r#"package Animal::Dog;
use strict;
use warnings;

use base 'Animal::Base';

sub new {
    my ($class, $name) = @_;
    my $self = $class->SUPER::new($name);
    $self->{species} = 'dog';
    return $self;
}

sub speak {
    my ($self) = @_;
    print $self->{name} . " says: Woof!\n";
    return $self;
}

sub fetch {
    my ($self, $item) = @_;
    print $self->{name} . " fetches the $item\n";
    return $self;
}

sub train {
    my ($self, $command) = @_;
    print "Training " . $self->{name} . " to $command\n";
    return $self;
}

# Class methods (dual indexing targets)
sub get_species_sound {
    return "Woof";
}

sub is_domestic {
    return 1;
}

1;
"#),
            ("Animal/Cat.pm", r#"package Animal::Cat;
use strict;
use warnings;

use base 'Animal::Base';

sub new {
    my ($class, $name) = @_;
    my $self = $class->SUPER::new($name);
    $self->{species} = 'cat';
    return $self;
}

sub speak {
    my ($self) = @_;
    print $self->{name} . " says: Meow!\n";
    return $self;
}

sub climb {
    my ($self, $object) = @_;
    print $self->{name} . " climbs the $object\n";
    return $self;
}

# Class methods
sub get_species_sound {
    return "Meow";
}

sub is_independent {
    return 1;
}

1;
"#),
            ("Animal/Base.pm", r#"package Animal::Base;
use strict;
use warnings;

sub new {
    my ($class, $name) = @_;
    return bless {
        name => $name,
        age => 0
    }, $class;
}

sub move {
    my ($self) = @_;
    print $self->{name} . " is moving\n";
    return $self;
}

sub get_name {
    my ($self) = @_;
    return $self->{name};
}

sub set_age {
    my ($self, $age) = @_;
    $self->{age} = $age;
    return $self;
}

1;
"#),
            ("Vehicle/Car.pm", r#"package Vehicle::Car;
use strict;
use warnings;

sub new {
    my ($class, $make, $model) = @_;
    return bless {
        make => $make,
        model => $model,
        speed => 0,
        engine_running => 0
    }, $class;
}

sub start {
    my ($self) = @_;
    $self->{engine_running} = 1;
    print "Starting " . $self->{make} . " " . $self->{model} . "\n";
    return $self;  # Enable chaining
}

sub accelerate {
    my ($self, $target_speed) = @_;
    if ($self->{engine_running}) {
        $self->{speed} = $target_speed;
        print "Accelerating to $target_speed mph\n";
    }
    return $self;  # Enable chaining
}

sub brake {
    my ($self) = @_;
    $self->{speed} = 0;
    print "Braking to stop\n";
    return $self;  # Enable chaining
}

1;
"#),
        ],
        expected_qualified_refs: vec![
            QualifiedReference {
                symbol: "Animal::Dog::get_species_sound".to_string(),
                file: "oop_test.pl".to_string(),
                line: 18,
                column: 18,
                reference_type: ReferenceType::Call,
            },
            QualifiedReference {
                symbol: "Animal::Cat::get_species_sound".to_string(),
                file: "oop_test.pl".to_string(),
                line: 19,
                column: 18,
                reference_type: ReferenceType::Call,
            },
            QualifiedReference {
                symbol: "Animal::Dog::is_domestic".to_string(),
                file: "oop_test.pl".to_string(),
                line: 30,
                column: 4,
                reference_type: ReferenceType::Call,
            },
        ],
        expected_bare_refs: vec![
            BareReference {
                symbol: "speak".to_string(),
                file: "oop_test.pl".to_string(),
                line: 14,
                column: 6,
                reference_type: ReferenceType::Call,
            },
            BareReference {
                symbol: "fetch".to_string(),
                file: "oop_test.pl".to_string(),
                line: 16,
                column: 6,
                reference_type: ReferenceType::Call,
            },
            BareReference {
                symbol: "move".to_string(),
                file: "oop_test.pl".to_string(),
                line: 24,
                column: 13,
                reference_type: ReferenceType::Call,
            },
        ],
        cross_file_links: vec![
            CrossFileLink {
                from_file: "oop_test.pl".to_string(),
                to_file: "Animal/Dog.pm".to_string(),
                symbol: "speak".to_string(),
                link_type: LinkType::MethodCall,
            },
            CrossFileLink {
                from_file: "oop_test.pl".to_string(),
                to_file: "Animal/Cat.pm".to_string(),
                symbol: "speak".to_string(),
                link_type: LinkType::MethodCall,
            },
            CrossFileLink {
                from_file: "Animal/Dog.pm".to_string(),
                to_file: "Animal/Base.pm".to_string(),
                symbol: "new".to_string(),
                link_type: LinkType::Inheritance,
            },
        ],
        indexing_efficiency: 96.2,
        test_category: DualIndexingCategory::MethodCallResolution,
    }
}

/// Load all dual indexing corpus entries
#[cfg(test)]
pub fn load_dual_indexing_corpus() -> Vec<DualIndexingCorpusEntry> {
    vec![
        basic_package_resolution_corpus(),
        cross_file_navigation_corpus(),
        method_call_resolution_corpus(),
    ]
}

/// Load corpus by category
#[cfg(test)]
pub fn load_corpus_by_category(category: DualIndexingCategory) -> Vec<DualIndexingCorpusEntry> {
    load_dual_indexing_corpus()
        .into_iter()
        .filter(|entry| entry.test_category == category)
        .collect()
}

/// Calculate expected indexing efficiency across all corpus entries
#[cfg(test)]
pub fn calculate_average_indexing_efficiency() -> f32 {
    let corpus = load_dual_indexing_corpus();
    let total: f32 = corpus.iter().map(|entry| entry.indexing_efficiency).sum();
    total / corpus.len() as f32
}

use std::sync::LazyLock;

/// Lazy-loaded dual indexing corpus registry
#[cfg(test)]
pub static DUAL_INDEXING_CORPUS: LazyLock<HashMap<&'static str, DualIndexingCorpusEntry>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();

        for entry in load_dual_indexing_corpus() {
            registry.insert(entry.name, entry);
        }

        registry
    });

/// Get corpus entry by name
#[cfg(test)]
pub fn get_corpus_entry_by_name(name: &str) -> Option<&'static DualIndexingCorpusEntry> {
    DUAL_INDEXING_CORPUS.get(name)
}

/// Property-based testing utilities for dual indexing validation
#[cfg(test)]
pub mod property_testing {
    use super::*;

    /// Validate that all expected references are properly indexed
    pub fn validate_dual_indexing_coverage(entry: &DualIndexingCorpusEntry) -> bool {
        // Check that qualified references have corresponding bare references where applicable
        let qualified_symbols: std::collections::HashSet<String> = entry
            .expected_qualified_refs
            .iter()
            .map(|r| extract_bare_symbol(&r.symbol))
            .collect();

        let bare_symbols: std::collections::HashSet<String> = entry
            .expected_bare_refs
            .iter()
            .map(|r| r.symbol.clone())
            .collect();

        // Dual indexing coverage: most qualified symbols should also have bare equivalents
        let intersection_count = qualified_symbols.intersection(&bare_symbols).count();
        let coverage_ratio = intersection_count as f32 / qualified_symbols.len() as f32;

        coverage_ratio >= 0.8 // 80% minimum dual indexing coverage
    }

    /// Extract bare symbol name from qualified symbol
    fn extract_bare_symbol(qualified: &str) -> String {
        qualified.split("::").last().unwrap_or(qualified).to_string()
    }

    /// Validate cross-file link consistency
    pub fn validate_cross_file_links(entry: &DualIndexingCorpusEntry) -> bool {
        // Check that all cross-file links reference existing files
        for link in &entry.cross_file_links {
            let from_exists = entry.perl_files.iter().any(|(name, _)| name == &link.from_file);
            let to_exists = entry.perl_files.iter().any(|(name, _)| name == &link.to_file);

            if !from_exists || !to_exists {
                return false;
            }
        }

        true
    }
}