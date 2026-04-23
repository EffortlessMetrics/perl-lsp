#!/usr/bin/env perl
# Test: Enhanced VariableListDeclaration Production Scenarios
# Impact: Comprehensive testing of complex variable declaration and assignment patterns
# NodeKinds: VariableListDeclaration, ListAssignment, Destructuring
# 
# This file tests the parser's ability to handle:
# 1. Complex list assignment patterns
# 2. Mixed-type variable declarations
# 3. Nested destructuring scenarios
# 4. Performance-optimized list operations
# 5. Real-world data processing patterns
# 6. Advanced array and hash slicing
# 7. Function return value destructuring
# 8. Error handling and edge cases

use strict;
use warnings;

print "=== Enhanced VariableListDeclaration Production Tests ===\n\n";

# Test 1: Basic list assignment patterns
print "=== Basic List Assignment Patterns ===\n";

# Simple scalar list assignment
my ($a, $b, $c) = (1, 2, 3);
print "Simple assignment: a=$a, b=$b, c=$c\n";

# List assignment with expressions
my ($sum, $product, $difference) = (10 + 5, 3 * 4, 20 - 8);
print "Expression assignment: sum=$sum, product=$product, difference=$difference\n";

# List assignment with function calls
sub get_coordinates { return (10.5, 20.7, 30.2); }
my ($lat, $lon, $alt) = get_coordinates();
print "Function return assignment: lat=$lat, lon=$lon, alt=$alt\n";

# List assignment with string operations
my ($concat, $repeat, $substr) = ('Hello' . ' World', 'A' x 5, substr('Perl', 1, 2));
print "String operations: concat=$concat, repeat=$repeat, substr=$substr\n\n";

# Test 2: Mixed-type variable declarations
print "=== Mixed-Type Variable Declarations ===\n";

# Scalars, arrays, and hashes in same declaration
my ($scalar1, @array1, %hash1) = ('scalar_value', (1, 2, 3), (key1 => 'value1', key2 => 'value2'));
print "Mixed types: scalar=$scalar1, array=@array1, hash=" . join(',', %hash1) . "\n";

# Complex mixed assignment
my ($text, @numbers, %config, $final) = (
    'processing',
    (1, 2, 3, 4, 5),
    (debug => 1, verbose => 0, timeout => 30),
    'complete'
);
print "Complex mixed: text=$text, numbers=@numbers, config=" . join(',', %config) . ", final=$final\n";

# References in list assignment
my ($scalar_ref, $array_ref, $hash_ref, $code_ref) = (
    \$scalar1,
    \@array1,
    \%hash1,
    sub { return "code reference result"; }
);
print "References: scalar=" . $$scalar_ref . ", array=" . join(',', @$array_ref) . ", hash=" . $hash_ref->{key1} . ", code=" . $code_ref->() . "\n\n";

# Test 3: Array destructuring patterns
print "=== Array Destructuring Patterns ===\n";

# Head-tail destructuring
my @source_array = (1, 2, 3, 4, 5);
my ($head, @tail) = @source_array;
print "Head-tail: head=$head, tail=@tail\n";

# Body-last destructuring
my (@body, $last) = @source_array;
print "Body-last: body=@body, last=$last\n";

# Multiple elements and rest
my ($first, $second, $third, @rest) = @source_array;
print "Multiple: first=$first, second=$second, third=$third, rest=@rest\n";

# Array slicing in list assignment
my ($elem1, $elem3, $elem5) = @source_array[0, 2, 4];
print "Sliced assignment: elem1=$elem1, elem3=$elem3, elem5=$elem5\n";

# Range-based assignment
my ($range_start, @range_middle, $range_end) = @source_array[1..3];
print "Range assignment: start=$range_start, middle=@range_middle, end=$range_end\n\n";

# Test 4: Hash destructuring patterns
print "=== Hash Destructuring Patterns ===\n";

# Basic hash destructuring
my %source_hash = (name => 'Alice', age => 30, city => 'NYC', job => 'Engineer');
my ($name, $age, $city) = @source_hash{'name', 'age', 'city'};
print "Hash destructuring: name=$name, age=$age, city=$city\n";

# Hash slicing with defaults
my ($country, $state, $zip) = (@source_hash{'country', 'state', 'zip'}, 'USA', 'NY', '10001');
print "Hash with defaults: country=$country, state=$state, zip=$zip\n";

# Multiple hash assignments
my (%hash1_copy, %hash2_copy);
(%hash1_copy, %hash2_copy) = (%source_hash, (extra => 'value', another => 'data'));
print "Multiple hash assignment: hash1=" . join(',', %hash1_copy) . ", hash2=" . join(',', %hash2_copy) . "\n";

# Hash and array mixed destructuring
my ($hash_name, @hash_values) = %source_hash;
print "Hash as list: name=$hash_name, values=@hash_values\n\n";

# Test 5: Function return value destructuring
print "=== Function Return Destructuring ===\n";

