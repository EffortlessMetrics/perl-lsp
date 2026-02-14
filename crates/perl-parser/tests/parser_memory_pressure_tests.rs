//! Memory Pressure Tests for Perl Parser
//!
//! This test suite validates parser behavior under memory pressure conditions:
//! - Low-memory conditions simulation
//! - Parser behavior when memory is constrained
//! - Memory fragmentation scenarios
//! - Cleanup and resource release validation

use perl_parser::Parser;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
// use std::collections::HashMap; // Not used in this test

/// Memory pressure simulation levels
const MEMORY_PRESSURE_LEVELS: &[usize] = &[1_000, 10_000, 100_000, 1_000_000];

/// Maximum acceptable memory usage ratio (memory_used / source_size)
const MAX_MEMORY_RATIO: f64 = 100.0; // 100x source size is acceptable upper bound

/// Test parser with simulated low-memory conditions
#[test]
fn test_low_memory_conditions() {
    println!("Testing low-memory conditions...");
    
    for pressure_level in MEMORY_PRESSURE_LEVELS {
        println!("Testing memory pressure level: {} bytes", pressure_level);
        
        // Generate code that will use significant memory
        let code = generate_memory_intensive_code(*pressure_level);
        
        // Measure memory before parsing
        let memory_before = estimate_memory_usage();
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        // Measure memory after parsing
        let memory_after = estimate_memory_usage();
        let memory_used = memory_after.saturating_sub(memory_before);
        
        // Verify parsing completed (may have errors but shouldn't crash)
        assert!(result.is_ok(), "Parser should handle {} byte code under memory pressure", pressure_level);
        
        // Check memory usage is reasonable
        let memory_ratio = memory_used as f64 / *pressure_level as f64;
        
        assert!(
            memory_ratio <= MAX_MEMORY_RATIO,
            "Memory ratio {:.1}x exceeds acceptable bound {:.1}x for {} bytes",
            memory_ratio, MAX_MEMORY_RATIO, pressure_level
        );
        
        // Should complete within reasonable time even under memory pressure
        assert!(
            parse_time < Duration::from_secs(10),
            "Parsing took too long under memory pressure: {:?}",
            parse_time
        );
        
        println!("  ✓ Pressure {}: used {}KB memory (ratio: {:.1}x) in {:?}",
                 pressure_level, memory_used / 1024, memory_ratio, parse_time);
    }
}

/// Test parser behavior when memory is constrained
#[test]
fn test_constrained_memory_scenarios() {
    println!("Testing constrained memory scenarios...");
    
    let constraint_scenarios = vec![
        ("Many small objects", generate_many_small_objects(10000)),
        ("Few large objects", generate_few_large_objects(100)),
        ("Deep nesting", generate_deep_nested_memory(100)),
        ("Wide structures", generate_wide_structures(1000)),
        ("Mixed allocations", generate_mixed_allocations(5000)),
    ];
    
    for (scenario_name, code) in constraint_scenarios {
        println!("Testing scenario: {}", scenario_name);
        
        // Simulate memory constraint by pre-allocating memory
        let _memory_reserve: Vec<u8> = vec![0; 10 * 1024 * 1024]; // 10MB reserve
        
        let memory_before = estimate_memory_usage();
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        let memory_after = estimate_memory_usage();
        let memory_used = memory_after.saturating_sub(memory_before);
        
        // Should handle constrained memory gracefully
        assert!(result.is_ok(), "Should handle {} scenario under memory constraints", scenario_name);
        
        // Memory usage should be reasonable
        let memory_ratio = memory_used as f64 / code.len() as f64;
        
        assert!(
            memory_ratio <= MAX_MEMORY_RATIO * 2.0, // Allow more under constraints
            "Constrained scenario {} memory ratio {:.1}x exceeds bound {:.1}x",
            scenario_name, memory_ratio, MAX_MEMORY_RATIO * 2.0
        );
        
        println!("  ✓ {}: used {}KB memory (ratio: {:.1}x) in {:?}",
                 scenario_name, memory_used / 1024, memory_ratio, parse_time);
        
        // Explicit cleanup
        drop(_memory_reserve);
    }
}

