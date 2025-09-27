//! Modern Perl syntax test fixtures for comprehensive language support
//!
//! Provides realistic Perl code samples covering modern Perl 5 features
//! that may not be fully tested in existing fixtures:
//! - Subroutine signatures (v5.20+)
//! - Postfix dereferencing (v5.20+)
//! - State variables with enhanced scoping
//! - Try/catch blocks (experimental)
//! - Unicode variable names and strings
//! - Complex nested data structures
//! - Modern object-oriented patterns

#[cfg(test)]
pub struct ModernPerlFixture {
    pub name: &'static str,
    pub perl_code: &'static str,
    pub perl_version_required: &'static str,
    pub expected_ast_nodes: usize,
    pub parsing_time_us: Option<u64>,
    pub feature_category: ModernFeatureCategory,
    pub stability: FeatureStability,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum ModernFeatureCategory {
    SubroutineSignatures,
    PostfixDereferencing,
    StateVariables,
    ExperimentalFeatures,
    UnicodeAdvanced,
    ObjectOriented,
    AsyncPatterns,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum FeatureStability {
    Stable,
    Experimental,
    Deprecated,
}

/// Modern Perl syntax test data with latest language features
#[cfg(test)]
pub fn load_modern_perl_fixtures() -> Vec<ModernPerlFixture> {
    vec![
        // Subroutine signatures (Perl v5.20+)
        ModernPerlFixture {
            name: "subroutine_signatures_comprehensive",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;
use feature 'signatures';
no warnings 'experimental::signatures';

# Basic subroutine signatures
sub add($x, $y) {
    return $x + $y;
}

# Default parameters in signatures
sub greet($name = "World") {
    return "Hello, $name!";
}

# Optional parameters with slurpy
sub process_data($required, $optional = undef, @rest) {
    my $result = $required;
    $result .= " $optional" if defined $optional;
    $result .= " " . join(", ", @rest) if @rest;
    return $result;
}

# Named parameters simulation
sub configure($host, $port = 80, %options) {
    my $config = {
        host => $host,
        port => $port,
        %options
    };
    return $config;
}

# Complex signature with type hints in comments
sub calculate_area(
    $length,    # numeric: length in meters
    $width,     # numeric: width in meters
    $unit = 'm' # string: unit of measurement
) {
    my $area = $length * $width;
    return "$area $unit²";
}

# Method with signature
package Calculator;

sub new($class, $precision = 2) {
    my $self = {
        precision => $precision
    };
    return bless $self, $class;
}

sub divide($self, $numerator, $denominator) {
    die "Division by zero" if $denominator == 0;
    my $result = $numerator / $denominator;
    return sprintf("%.${$self}{precision}f", $result);
}

# Usage examples
my $sum = add(5, 3);
my $greeting = greet();
my $custom_greeting = greet("Alice");
my $processed = process_data("base", "extra", "arg1", "arg2");
my $config = configure("localhost", 8080, ssl => 1, timeout => 30);
my $area = calculate_area(10, 5, 'ft');

my $calc = Calculator->new(4);
my $result = $calc->divide(10, 3);

1;
"#,
            perl_version_required: "5.20",
            expected_ast_nodes: 95,
            parsing_time_us: Some(280),
            feature_category: ModernFeatureCategory::SubroutineSignatures,
            stability: FeatureStability::Stable,
        },

        // Postfix dereferencing (Perl v5.20+)
        ModernPerlFixture {
            name: "postfix_dereferencing_comprehensive",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;
use feature 'postderef';
no warnings 'experimental::postderef';

# Complex nested data structure
my $data = {
    users => [
        {
            name => "Alice",
            addresses => [
                { type => "home", city => "New York" },
                { type => "work", city => "Boston" }
            ],
            preferences => {
                theme => "dark",
                notifications => {
                    email => 1,
                    sms => 0
                }
            }
        },
        {
            name => "Bob",
            addresses => [
                { type => "home", city => "Seattle" }
            ],
            preferences => {
                theme => "light",
                notifications => {
                    email => 0,
                    sms => 1
                }
            }
        }
    ],
    settings => {
        version => "1.0",
        features => ["auth", "api", "ui"]
    }
};

# Traditional dereferencing vs postfix
my @users_traditional = @{$data->{users}};
my @users_postfix = $data->{users}->@*;

# Array access with postfix
my $first_user = $data->{users}->[0];
my @addresses_traditional = @{$first_user->{addresses}};
my @addresses_postfix = $first_user->{addresses}->@*;

# Hash access with postfix
my %preferences_traditional = %{$first_user->{preferences}};
my %preferences_postfix = $first_user->{preferences}->%*;

# Complex chained postfix dereferencing
my @all_cities = map { $_->{city} }
                 map { $_->{addresses}->@* }
                 $data->{users}->@*;

# Postfix with array slices
my @first_two_users = $data->{users}->@[0,1];
my @feature_subset = $data->{settings}->{features}->@[0,2];

# Postfix with hash slices
my @notification_values = $first_user->{preferences}->{notifications}->%{qw(email sms)};

# Method calls with postfix dereferencing
sub get_user_cities {
    my ($users_ref) = @_;
    return map {
        map { $_->{city} } $_->{addresses}->@*
    } $users_ref->@*;
}

my @cities = get_user_cities($data->{users});

# Complex postfix in subroutine
sub analyze_preferences {
    my ($data_ref) = @_;

    my %theme_count;
    my %notification_stats = (email => 0, sms => 0);

    for my $user ($data_ref->{users}->@*) {
        my $theme = $user->{preferences}->{theme};
        $theme_count{$theme}++;

        my %notifications = $user->{preferences}->{notifications}->%*;
        $notification_stats{email} += $notifications{email};
        $notification_stats{sms} += $notifications{sms};
    }

    return {
        themes => \%theme_count,
        notifications => \%notification_stats
    };
}

my $analysis = analyze_preferences($data);
my @theme_names = $analysis->{themes}->%*;

# Anonymous reference creation and immediate postfix
my @processed_data = ({
    raw => [1, 2, 3],
    processed => [map { $_ * 2 } (1, 2, 3)]
})->{processed}->@*;

1;
"#,
            perl_version_required: "5.20",
            expected_ast_nodes: 85,
            parsing_time_us: Some(320),
            feature_category: ModernFeatureCategory::PostfixDereferencing,
            stability: FeatureStability::Stable,
        },

        // State variables with enhanced scoping
        ModernPerlFixture {
            name: "state_variables_comprehensive",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;
use feature 'state';

# Basic state variable usage
sub counter {
    state $count = 0;
    return ++$count;
}

# State with complex initialization
sub fibonacci {
    state @sequence = (0, 1);
    state $index = 0;

    if ($index >= @sequence) {
        push @sequence, $sequence[-1] + $sequence[-2];
    }

    return $sequence[$index++];
}

# State variables in different scopes
sub cache_manager {
    my ($action, $key, $value) = @_;

    state %cache;
    state $max_size = 100;
    state $current_size = 0;

    if ($action eq 'get') {
        return $cache{$key};
    }
    elsif ($action eq 'set') {
        if (!exists $cache{$key} && $current_size >= $max_size) {
            # Simple LRU: remove oldest entry
            my $oldest_key = (sort keys %cache)[0];
            delete $cache{$oldest_key};
            $current_size--;
        }

        if (!exists $cache{$key}) {
            $current_size++;
        }

        $cache{$key} = $value;
        return $value;
    }
    elsif ($action eq 'clear') {
        %cache = ();
        $current_size = 0;
        return 1;
    }
    elsif ($action eq 'stats') {
        return {
            size => $current_size,
            max_size => $max_size,
            keys => [keys %cache]
        };
    }
}

# State variables in object context
package SessionManager;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub get_session_id {
    my ($self) = @_;

    # State persists across all instances
    state $session_counter = 1000;
    state %active_sessions;

    my $session_id = sprintf("sess_%d_%d", time(), ++$session_counter);
    $active_sessions{$session_id} = time();

    return $session_id;
}

sub cleanup_sessions {
    my ($self, $max_age) = @_;

    state %active_sessions;
    my $current_time = time();
    my $cleaned = 0;

    for my $session_id (keys %active_sessions) {
        if ($current_time - $active_sessions{$session_id} > $max_age) {
            delete $active_sessions{$session_id};
            $cleaned++;
        }
    }

    return $cleaned;
}

# Usage examples
print "Counter: " . counter() . "\n";  # 1
print "Counter: " . counter() . "\n";  # 2
print "Counter: " . counter() . "\n";  # 3

print "Fibonacci: " . fibonacci() . "\n";  # 0
print "Fibonacci: " . fibonacci() . "\n";  # 1
print "Fibonacci: " . fibonacci() . "\n";  # 1
print "Fibonacci: " . fibonacci() . "\n";  # 2

cache_manager('set', 'user1', { name => 'Alice', age => 30 });
cache_manager('set', 'user2', { name => 'Bob', age => 25 });
my $user1 = cache_manager('get', 'user1');
my $stats = cache_manager('stats');

my $session_mgr = SessionManager->new();
my $session1 = $session_mgr->get_session_id();
my $session2 = $session_mgr->get_session_id();

# State with anonymous subroutines
my $closure_counter = sub {
    state $private_count = 100;
    return ++$private_count;
};

print "Closure: " . $closure_counter->() . "\n";  # 101
print "Closure: " . $closure_counter->() . "\n";  # 102

1;
"#,
            perl_version_required: "5.10",
            expected_ast_nodes: 120,
            parsing_time_us: Some(350),
            feature_category: ModernFeatureCategory::StateVariables,
            stability: FeatureStability::Stable,
        },

        // Experimental features (try/catch, etc.)
        ModernPerlFixture {
            name: "experimental_features_comprehensive",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;
use feature 'try';
no warnings 'experimental::try';

# Try/catch blocks (experimental in Perl 5.34+)
sub safe_divide {
    my ($numerator, $denominator) = @_;

    try {
        die "Division by zero" if $denominator == 0;
        die "Invalid arguments" unless defined $numerator && defined $denominator;

        my $result = $numerator / $denominator;
        return $result;
    }
    catch ($e) {
        warn "Division error: $e";
        return undef;
    }
}

# Nested try/catch
sub complex_operation {
    my ($data) = @_;

    try {
        try {
            my $processed = process_data($data);
            my $validated = validate_result($processed);
            return $validated;
        }
        catch ($inner_error) {
            warn "Inner operation failed: $inner_error";
            # Fallback operation
            return fallback_process($data);
        }
    }
    catch ($outer_error) {
        warn "All operations failed: $outer_error";
        return { error => "Complete failure", data => $data };
    }
}

sub process_data {
    my ($data) = @_;
    die "Invalid data format" unless ref $data eq 'HASH';
    die "Missing required field" unless exists $data->{id};
    return { processed => $data->{id} * 2 };
}

sub validate_result {
    my ($result) = @_;
    die "Validation failed" unless $result->{processed} > 0;
    return $result;
}

sub fallback_process {
    my ($data) = @_;
    return { fallback => 1, original => $data };
}

# Mixed traditional eval and modern try/catch
sub hybrid_error_handling {
    my ($risky_operation) = @_;

    # Traditional eval for older Perl compatibility
    my $eval_result = eval {
        # Some operation that might fail
        return perform_risky_operation($risky_operation);
    };

    if ($@) {
        warn "Eval caught: $@";

        # Modern try/catch for newer error handling
        try {
            return fallback_operation($risky_operation);
        }
        catch ($modern_error) {
            warn "Modern catch: $modern_error";
            return { status => 'failed', error => $modern_error };
        }
    }

    return $eval_result;
}

sub perform_risky_operation {
    my ($op) = @_;
    die "Risky operation failed" if rand() < 0.3;
    return { status => 'success', data => $op };
}

sub fallback_operation {
    my ($op) = @_;
    die "Fallback also failed" if rand() < 0.1;
    return { status => 'fallback_success', data => $op };
}

# Usage examples
my $result1 = safe_divide(10, 2);      # 5
my $result2 = safe_divide(10, 0);      # undef with warning

my $complex_result = complex_operation({ id => 42 });
my $failed_result = complex_operation({ invalid => "data" });

my $hybrid_result = hybrid_error_handling("test_operation");

# Try/catch with resource cleanup
sub file_operation_with_cleanup {
    my ($filename) = @_;

    my $fh;
    try {
        open $fh, '<', $filename or die "Cannot open $filename: $!";
        my $content = do { local $/; <$fh> };
        close $fh;
        return { success => 1, content => $content };
    }
    catch ($error) {
        close $fh if defined $fh;
        return { success => 0, error => $error };
    }
}

1;
"#,
            perl_version_required: "5.34",
            expected_ast_nodes: 98,
            parsing_time_us: Some(380),
            feature_category: ModernFeatureCategory::ExperimentalFeatures,
            stability: FeatureStability::Experimental,
        },

        // Advanced Unicode and internationalization
        ModernPerlFixture {
            name: "unicode_advanced_comprehensive",
            perl_code: r#"#!/usr/bin/perl
use strict;
use warnings;
use utf8;
use Unicode::Normalize;
use Encode qw(encode decode);

# Advanced Unicode variable names and processing
my $café_données = {
    '🏠' => "home",
    '🏢' => "office",
    '🚗' => "car",
    '✈️' => "plane"
};

# Unicode normalization
sub normalize_text {
    my ($text, $form) = @_;
    $form //= 'NFC';  # Default to canonical composed form

    my %normalizers = (
        'NFC'  => sub { NFC($_[0]) },   # Canonical Composed
        'NFD'  => sub { NFD($_[0]) },   # Canonical Decomposed
        'NFKC' => sub { NFKC($_[0]) },  # Compatibility Composed
        'NFKD' => sub { NFKD($_[0]) },  # Compatibility Decomposed
    );

    die "Unknown normalization form: $form" unless exists $normalizers{$form};
    return $normalizers{$form}->($text);
}

# Multi-language text processing
my %multilingual_data = (
    english => "Hello World",
    spanish => "Hola Mundo",
    french => "Bonjour le Monde",
    german => "Hallo Welt",
    russian => "Привет мир",
    japanese => "こんにちは世界",
    chinese => "你好世界",
    arabic => "مرحبا بالعالم",
    hebrew => "שלום עולם",
    emoji => "👋🌍",
);

# Unicode-aware string operations
sub analyze_unicode_string {
    my ($string) = @_;

    return {
        length_bytes => length(encode('UTF-8', $string)),
        length_chars => length($string),
        normalized => {
            nfc  => normalize_text($string, 'NFC'),
            nfd  => normalize_text($string, 'NFD'),
            nfkc => normalize_text($string, 'NFKC'),
            nfkd => normalize_text($string, 'NFKD'),
        },
        encoding_info => {
            utf8_valid => eval { decode('UTF-8', encode('UTF-8', $string), Encode::FB_CROAK); 1 },
            ascii_only => $string =~ /^[\x00-\x7F]*$/,
            has_emoji => $string =~ /[\x{1F600}-\x{1F64F}]|[\x{1F300}-\x{1F5FF}]|[\x{1F680}-\x{1F6FF}]|[\x{1F1E0}-\x{1F1FF}]/,
        }
    };
}

# Complex Unicode regex patterns
sub extract_unicode_patterns {
    my ($text) = @_;

    my %patterns = (
        # Various script detection
        latin => qr/\p{Script=Latin}/,
        cyrillic => qr/\p{Script=Cyrillic}/,
        arabic => qr/\p{Script=Arabic}/,
        chinese => qr/\p{Script=Han}/,
        japanese_hiragana => qr/\p{Script=Hiragana}/,
        japanese_katakana => qr/\p{Script=Katakana}/,

        # Character categories
        letters => qr/\p{Letter}/,
        numbers => qr/\p{Number}/,
        punctuation => qr/\p{Punctuation}/,
        symbols => qr/\p{Symbol}/,

        # Specific Unicode blocks
        emoji_emoticons => qr/\p{Block=Emoticons}/,
        emoji_symbols => qr/\p{Block=Miscellaneous_Symbols_And_Pictographs}/,
        emoji_transport => qr/\p{Block=Transport_And_Map_Symbols}/,
    );

    my %matches;
    for my $pattern_name (keys %patterns) {
        my @matches = $text =~ /$patterns{$pattern_name}/g;
        $matches{$pattern_name} = \@matches if @matches;
    }

    return \%matches;
}

# Unicode-aware file I/O
sub read_unicode_file {
    my ($filename, $encoding) = @_;
    $encoding //= 'UTF-8';

    open my $fh, "<:encoding($encoding)", $filename
        or die "Cannot open $filename with encoding $encoding: $!";

    my $content = do { local $/; <$fh> };
    close $fh;

    return {
        content => $content,
        encoding => $encoding,
        analysis => analyze_unicode_string($content),
        patterns => extract_unicode_patterns($content),
    };
}

# Unicode identifiers in package names and subroutines
package Café::Système;

sub traiter_données {
    my ($self, $données) = @_;

    my %résultat;
    for my $clé (keys %$données) {
        my $valeur = $données->{$clé};
        $résultat{$clé} = ref $valeur ? $valeur : normalize_text($valeur);
    }

    return \%résultat;
}

sub calculer_statistiques {
    my ($self, @données) = @_;

    my $total = 0;
    my $count = 0;

    for my $élément (@données) {
        if (looks_like_number($élément)) {
            $total += $élément;
            $count++;
        }
    }

    return {
        total => $total,
        count => $count,
        moyenne => $count > 0 ? $total / $count : 0,
    };
}

# Usage examples
my $unicode_analysis = analyze_unicode_string("Hello 世界! 🌍 Café");
my $patterns = extract_unicode_patterns("Mixed text: English, 中文, العربية, 🚀");

my $café_system = Café::Système->new();
my $processed = $café_system->traiter_données(\%multilingual_data);
my $stats = $café_system->calculer_statistiques(1, 2, 3.14, "π", 42);

# Complex Unicode transformations
my @transformed_text = map {
    my $text = $multilingual_data{$_};
    {
        language => $_,
        original => $text,
        normalized => normalize_text($text, 'NFKC'),
        analysis => analyze_unicode_string($text),
    }
} keys %multilingual_data;

use Scalar::Util qw(looks_like_number);

1;
"#,
            perl_version_required: "5.14",
            expected_ast_nodes: 140,
            parsing_time_us: Some(420),
            feature_category: ModernFeatureCategory::UnicodeAdvanced,
            stability: FeatureStability::Stable,
        },
    ]
}

/// Load fixtures by feature category
#[cfg(test)]
pub fn load_fixtures_by_category(category: ModernFeatureCategory) -> Vec<ModernPerlFixture> {
    load_modern_perl_fixtures()
        .into_iter()
        .filter(|fixture| fixture.feature_category == category)
        .collect()
}

/// Load fixtures by stability level
#[cfg(test)]
pub fn load_fixtures_by_stability(stability: FeatureStability) -> Vec<ModernPerlFixture> {
    load_modern_perl_fixtures()
        .into_iter()
        .filter(|fixture| fixture.stability == stability)
        .collect()
}

/// Load fixtures requiring specific Perl version
#[cfg(test)]
pub fn load_fixtures_by_version(min_version: &str) -> Vec<ModernPerlFixture> {
    load_modern_perl_fixtures()
        .into_iter()
        .filter(|fixture| version_compare(fixture.perl_version_required, min_version))
        .collect()
}

/// Simple version comparison helper
#[cfg(test)]
fn version_compare(required: &str, available: &str) -> bool {
    // Simple comparison for major.minor versions
    let req_parts: Vec<u32> = required.split('.').filter_map(|s| s.parse().ok()).collect();
    let avail_parts: Vec<u32> = available.split('.').filter_map(|s| s.parse().ok()).collect();

    for (i, &req) in req_parts.iter().enumerate() {
        let avail = avail_parts.get(i).unwrap_or(&0);
        if avail < &req {
            return false;
        } else if avail > &req {
            return true;
        }
    }

    true
}

use std::sync::LazyLock;
use std::collections::HashMap;

/// Lazy-loaded modern Perl fixture registry
#[cfg(test)]
pub static MODERN_FIXTURE_REGISTRY: LazyLock<HashMap<&'static str, ModernPerlFixture>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();

        for fixture in load_modern_perl_fixtures() {
            registry.insert(fixture.name, fixture);
        }

        registry
    });

/// Get modern fixture by name
#[cfg(test)]
pub fn get_modern_fixture_by_name(name: &str) -> Option<&'static ModernPerlFixture> {
    MODERN_FIXTURE_REGISTRY.get(name)
}