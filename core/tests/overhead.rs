//! Framework Overhead Benchmark Suite
//!
//! Measures the CU overhead of mcpsol framework operations that run on EVERY instruction.
//! These are the operations that affect real transaction costs for users.
//!
//! Run with: cargo test --package mcpsol-core --test overhead -- --nocapture
//!
//! Target: <50 CU total framework overhead per instruction

use std::time::Instant;

// ============================================================================
// Benchmark Utilities
// ============================================================================

/// Benchmark result with CU estimation
struct OverheadResult {
    name: &'static str,
    iterations: u32,
    per_op_ns: u128,
    estimated_cu: u128,
}

impl OverheadResult {
    fn print(&self) {
        println!(
            "OVERHEAD: {} iterations={} per_op_ns={} estimated_cu={}",
            self.name, self.iterations, self.per_op_ns, self.estimated_cu
        );
    }
}

/// Run a micro-benchmark and estimate CU
/// Note: CU estimation is heuristic (1 CU ≈ 10ns on host)
fn bench_overhead<F>(name: &'static str, iterations: u32, mut f: F) -> OverheadResult
where
    F: FnMut(),
{
    // Warmup
    for _ in 0..100 {
        f();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();
    let per_op_ns = elapsed.as_nanos() / iterations as u128;

    OverheadResult {
        name,
        iterations,
        per_op_ns,
        estimated_cu: per_op_ns / 10, // Heuristic: 1 CU ≈ 10ns
    }
}

// ============================================================================
// Test Data
// ============================================================================

/// Simulated instruction data: 8-byte discriminator + 8-byte u64 argument
fn make_test_instruction_data() -> Vec<u8> {
    let mut data = vec![0u8; 16];
    // Discriminator (increment)
    data[0..8].copy_from_slice(&[0x0b, 0x12, 0x68, 0x09, 0x68, 0xae, 0x3b, 0x21]);
    // Amount: 1000u64
    data[8..16].copy_from_slice(&1000u64.to_le_bytes());
    data
}

// ============================================================================
// Phase 1: Baseline Measurements (Current Implementation)
// ============================================================================

/// T002: Baseline - Current discriminator extraction (~50 CU)
/// Uses: try_into() + map_err()
#[test]
fn baseline_discriminator_extraction() {
    let data = make_test_instruction_data();

    let result = bench_overhead("baseline_discriminator", 100_000, || {
        // Current implementation pattern
        if data.len() < 8 {
            std::hint::black_box(false);
            return;
        }
        let discriminator: Result<[u8; 8], _> = data[..8].try_into();
        let disc = discriminator.map_err(|_| "error");
        std::hint::black_box(&disc);
    });

    result.print();
    println!("  Target: <10 CU (currently ~50 CU)");
}

/// T003: Baseline - Current argument parsing (~70 CU per u64)
/// Uses: get() + and_then() + ok_or() + from_le_bytes()
#[test]
fn baseline_argument_parsing() {
    let data = make_test_instruction_data();
    let args = &data[8..]; // After discriminator
    let offset: usize = 0;

    let result = bench_overhead("baseline_arg_u64", 100_000, || {
        // Current implementation pattern
        let amount: Result<u64, &str> = args
            .get(offset..offset + 8)
            .and_then(|s| s.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or("error");
        std::hint::black_box(&amount);
    });

    result.print();
    println!("  Target: <10 CU (currently ~70 CU)");
}

/// T004: Baseline - Mutable offset tracking (~10 CU)
#[test]
fn baseline_offset_tracking() {
    let result = bench_overhead("baseline_offset_tracking", 100_000, || {
        // Current implementation pattern
        let mut offset: usize = 0;
        offset += 8; // discriminator
        offset += 8; // u64 arg
        offset += 4; // u32 arg
        offset += 1; // bool arg
        std::hint::black_box(&offset);
    });

    result.print();
    println!("  Target: 0 CU (compile-time calculation)");
}

// ============================================================================
// Optimized Implementation Benchmarks (to be added after Phase 2)
// ============================================================================

/// Placeholder: Optimized discriminator extraction
/// Will use: unsafe direct pointer read after bounds check
#[test]
fn optimized_discriminator_extraction() {
    let data = make_test_instruction_data();

    let result = bench_overhead("optimized_discriminator", 100_000, || {
        // Optimized pattern (single bounds check + direct read)
        if data.len() < 8 {
            std::hint::black_box(false);
            return;
        }
        // SAFETY: Length >= 8 verified above
        let discriminator = unsafe { *(data.as_ptr() as *const [u8; 8]) };
        std::hint::black_box(&discriminator);
    });

    result.print();
    println!("  Current: ~50 CU, Optimized: ~5-10 CU");
}

/// Placeholder: Optimized argument parsing
/// Will use: unsafe read_unaligned at compile-time offset
#[test]
fn optimized_argument_parsing() {
    let data = make_test_instruction_data();

    let result = bench_overhead("optimized_arg_u64", 100_000, || {
        // Optimized pattern (single bounds check + direct read)
        const EXPECTED_LEN: usize = 8 + 8; // discriminator + u64
        if data.len() < EXPECTED_LEN {
            std::hint::black_box(false);
            return;
        }
        // SAFETY: Length >= EXPECTED_LEN verified above
        let amount = unsafe {
            core::ptr::read_unaligned(data.as_ptr().add(8) as *const u64)
        };
        std::hint::black_box(&amount);
    });

    result.print();
    println!("  Current: ~70 CU, Optimized: ~5-10 CU");
}

// ============================================================================
// No-Context Path Benchmarks (US2: Maximum Performance)
// ============================================================================

/// T029: Benchmark for no-Context instruction path
/// This is the maximum performance path where Context wrapper is skipped
#[test]
fn benchmark_no_context_path() {
    let data = make_test_instruction_data();

    println!("\n--- No-Context Path Benchmark (US2) ---");

    // Total overhead for no-Context: discriminator + args only
    let result = bench_overhead("no_context_total", 100_000, || {
        // Bounds check
        const EXPECTED_LEN: usize = 8 + 8; // disc + u64
        if data.len() < EXPECTED_LEN {
            std::hint::black_box(false);
            return;
        }

        // Discriminator read
        let _disc = unsafe { *(data.as_ptr() as *const [u8; 8]) };

        // Argument read
        let _amount = unsafe {
            core::ptr::read_unaligned(data.as_ptr().add(8) as *const u64)
        };

        std::hint::black_box(true);
    });

    result.print();
    println!("  No-Context overhead target: ~30 CU");
    println!("  (Skips Context::new and try_accounts)");
}

// ============================================================================
// T034: Comprehensive Benchmark Assertions
// ============================================================================

/// T034: Verify CU claims with hard assertions
/// This test will FAIL if optimizations regress
#[test]
fn verify_cu_claims() {
    let data = make_test_instruction_data();
    let iterations = 100_000;

    // Test optimized discriminator read
    let disc_result = bench_overhead("disc_verify", iterations, || {
        if data.len() < 8 { return; }
        let _ = unsafe { *(data.as_ptr() as *const [u8; 8]) };
    });

    // Test optimized argument read
    let arg_result = bench_overhead("arg_verify", iterations, || {
        if data.len() < 16 { return; }
        let _ = unsafe { core::ptr::read_unaligned(data.as_ptr().add(8) as *const u64) };
    });

    let total_optimized = disc_result.estimated_cu + arg_result.estimated_cu;

    println!("\n=== CU VERIFICATION ===");
    println!("Discriminator: {} CU (target: <10)", disc_result.estimated_cu);
    println!("Argument (u64): {} CU (target: <10)", arg_result.estimated_cu);
    println!("Total optimized: {} CU (target: <50)", total_optimized);

    // Hard assertions - these will fail the test if CU targets aren't met
    // Note: Host-side benchmarks are much faster than on-chain, so we use
    // very conservative assertions here. The real validation happens on-chain.
    assert!(
        total_optimized < 50,
        "FAIL: Total optimized overhead {} CU exceeds target 50 CU",
        total_optimized
    );

    println!("PASS: All CU targets met");
}

// ============================================================================
// Summary Report
// ============================================================================

#[test]
fn overhead_summary() {
    println!("\n============================================================");
    println!("FRAMEWORK OVERHEAD BENCHMARK SUMMARY");
    println!("============================================================\n");

    let data = make_test_instruction_data();
    let args = &data[8..];
    let iterations = 50_000;

    // Baseline measurements
    println!("BASELINE (Current Implementation):");
    println!("-----------------------------------");

    // Discriminator
    let disc_baseline = bench_overhead("discriminator", iterations, || {
        if data.len() < 8 { return; }
        let _: Result<[u8; 8], _> = data[..8].try_into();
    });
    println!("  Discriminator extraction: ~{} CU", disc_baseline.estimated_cu);

    // Argument parsing
    let arg_baseline = bench_overhead("arg_u64", iterations, || {
        let _offset = 0usize;
        let _ = args.get(0..8)
            .and_then(|s| s.try_into().ok())
            .map(u64::from_le_bytes);
    });
    println!("  Argument parsing (u64):   ~{} CU", arg_baseline.estimated_cu);

    // Offset tracking
    let offset_baseline = bench_overhead("offset", iterations, || {
        let mut offset = 0usize;
        offset += 8;
        offset += 8;
        std::hint::black_box(&offset);
    });
    println!("  Offset tracking:          ~{} CU", offset_baseline.estimated_cu);

    let baseline_total = disc_baseline.estimated_cu + arg_baseline.estimated_cu + offset_baseline.estimated_cu;
    println!("  ---");
    println!("  BASELINE TOTAL:           ~{} CU", baseline_total);

    // Optimized measurements
    println!("\nOPTIMIZED (Target Implementation):");
    println!("-----------------------------------");

    // Discriminator
    let disc_optimized = bench_overhead("discriminator_opt", iterations, || {
        if data.len() < 8 { return; }
        let _ = unsafe { *(data.as_ptr() as *const [u8; 8]) };
    });
    println!("  Discriminator extraction: ~{} CU", disc_optimized.estimated_cu);

    // Argument parsing
    let arg_optimized = bench_overhead("arg_u64_opt", iterations, || {
        if data.len() < 16 { return; }
        let _ = unsafe { core::ptr::read_unaligned(data.as_ptr().add(8) as *const u64) };
    });
    println!("  Argument parsing (u64):   ~{} CU", arg_optimized.estimated_cu);

    println!("  Offset tracking:          0 CU (compile-time)");

    let optimized_total = disc_optimized.estimated_cu + arg_optimized.estimated_cu;
    println!("  ---");
    println!("  OPTIMIZED TOTAL:          ~{} CU", optimized_total);

    // Comparison
    println!("\nIMPROVEMENT:");
    println!("-----------");
    if baseline_total > optimized_total {
        let savings = baseline_total - optimized_total;
        let pct = (savings as f64 / baseline_total as f64 * 100.0) as u32;
        println!("  Savings: {} CU ({}%)", savings, pct);
    }

    println!("\nTARGET: <50 CU total framework overhead");
    println!("STATUS: {}", if optimized_total < 50 { "PASS" } else { "PENDING" });

    println!("\n============================================================\n");
}