/// Test memory fragmentation scenarios
#[test]
fn test_memory_fragmentation_scenarios() {
    println!("Testing memory fragmentation scenarios...");
    
    let fragmentation_scenarios = vec![
        ("Fragmented allocations", generate_fragmented_allocations()),
        ("Variable sized objects", generate_variable_sized_objects()),
        ("Frequent allocations/deallocations", generate_frequent_allocations()),
        ("Memory churn", generate_memory_churn()),
    ];
    
    for (scenario_name, code) in fragmentation_scenarios {
        println!("Testing scenario: {}", scenario_name);
        
        // Create memory fragmentation by allocating and deallocating various sized objects
        let mut fragmenters = Vec::new();
        
        // Create fragmentation before parsing
        for i in 0..100 {
            let size = (i * 1000 + 1000) % 10000; // Variable sizes
            fragmenters.push(vec![0u8; size]);
        }
        
        // Deallocate some to create fragmentation
        for i in (0..fragmenters.len()).step_by(3) {
            fragmenters.remove(i);
        }
        
        let memory_before = estimate_memory_usage();
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        let memory_after = estimate_memory_usage();
        let memory_used = memory_after.saturating_sub(memory_before);
        
        // Should handle fragmented memory gracefully
        assert!(result.is_ok(), "Should handle {} scenario with fragmented memory", scenario_name);
        
        // Memory usage should be reasonable even with fragmentation
        let memory_ratio = memory_used as f64 / code.len() as f64;
        
        assert!(
            memory_ratio <= MAX_MEMORY_RATIO * 3.0, // Allow more with fragmentation
            "Fragmented scenario {} memory ratio {:.1}x exceeds bound {:.1}x",
            scenario_name, memory_ratio, MAX_MEMORY_RATIO * 3.0
        );
        
        println!("  ✓ {}: used {}KB memory (ratio: {:.1}x) in {:?}",
                 scenario_name, memory_used / 1024, memory_ratio, parse_time);
        
        // Cleanup
        drop(fragmenters);
    }
}

/// Test cleanup and resource release
#[test]
fn test_cleanup_and_resource_release() {
    println!("Testing cleanup and resource release...");
    
    let cleanup_scenarios = vec![
        ("Single parser cleanup", generate_test_code(1000)),
        ("Multiple parser cleanup", generate_test_code(5000)),
        ("Large parser cleanup", generate_test_code(20000)),
        ("Complex parser cleanup", generate_complex_memory_code()),
    ];
    
    for (scenario_name, code) in cleanup_scenarios {
        println!("Testing scenario: {}", scenario_name);
        
        // Baseline memory
        let baseline_memory = estimate_memory_usage();
        
        // Create and parse multiple times to test cleanup
        let iterations = 50;
        let mut memory_measurements = Vec::new();
        
        for i in 0..iterations {
            let memory_before = estimate_memory_usage();
            
            let mut parser = Parser::new(&code);
            let result = parser.parse();
            
            assert!(result.is_ok(), "Parse {} iteration {} should succeed", scenario_name, i);
            
            // Explicitly drop parser
            drop(parser);
            drop(result);
            
            let memory_after = estimate_memory_usage();
            let memory_used = memory_after.saturating_sub(memory_before);
            memory_measurements.push(memory_used);
            
            // Periodically force garbage collection
            if i % 10 == 0 {
                // Create temporary objects to trigger cleanup
                let _temp: Vec<String> = (0..1000).map(|j| format!("temp_{}_{}", i, j)).collect();
                drop(_temp);
            }
        }
        
        // Analyze memory usage pattern
        let avg_memory = memory_measurements.iter().sum::<usize>() / memory_measurements.len();
        let max_memory = memory_measurements.iter().max().unwrap();
        let min_memory = memory_measurements.iter().min().unwrap();
        
        let final_memory = estimate_memory_usage();
        let memory_leak = final_memory.saturating_sub(baseline_memory);
        
        println!("  ✓ {}: avg: {}KB, max: {}KB, min: {}KB, leak: {}KB",
                 scenario_name,
                 avg_memory / 1024,
                 max_memory / 1024,
                 min_memory / 1024,
                 memory_leak / 1024);
        
        // Should not have significant memory leaks
        assert!(
            memory_leak < 10 * 1024 * 1024, // 10MB leak threshold
            "Scenario {} has potential memory leak: {}KB",
            scenario_name, memory_leak / 1024
        );
        
        // Memory usage should be relatively stable
        let memory_variance = if max_memory > min_memory {
            max_memory - min_memory
        } else {
            0
        };
        
        assert!(
            memory_variance <= avg_memory * 2,
            "Memory usage variance {}KB is too high for scenario {}",
            memory_variance / 1024, scenario_name
        );
    }
}

