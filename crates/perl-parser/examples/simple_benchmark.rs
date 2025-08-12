//! Simple benchmark for the modern parser
use perl_parser::Parser;
use std::time::Instant;

fn main() {
    println!("=== Modern Parser Benchmark ===\n");

    let test_cases = vec![
        ("Simple", "$x = 42;"),
        ("Expression", "my $result = ($a + $b) * $c;"),
        (
            "Control Flow",
            r#"
if ($x > 10) {
    while ($y < 100) {
        $y = $y * 2;
    }
}"#,
        ),
        ("Method Call", "$obj->method($arg1, $arg2);"),
        ("For Loop", "for (my $i = 0; $i < 10; $i++) { print $i; }"),
        (
            "Complex",
            r#"
sub process_data {
    my $data = shift;
    my $result = 0;
    for (my $i = 0; $i < 10; $i++) {
        $result = $result + $data;
    }
    return $result;
}"#,
        ),
    ];

    const ITERATIONS: u32 = 1000;

    println!("Running {} iterations per test case...\n", ITERATIONS);
    println!(
        "{:<15} {:<10} {:<15} {:<15}",
        "Test Case", "Size", "Avg Time (µs)", "Throughput (MB/s)"
    );
    println!("{:-<55}", "");

    let mut total_time = 0.0;
    let mut total_bytes = 0;

    for (name, code) in &test_cases {
        let code_bytes = code.len();

        // Warm up
        for _ in 0..10 {
            let mut parser = Parser::new(code);
            let _ = parser.parse();
        }

        // Benchmark
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            assert!(result.is_ok());
        }
        let elapsed = start.elapsed();

        let avg_time_us = elapsed.as_secs_f64() * 1_000_000.0 / ITERATIONS as f64;
        let throughput_mbps =
            (code_bytes as f64 * ITERATIONS as f64) / elapsed.as_secs_f64() / 1_000_000.0;

        println!(
            "{:<15} {:<10} {:<15.2} {:<15.2}",
            name,
            format!("{} B", code_bytes),
            avg_time_us,
            throughput_mbps
        );

        total_time += avg_time_us;
        total_bytes += code_bytes;
    }

    println!("{:-<55}", "");
    let avg_time = total_time / test_cases.len() as f64;
    let avg_throughput = (total_bytes as f64 * ITERATIONS as f64 * test_cases.len() as f64)
        / (total_time * test_cases.len() as f64 / 1_000_000.0)
        / 1_000_000.0;

    println!(
        "{:<15} {:<10} {:<15.2} {:<15.2}",
        "Average",
        format!("{} B", total_bytes / test_cases.len()),
        avg_time,
        avg_throughput
    );

    println!("\n📊 Performance Summary:");
    println!("  • Average parse time: {:.2} µs", avg_time);
    println!(
        "  • Parse rate: ~{:.0} µs/KB",
        avg_time * 1000.0 / (total_bytes as f64 / test_cases.len() as f64)
    );
    println!("  • Throughput: {:.2} MB/s", avg_throughput);
}
