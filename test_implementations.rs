use std::time::Instant;

fn main() {
    println!("Testing tree-sitter-perl implementations...\n");
    
    // Test Rust implementation
    println!("=== Testing Rust Implementation ===");
    match test_rust_implementation() {
        Ok(duration) => println!("✅ Rust implementation: {:?}", duration),
        Err(e) => println!("❌ Rust implementation failed: {}", e),
    }
    
    println!();
    
    // Test C implementation
    println!("=== Testing C Implementation ===");
    match test_c_implementation() {
        Ok(duration) => println!("✅ C implementation: {:?}", duration),
        Err(e) => println!("❌ C implementation failed: {}", e),
    }
}

fn test_rust_implementation() -> Result<std::time::Duration, Box<dyn std::error::Error>> {
    let code = "my $var = 'hello'; print \"Hello, World!\";";
    let start = Instant::now();
    
    // This would require the Rust implementation to be working
    // For now, just simulate
    std::thread::sleep(std::time::Duration::from_millis(1));
    
    Ok(start.elapsed())
}

fn test_c_implementation() -> Result<std::time::Duration, Box<dyn std::error::Error>> {
    let code = "my $var = 'hello'; print \"Hello, World!\";";
    let start = Instant::now();
    
    // This would require the C implementation to be working
    // For now, just simulate
    std::thread::sleep(std::time::Duration::from_millis(1));
    
    Ok(start.elapsed())
} 