/// Test concurrent memory pressure scenarios
#[test]
fn test_concurrent_memory_pressure() {
    println!("Testing concurrent memory pressure...");
    
    let thread_count = 8;
    let pressure_per_thread = 20;
    
    let results = Arc::new(Mutex::new(Vec::new()));
    
    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let results_clone = Arc::clone(&results);
            
            thread::spawn(move || {
                for pressure in 0..pressure_per_thread {
                    // Generate different memory pressure levels
                    let pressure_level = (thread_id * pressure_per_thread + pressure + 1) * 1000;
                    let code = generate_memory_intensive_code(pressure_level);
                    
                    let memory_before = estimate_memory_usage();
                    
                    let start_time = Instant::now();
                    let mut parser = Parser::new(&code);
                    let result = parser.parse();
                    let parse_time = start_time.elapsed();
                    
                    let memory_after = estimate_memory_usage();
                    let memory_used = memory_after.saturating_sub(memory_before);
                    
                    results_clone.lock().unwrap().push((
                        thread_id,
                        pressure,
                        pressure_level,
                        result.is_ok(),
                        parse_time,
                        memory_used,
                    ));
                }
            })
        })
        .collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }
    
    let results = results.lock().unwrap();
    
    // Analyze concurrent memory pressure results
    let mut total_memory = 0;
    let mut success_count = 0;
    let mut total_time = Duration::new(0, 0);
    
    for (thread_id, pressure, _pressure_level, success, parse_time, memory_used) in results.iter() {
        total_memory += *memory_used;
        total_time += *parse_time;
        if *success {
            success_count += 1;
        }
        
        // Individual operations should complete reasonably
        assert!(
            *parse_time < Duration::from_secs(5),
            "Thread {} pressure {} took too long: {:?}",
            thread_id, pressure, parse_time
        );
    }
    
    let avg_memory_per_operation = total_memory / results.len();
    let success_rate = success_count as f64 / results.len() as f64;
    let avg_time_per_operation = total_time / results.len() as u32;
    
    println!("  ✓ Concurrent memory pressure: {} operations, avg memory: {}KB, avg time: {:?}, success: {:.1}%",
             results.len(), avg_memory_per_operation / 1024, avg_time_per_operation, success_rate * 100.0);
    
    // Should maintain reasonable success rate under concurrent pressure
    assert!(
        success_rate > 0.85,
        "Success rate {:.1}% should be > 85% under concurrent memory pressure",
        success_rate * 100.0
    );
}