# Function returning list
sub get_user_data {
    return (
        'John Doe',
        35,
        'john@example.com',
        ['reading', 'coding', 'gaming'],
        { city => 'Boston', country => 'USA' }
    );
}

my ($user_name, $user_age, $user_email, $user_hobbies, $user_location) = get_user_data();
print "User data: name=$user_name, age=$user_age, email=$user_email\n";
print "Hobbies: " . join(', ', @$user_hobbies) . "\n";
print "Location: " . $user_location->{city} . ", " . $user_location->{country} . "\n";

# Function with conditional returns
sub get_conditional_data {
    my ($type) = @_;
    
    if ($type eq 'user') {
        return ('Alice', 25, 'alice@example.com');
    } elsif ($type eq 'admin') {
        return ('Bob', 40, 'bob@admin.com', 'superuser');
    } else {
        return ('Guest', 0, 'guest@example.com', 'basic', 'temp');
    }
}

my ($cond_name, $cond_age, $cond_email, $cond_role, $cond_status) = get_conditional_data('admin');
print "Conditional data: name=$cond_name, age=$cond_age, email=$cond_email, role=$cond_role, status=$cond_status\n";

# Nested function calls with destructuring
sub calculate_stats {
    my (@numbers) = @_;
    my $sum = 0;
    $sum += $_ for @numbers;
    my $avg = $sum / @numbers;
    my ($min, $max) = (sort { $a <=> $b } @numbers)[0, -1];
    return ($sum, $avg, $min, $max);
}

my ($stats_sum, $stats_avg, $stats_min, $stats_max) = calculate_stats(5, 10, 15, 20, 25);
print "Stats: sum=$stats_sum, avg=$stats_avg, min=$stats_min, max=$stats_max\n\n";

# Test 6: Advanced list operations
print "=== Advanced List Operations ===\n";

# List assignment with map
my @numbers = (1, 2, 3, 4, 5);
my ($doubled1, $doubled2) = map { $_ * 2 } @numbers[0, 1];
print "Map assignment: doubled1=$doubled1, doubled2=$doubled2\n";

# List assignment with grep
my ($even1, $even2, $even3) = grep { $_ % 2 == 0 } @numbers;
print "Grep assignment: even1=$even1, even2=$even2, even3=$even3\n";

# List assignment with sort
my @unsorted = (3, 1, 4, 1, 5, 9, 2);
my ($smallest, $second_smallest, $largest) = (sort @unsorted)[0, 1, -1];
print "Sort assignment: smallest=$smallest, second_smallest=$second_smallest, largest=$largest\n";

# List assignment with splice
my @splicable = (10, 20, 30, 40, 50);
my ($spliced1, $spliced2, @spliced_rest) = splice(@splicable, 1, 3);
print "Splice assignment: spliced1=$spliced1, spliced2=$spliced2, rest=@spliced_rest\n";
print "Remaining array: @splicable\n";

# List assignment with split
my $csv_data = "apple,banana,cherry,date";
my ($fruit1, $fruit2, $fruit3, $fruit4) = split /,/, $csv_data;
print "Split assignment: fruit1=$fruit1, fruit2=$fruit2, fruit3=$fruit3, fruit4=$fruit4\n\n";

# Test 7: Performance-optimized patterns
print "=== Performance-Optimized Patterns ===\n";

# Benchmark different assignment patterns
sub benchmark_assignments {
    my ($iterations) = @_;
    $iterations ||= 50000;
    
    my @test_data = (1..100);
    
    # Pattern 1: Individual assignments
    my $start = time();
    for (1..$iterations) {
        my $a = $test_data[0];
        my $b = $test_data[1];
        my $c = $test_data[2];
        my $d = $test_data[3];
        my $e = $test_data[4];
    }
    my $individual_time = time() - $start;
    
    # Pattern 2: List assignment
    $start = time();
    for (1..$iterations) {
        my ($a, $b, $c, $d, $e) = @test_data[0..4];
    }
    my $list_time = time() - $start;
    
    # Pattern 3: Array slice
    $start = time();
    for (1..$iterations) {
        my @slice = @test_data[0..4];
    }
    my $slice_time = time() - $start;
    
    print "Assignment benchmark ($iterations iterations):\n";
    print "  Individual assignments: $individual_time seconds\n";
    print "  List assignment: $list_time seconds\n";
    print "  Array slice: $slice_time seconds\n";
    print "  Performance ratios:\n";
    print "    List/Individual: " . sprintf('%.2f', $list_time / $individual_time) . "x\n";
    print "    Slice/Individual: " . sprintf('%.2f', $slice_time / $individual_time) . "x\n";
}

benchmark_assignments(100000);

