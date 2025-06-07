//! Dead code analysis and cleanup recommendations
//!
//! This analysis identifies code that should be removed rather than tested

/// Analysis of uncovered lines to determine if they represent:
/// 1. Dead code that should be removed
/// 2. Error paths that need better integration testing
/// 3. Edge cases that are difficult to trigger
pub fn analyze_uncovered_code() {
    // Based on the coverage report, here's what we should consider removing:

    println!("=== DEAD CODE ANALYSIS ===");

    // main.rs (0/19 lines covered)
    println!("main.rs: 100% uncovered - this is expected for CLI entry point");
    println!("  - Lines 12-15: Error handling in main() - hard to test, but necessary");
    println!("  - Lines 20-49: CLI flow - should be tested via subprocess, not unit tests");
    println!("  - Recommendation: Add CLI integration tests, not unit tests");

    // cli.rs (6/34 lines covered)
    println!("\ncli.rs: 82% uncovered - mostly error paths");
    println!("  - Lines 56-62: Invalid log level handling - should be tested");
    println!("  - Lines 64-66: Logging setup errors - edge case");
    println!("  - Lines 94-98: Render file validation - should be tested");
    println!("  - Lines 102-105: Aux file directory validation - already tested");
    println!("  - Lines 113-115: Incomplete validation - possibly dead code?");

    // render.rs (56/110 lines covered)
    println!("\nrender.rs: 49% uncovered - lots of error handling");
    println!("  - Lines 45-47: Directory creation errors - edge case");
    println!("  - Lines 163-171: SVG rendering errors - needs Typst installed");
    println!("  - Lines 224-240: PNG rendering - needs Typst installed");
    println!("  - Lines 314-362: Temp file creation/cleanup - mostly error paths");
    println!("  - Recommendation: Mock Typst binary or make rendering optional");

    // anki.rs (49/86 lines covered)
    println!("\nanki.rs: 43% uncovered - network error handling");
    println!("  - Lines 120, 123: Version check logging/errors - hard to mock");
    println!("  - Lines 173-196: Card creation error paths - needs AnkiConnect");
    println!("  - Lines 247, 280-307: Network timeouts and malformed responses");
    println!("  - Recommendation: Better error injection in integration tests");

    // Specific recommendations for code removal:
    println!("\n=== REMOVAL CANDIDATES ===");

    println!("1. Excessive error logging: Some error cases log AND return errors");
    println!("2. Duplicate validation: Some validations happen in multiple places");
    println!("3. Unused render formats: If only Plain is used, remove SVG/PNG/HTML");
    println!("4. Complex temp file management: Could be simplified");
    println!("5. Over-engineered CLI validation: Some edge cases may be unnecessary");
}

/// Recommendations for achieving 80% coverage without excessive unit tests
pub fn coverage_strategy() {
    println!("\n=== 80% COVERAGE STRATEGY ===");

    println!("Current: 57.06% (412/722 lines)");
    println!("Target:  80.00% (578/722 lines) - need +166 lines");

    println!("\nHigh-impact improvements:");
    println!("1. main.rs CLI integration tests: +19 lines");
    println!("2. anki.rs error injection tests: +20-30 lines");
    println!("3. render.rs format consolidation: +20-30 lines");
    println!("4. cli.rs validation coverage: +15-20 lines");
    println!("5. Better property-based testing: +30-50 lines");
    println!("6. Remove dead code: +30-40 lines");

    println!("\nTotal potential: 134-189 lines (would reach 76-83%)");

    println!("\nFocus areas:");
    println!("- CLI subprocess testing for main.rs");
    println!("- Network error simulation for anki.rs");
    println!("- File system error injection");
    println!("- Remove unused rendering formats");
    println!("- Simplify over-engineered validation");
}

#[cfg(test)]
mod analysis_tests {
    #[test]
    fn print_analysis() {
        super::analyze_uncovered_code();
        super::coverage_strategy();
    }
}