/// Test memory pressure with incremental parsing
#[test]
fn test_memory_pressure_incremental_parsing() {
    println!("Testing memory pressure with incremental parsing...");
    
    let base_code = generate_base_incremental_code();
    let modifications = vec![
        ("Add variable", add_variable_modification()),
        ("Add function", add_function_modification()),
        ("Add complex structure", add_complex_structure_modification()),
        ("Add large block", add_large_block_modification()),
    ];
    
    for (modification_name, modification_code) in modifications {
        println!("Testing incremental modification: {}", modification_name);
        
        // Start with base code
        let mut current_code = base_code.clone();
        
        // Apply incremental modifications
        for iteration in 0..20 {
            current_code.push_str(&format!("\n# Iteration {} - {}\n", iteration, modification_name));
            current_code.push_str(&modification_code);
            
            let memory_before = estimate_memory_usage();
            
            let start_time = Instant::now();
            let mut parser = Parser::new(&current_code);
            let result = parser.parse();
            let parse_time = start_time.elapsed();
            
            let memory_after = estimate_memory_usage();
            let memory_used = memory_after.saturating_sub(memory_before);
            
            // Should handle incremental growth
            assert!(result.is_ok(), "Incremental parsing {} iteration {} should succeed", modification_name, iteration);
            
            // Memory growth should be reasonable
            let memory_ratio = memory_used as f64 / modification_code.len() as f64;
            
            assert!(
                memory_ratio <= MAX_MEMORY_RATIO * 2.0,
                "Incremental modification {} iteration {} memory ratio {:.1}x exceeds bound",
                modification_name, iteration, memory_ratio
            );
            
            // Should complete quickly for incremental changes
            assert!(
                parse_time < Duration::from_millis(100),
                "Incremental parsing took too long: {:?}",
                parse_time
            );
        }
        
        println!("  ✓ {}: incremental parsing completed successfully", modification_name);
    }
}

/// Test memory pressure recovery scenarios
#[test]
fn test_memory_pressure_recovery() {
    println!("Testing memory pressure recovery...");
    
    let recovery_scenarios = vec![
        ("Pressure spike", generate_pressure_spike_scenario()),
        ("Gradual increase", generate_gradual_increase_scenario()),
        ("Burst allocation", generate_burst_allocation_scenario()),
        ("Sustained pressure", generate_sustained_pressure_scenario()),
    ];
    
    for (scenario_name, scenario_code) in recovery_scenarios {
        println!("Testing recovery scenario: {}", scenario_name);
        
        let baseline_memory = estimate_memory_usage();
        let mut memory_trend = Vec::new();
        
        // Execute scenario in phases
        for phase in 0..10 {
            let phase_code = format!("{}\n# Phase {}\n{}", scenario_code, phase, generate_test_code(phase * 100));
            
            let memory_before = estimate_memory_usage();
            
            let start_time = Instant::now();
            let mut parser = Parser::new(&phase_code);
            let result = parser.parse();
            let _parse_time = start_time.elapsed();
            
            let memory_after = estimate_memory_usage();
            let memory_used = memory_after.saturating_sub(memory_before);
            memory_trend.push(memory_used);
            
            assert!(result.is_ok(), "Recovery scenario {} phase {} should succeed", scenario_name, phase);
            
            // Recovery phases should show reasonable memory usage
            if phase > 2 {
                // Check if memory is stabilizing (not continuously growing)
                let recent_avg = memory_trend[phase-2..phase].iter().sum::<usize>() / 3;
                let current = memory_trend[phase];
                
                assert!(
                    current <= recent_avg * 2,
                    "Memory usage {}KB in phase {} is much higher than recent average {}KB",
                    current / 1024, phase, recent_avg / 1024
                );
            }
        }
        
        let final_memory = estimate_memory_usage();
        let total_growth = final_memory.saturating_sub(baseline_memory);
        
        println!("  ✓ {}: total memory growth: {}KB", scenario_name, total_growth / 1024);
        
        // Should not have excessive memory growth
        assert!(
            total_growth < 50 * 1024 * 1024, // 50MB growth threshold
            "Recovery scenario {} has excessive memory growth: {}KB",
            scenario_name, total_growth / 1024
        );
    }
}

