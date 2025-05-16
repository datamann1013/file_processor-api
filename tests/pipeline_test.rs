// tests/pipeline_test.rs

#[cfg(test)]
mod pipeline_tests {
    /// This test is expected to pass.
    
    #[test]
    fn test_should_pass() {
        // Simple assertion that always succeeds
        assert_eq!(2 + 2, 4);
    }

    /// This test is intentionally written to fail,
    /// ensuring that your pipeline flags failures.
    #[test]
    fn test_should_fail() {
        // This assertion always fails
        assert!(false, "Intentional failure for pipeline test");
    }
}