# Memory-efficient large list processing
sub process_large_list {
    my ($large_list) = @_;
    
    # Process in chunks to avoid memory issues
    my $chunk_size = 1000;
    my $total_processed = 0;
    
    for (my $i = 0; $i < @$large_list; $i += $chunk_size) {
        my $end = $i + $chunk_size - 1;
        $end = $#$large_list if $end > $#$large_list;
        
        my @chunk = @$large_list[$i..$end];
        
        # Process chunk
        my ($chunk_sum, $chunk_count) = (0, 0);
        for my $item (@chunk) {
            $chunk_sum += $item;
            $chunk_count++;
        }
        
        $total_processed += $chunk_count;
        
        # Clear chunk to free memory
        @chunk = ();
    }
    
    return $total_processed;
}

my @large_data = (1..10000);
my $processed_count = process_large_list(\@large_data);
print "Processed $processed_count items from large list\n\n";

# Test 8: Real-world data processing patterns
print "=== Real-World Data Processing Patterns ===\n";

# Pattern 1: CSV parsing with list assignment
sub parse_csv_line {
    my ($line) = @_;
    my @fields = split /,/, $line;
    
    # Trim whitespace from each field
    s/^\s+|\s+$//g for @fields;
    
    return @fields;
}

my $csv_line = "  John Doe , 35 , New York , Engineer , 75000  ";
my ($csv_name, $csv_age, $csv_city, $csv_job, $csv_salary) = parse_csv_line($csv_line);
print "CSV parsed: name='$csv_name', age=$csv_age, city='$csv_city', job='$csv_job', salary=$csv_salary\n";

# Pattern 2: Configuration file parsing
sub parse_config_line {
    my ($line) = @_;
    
    # Skip comments and empty lines
    return () if $line =~ /^\s*#/ || $line =~ /^\s*$/;
    
    # Parse key=value pairs
    if ($line =~ /^\s*(\w+)\s*=\s*(.+?)\s*$/) {
        my ($key, $value) = ($1, $2);
        
        # Remove quotes if present
        $value =~ s/^["']|["']$//g;
        
        return ($key, $value);
    }
    
    return ();
}

my @config_lines = (
    "debug = true",
    "timeout = 30",
    "database = 'myapp'",
    "# This is a comment",
    "",
    "max_connections = 100"
);

my %config;
for my $config_line (@config_lines) {
    my ($key, $value) = parse_config_line($config_line);
    $config{$key} = $value if $key;
}

print "Config parsed: " . join(', ', map { "$_=$config{$_}" } sort keys %config) . "\n";

# Pattern 3: Log file analysis
sub analyze_log_entry {
    my ($log_line) = @_;
    
    # Parse log format: [timestamp] level message
    if ($log_line =~ /^\[([^\]]+)\]\s+(\w+)\s+(.+)$/) {
        my ($timestamp, $level, $message) = ($1, $2, $3);
        
        # Extract additional info from message if present
        my ($user, $action) = ('unknown', 'unknown');
        if ($message =~ /user=(\w+)\s+action=(\w+)/) {
            ($user, $action) = ($1, $2);
        }
        
        return ($timestamp, $level, $message, $user, $action);
    }
    
    return ();
}

my @log_entries = (
    "[2023-12-01 10:00:00] INFO User login user=john action=login",
    "[2023-12-01 10:01:00] ERROR Database connection failed",
    "[2023-12-01 10:02:00] INFO User logout user=john action=logout",
    "[2023-12-01 10:03:00] WARN Memory usage high"
);

my @log_analysis;
for my $entry (@log_entries) {
    my ($timestamp, $level, $message, $user, $action) = analyze_log_entry($entry);
    push @log_analysis, {
        timestamp => $timestamp,
        level => $level,
        message => $message,
        user => $user,
        action => $action
    } if $timestamp;
}

print "Log analysis: " . scalar(@log_analysis) . " entries processed\n";
for my $analysis (@log_analysis[0..2]) {
    print "  $analysis->{timestamp} [$analysis->{level}] user=$analysis->{user} action=$analysis->{action}\n";
}

# Pattern 4: Data transformation pipeline
sub transform_data_pipeline {
    my ($raw_data) = @_;
    
    my @transformed;
    
    for my $item (@$raw_data) {
        # Extract and transform components
        my ($id, $name, $score, $category) = @$item;
        
        # Apply transformations
        $name = uc($name);
        $score = sprintf('%.2f', $score * 1.1);  # 10% bonus
        $category = uc($category);
        
        # Create new structure
        my @transformed_item = ($id, $name, $score, $category, time());
        push @transformed, \@transformed_item;
    }
    
    return @transformed;
}

my @raw_items = (
    [1, 'alice', 85.5, 'student'],
    [2, 'bob', 92.3, 'teacher'],
    [3, 'charlie', 78.9, 'student']
);

my @transformed_items = transform_data_pipeline(\@raw_items);
print "Data transformation: " . scalar(@transformed_items) . " items processed\n";
for my $item (@transformed_items) {
    my ($id, $name, $score, $category, $timestamp) = @$item;
    print "  ID:$id Name:$name Score:$score Category:$category Time:$timestamp\n";
}

print "\n=== Enhanced VariableListDeclaration Production Tests Completed ===\n";
print "This file demonstrates comprehensive variable list declaration patterns\n";
print "for production Perl applications with performance considerations.\n";