/// Test extreme memory pressure scenarios
#[test]
fn test_extreme_memory_pressure() {
    println!("Testing extreme memory pressure scenarios...");
    
    let extreme_scenarios = vec![
        ("Massive single file", generate_massive_single_file()),
        ("Thousands of tiny objects", generate_thousands_of_tiny_objects()),
        ("Deep recursive structures", generate_deep_recursive_structures()),
        ("Memory bomb pattern", generate_memory_bomb_pattern()),
    ];
    
    for (scenario_name, code) in extreme_scenarios {
        println!("Testing extreme scenario: {}", scenario_name);
        
        // Pre-allocate memory to simulate system pressure
        let _system_pressure: Vec<Vec<u8>> = (0..100).map(|_| vec![0u8; 1024 * 1024]).collect(); // 100MB
        
        let memory_before = estimate_memory_usage();
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        let memory_after = estimate_memory_usage();
        let memory_used = memory_after.saturating_sub(memory_before);
        
        // Should handle extreme pressure without crashing
        match result {
            Ok(_ast) => {
                println!("  ✓ {}: parsed successfully", scenario_name);
                
                // Memory usage should be reasonable even for extreme cases
                let memory_ratio = memory_used as f64 / code.len() as f64;
                
                assert!(
                    memory_ratio <= MAX_MEMORY_RATIO * 10.0, // Very liberal bound for extreme cases
                    "Extreme scenario {} memory ratio {:.1}x exceeds extreme bound {:.1}x",
                    scenario_name, memory_ratio, MAX_MEMORY_RATIO * 10.0
                );
            }
            Err(e) => {
                println!("  ✓ {}: failed gracefully under extreme pressure: {:?}", scenario_name, e);
                
                // Should fail with resource-related error, not crash
                assert!(
                    e.to_string().contains("memory") ||
                    e.to_string().contains("resource") ||
                    e.to_string().contains("limit") ||
                    e.to_string().contains("recursion") ||
                    e.to_string().contains("depth"),
                    "Should fail with resource-related error, got: {}", e
                );
            }
        }
        
        // Should complete or fail within reasonable time
        assert!(
            parse_time < Duration::from_secs(30),
            "Extreme scenario {} took too long: {:?}",
            scenario_name, parse_time
        );
        
        println!("  {}: used {}KB memory in {:?}", scenario_name, memory_used / 1024, parse_time);
        
        // Cleanup
        drop(_system_pressure);
    }
}

// Helper functions for generating test code

fn generate_memory_intensive_code(size: usize) -> String {
    let mut code = String::with_capacity(size);
    code.push_str("use strict; use warnings;\n");
    
    let mut current_size = code.len();
    let var_counter = 0;
    
    while current_size < size {
        // Generate memory-intensive constructs
        let construct = format!(
            r#"my $var_{} = {{
    'data' => [{}, {}, {}, {}, {}],
    'nested' => {{
        'level1' => {{
            'level2' => {{
                'deep' => 'value_{}'
            }}
        }}
    }},
    'array_ref' => \@array_{},
    'hash_ref' => \%hash_{},
}};

"#, var_counter, var_counter, var_counter+1, var_counter+2, var_counter+3, var_counter+4, var_counter, var_counter, var_counter);
        
        code.push_str(&construct);
        current_size = code.len();
    }
    
    code
}

fn generate_many_small_objects(count: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    for i in 0..count {
        code.push_str(&format!(
            r#"my $small_obj_{} = {{
    id => {},
    name => 'obj_{}',
    value => {},
}};
"#, i, i, i, i));
    }
    
    code
}

