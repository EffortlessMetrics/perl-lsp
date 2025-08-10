// Test module to check lib test behavior
#[cfg(test)]
mod tests {
    #[test]
    fn lib_test_1() {
        assert_eq!(1 + 1, 2);
    }

    #[test]
    fn lib_test_2() {
        assert!(true);
    }
}