// Debug test to see what arguments are passed by cargo
use std::env;

#[test]
fn print_args() {
    eprintln!("=== ARGS FROM CARGO ===");
    for (i, arg) in env::args().enumerate() {
        eprintln!("ARG[{}]: {:?}", i, arg);
    }
    eprintln!("=======================");
}