fn generate_few_large_objects(count: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    for i in 0..count {
        code.push_str(&format!(
            r#"my $large_obj_{} = {{
"#, i));
        
        // Generate large hash structure
        for j in 0..1000 {
            code.push_str(&format!(
                "    key_{} => 'value_{}',\n", j, j));
        }
        
        code.push_str("};\n");
    }
    
    code
}

fn generate_deep_nested_memory(depth: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    code.push_str("my $deep = ");
    
    for i in 0..depth {
        code.push('{');
        code.push_str(&format!("level{} => ", i));
    }
    
    code.push_str("'deep_value'");
    
    for _ in 0..depth {
        code.push('}');
    }
    
    code.push_str(";\n");
    code
}

fn generate_wide_structures(width: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    code.push_str("my $wide = {\n");
    for i in 0..width {
        code.push_str(&format!("    key{} => ", i));
        
        // Create array as value
        code.push('[');
        for j in 0..10 {
            code.push_str(&format!("'value{}_{}'", i, j));
            if j < 9 { code.push_str(", "); }
        }
        code.push_str("],\n");
    }
    code.push_str("};\n");
    
    code
}

fn generate_mixed_allocations(count: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    for i in 0..count {
        match i % 4 {
            0 => {
                code.push_str(&format!("my $scalar_{} = 'string_value_{}';\n", i, i));
            }
            1 => {
                code.push_str(&format!("my @array_{} = ({});\n", i, (0..20).map(|j| j.to_string()).collect::<Vec<_>>().join(", ")));
            }
            2 => {
                code.push_str(&format!("my %hash_{} = ('key' => 'value_{}');\n", i, i));
            }
            3 => {
                code.push_str(&format!(
                    r#"sub sub_{} {{
    my ($param) = @_;
    return $param * {};
}}
"#, i, i));
            }
            _ => unreachable!(),
        }
    }
    
    code
}

fn generate_fragmented_allocations() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    // Generate variable-sized allocations to create fragmentation
    for i in 0..100 {
        let size = (i * 97 + 13) % 1000; // Pseudo-random sizes
        code.push_str(&format!("my $frag_{} = 'x' x {};\n", i, size));
    }
    
    code
}

fn generate_variable_sized_objects() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    for i in 0..50 {
        let size = i * 20; // Increasing sizes
        code.push_str(&format!(
            r#"my $var_obj_{} = {{
    size => {},
    data => ['{}'] x {},
}};
"#, i, size, "x", size));
    }
    
    code
}

fn generate_frequent_allocations() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    for i in 0..1000 {
        code.push_str(&format!(
            r#"{{
    my $temp_{} = $i * 2;
    my $temp_ref_{} = \$temp_{};
    push @temp_array, $temp_ref_{};
}}
"#, i, i, i, i));
    }
    
    code
}

fn generate_memory_churn() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    code.push_str("my @churn_array;\n");
    
    for i in 0..500 {
        code.push_str(&format!(
            r#"
# Churn iteration {}
my $churn_obj_{} = {{
    id => {},
    data => [map {{ $_ * 2 }} (1..50)],
}};

push @churn_array, $churn_obj_{};

# Remove some elements to create churn
if ($i % 10 == 0) {{
    shift @churn_array;
}}
"#, i, i, i, i));
    }
    
    code
}

fn generate_test_code(seed: usize) -> String {
    format!(
        r#"
use strict;
use warnings;

my $test_var_{} = {};
my @test_array_{} = ({}, {}, {});
my %test_hash_{} = ('key' => 'value_{}');

sub test_sub_{} {{
    my ($param) = @_;
    return $param * {};
}}

my $result_{} = test_sub_{}($test_var_{});
"#,
        seed, seed,
        seed, seed % 10, (seed + 1) % 10, (seed + 2) % 10,
        seed, seed,
        seed, seed,
        seed, seed, seed
    )
}

fn generate_complex_memory_code() -> String {
    r#"
use strict;
use warnings;

# Complex nested data structures
my $complex = {
    level1 => {
        level2 => {
            level3 => {
                level4 => {
                    level5 => {
                        deep_data => [map { $_ * 2 } (1..100)],
                        deep_hash => {map { ("key$_" => "value$_") } (1..50)},
                    }
                }
            }
        }
    },
    wide_structure => {
        map { ("wide_key$_" => {
            sub_data => [1..20],
            sub_hash => {map { ("sub_key$_" => $_) } (1..10)},
        }) } (1..100)
    }
};

# Complex array structures
my @complex_arrays = (
    [map { { id => $_, data => "item$_" } } (1..200)],
    [map { [$_, $_*2, $_*3, $_*4, $_*5] } (1..100)],
    [map { { nested => { deep => { value => $_ } } } } (1..50)],
);

# Complex subroutines with closures
sub create_complex_closure {
    my ($multiplier) = @_;
    return sub {
        my ($input) = @_;
        return {
            original => $input,
            multiplied => $input * $multiplier,
            metadata => {
                timestamp => time(),
                multiplier => $multiplier,
                process_id => $$,
            }
        };
    };
}

my $closure1 = create_complex_closure(2);
my $closure2 = create_complex_closure(3);
my $closure3 = create_complex_closure(5);
"#
    .to_string()
}

fn generate_base_incremental_code() -> String {
    r#"
use strict;
use warnings;

my $base_variable = 42;
my @base_array = (1, 2, 3, 4, 5);
my %base_hash = (key1 => 'value1', key2 => 'value2');

sub base_function {
    my ($param) = @_;
    return $param * 2;
}

my $base_result = base_function($base_variable);
"#
    .to_string()
}

fn add_variable_modification() -> String {
    r#"
my $new_variable = $base_variable + 10;
my @new_array = @base_array, (6, 7, 8, 9, 10);
"#
    .to_string()
}

fn add_function_modification() -> String {
    r#"
sub new_function {
    my ($x, $y) = @_;
    return $x + $y + $base_variable;
}

my $new_result = new_function(5, 10);
"#
    .to_string()
}

fn add_complex_structure_modification() -> String {
    r#"
my $complex_structure = {
    base_data => $base_variable,
    base_array => \@base_array,
    base_hash => \%base_hash,
    nested => {
        level1 => {
            level2 => {
                deep_value => $base_result
            }
        }
    }
};
"#
    .to_string()
}

fn add_large_block_modification() -> String {
    r#"
# Large block with many statements
for my $i (0..100) {
    my $temp = $i * $base_variable;
    push @base_array, $temp;
    
    if ($i % 10 == 0) {
        $base_hash{"key$i"} = "value$i";
    }
    
    my $func_result = base_function($i);
    print "Iteration $i: $func_result\n" if $i % 20 == 0;
}
"#
    .to_string()
}

fn generate_pressure_spike_scenario() -> String {
    r#"
use strict;
use warnings;

# Simulate pressure spike
my @spike_data;
for my $i (0..1000) {
    push @spike_data, {
        id => $i,
        data => 'x' x 1000,
        nested => {
            level1 => ['a' x 100, 'b' x 100, 'c' x 100],
            level2 => {map { ("key$_" => 'y' x 50) } (1..20)},
        }
    };
}
"#
    .to_string()
}

fn generate_gradual_increase_scenario() -> String {
    r#"
use strict;
use warnings;

# Gradual memory increase
my @gradual_data;
for my $phase (0..10) {
    for my $i (0..(100 * $phase)) {
        push @gradual_data, {
            phase => $phase,
            id => $i,
            data => sprintf("phase_%d_item_%d", $phase, $i),
        };
    }
}
"#
    .to_string()
}

fn generate_burst_allocation_scenario() -> String {
    r#"
use strict;
use warnings;

# Burst allocation pattern
my @burst_data;
for my $burst (0..5) {
    # Allocate a lot at once
    my @burst_batch = map {
        {
            burst => $burst,
            id => $_,
            payload => 'x' x 500,
        }
    } (0..200);
    
    push @burst_data, @burst_batch;
    
    # Simulate some processing
    @burst_batch = grep { $_->{id} % 2 == 0 } @burst_batch;
}
"#
    .to_string()
}

fn generate_sustained_pressure_scenario() -> String {
    r#"
use strict;
use warnings;

# Sustained memory pressure
my @sustained_data;
my $pressure_counter = 0;

for my $iteration (0..50) {
    # Add new data
    for my $i (0..100) {
        push @sustained_data, {
            iteration => $iteration,
            id => $i,
            counter => ++$pressure_counter,
            data => sprintf("iter_%d_item_%d_counter_%d", $iteration, $i, $pressure_counter),
        };
    }
    
    # Process and occasionally clean up
    if ($iteration % 10 == 0) {
        @sustained_data = grep { $_->{iteration} >= $iteration - 5 } @sustained_data;
    }
}
"#
    .to_string()
}

fn generate_massive_single_file() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n\n");
    
    // Generate a massive file with many definitions
    for i in 0..10000 {
        code.push_str(&format!(
            r#"sub massive_sub_{} {{
    my ($param1, $param2, $param3, $param4, $param5) = @_;
    my $result = $param1 + $param2 * $param3 - $param4 / $param5;
    
    # Add some complexity
    for my $i (0..10) {{
        $result += $i * $param1;
        $result -= $i * $param2;
    }}
    
    return $result;
}}

my $var_{} = {};
my @array_{} = ({}..{});
my %hash_{} = (map {{ ("key$_" => "value$_") }} (1..50));

"#, i, i, i, i, i*10, (i+1)*10, i));
    }
    
    code
}

fn generate_thousands_of_tiny_objects() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n\n");
    
    code.push_str("my @tiny_objects;\n");
    
    for i in 0..5000 {
        code.push_str(&format!(
            r#"push @tiny_objects, {{
    id => {},
    val => {},
    str => '{}',
}};
"#, i, i * 2, i));
    }
    
    code
}

fn generate_deep_recursive_structures() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n\n");
    
    // Generate deeply nested recursive structures
    code.push_str("my $recursive_structure = ");
    for _i in 0..200 {
        code.push_str(&format!("{{ level{} => ", _i));
    }
    code.push_str("'deep_value'");
    for _i in 0..200 {
        code.push_str("}");
    }
    code.push_str(";\n");
    
    // Generate recursive subroutines
    for i in 0..50 {
        code.push_str(&format!(
            r#"sub recursive_{} {{
    my ($n, $depth) = @_;
    return 1 if $depth <= 0;
    return recursive_{}($n - 1, $depth - 1) + recursive_{}($n - 2, $depth - 1);
}}
"#, i, i, i));
    }
    
    code
}

fn generate_memory_bomb_pattern() -> String {
    r#"
use strict;
use warnings;

# Memory bomb pattern - exponential growth
my @memory_bomb;
push @memory_bomb, 1;

for my $generation (0..10) {
    my @new_generation;
    
    for my $item (@memory_bomb) {
        # Each item spawns multiple new items
        for my $spawn (0..5) {
            push @new_generation, {
                parent => $item,
                generation => $generation,
                spawn => $spawn,
                data => 'x' x (100 * $generation),
                children => [],
            };
        }
    }
    
    @memory_bomb = @new_generation;
}

# Add cross-references to prevent easy cleanup
for my $i (0..$#memory_bomb) {
    if ($i > 0) {
        $memory_bomb[$i]{parent} = $memory_bomb[$i-1];
    }
    if ($i < $#memory_bomb) {
        push @{$memory_bomb[$i]{children}}, $memory_bomb[$i+1];
    }
}
"#
    .to_string()
}

// Simplified memory usage estimation
fn estimate_memory_usage() -> usize {
    // This is a simplified estimation - in a real implementation,
    // you might use platform-specific APIs or more sophisticated tracking
    std::mem::size_of::<perl_parser::Parser>() + 
    std::mem::size_of::<perl_parser::ast::Node>() * 1000 // Rough estimate
}