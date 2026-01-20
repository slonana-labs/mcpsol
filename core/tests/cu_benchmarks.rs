//! Comprehensive CU benchmark suite for mcpsol
//!
//! This suite provides complete CU measurement coverage for regression testing.
//! Run with: cargo test --package mcpsol-core --test cu_benchmarks -- --nocapture
//!
//! Output format is machine-parseable for CI integration.

use mcpsol_core::{
    ArgType, CachedSchemaPages, McpSchemaBuilder, McpToolBuilder,
    generate_compact_schema, generate_paginated_schema_bytes,
    estimate_schema_size, estimate_single_tool_size,
};
use std::time::Instant;

// ============================================================================
// Test Schemas
// ============================================================================

fn build_minimal_schema() -> mcpsol_core::McpSchema {
    McpSchemaBuilder::new("minimal")
        .add_tool(McpToolBuilder::new("list_tools").build())
        .build()
}

fn build_typical_schema() -> mcpsol_core::McpSchema {
    McpSchemaBuilder::new("counter")
        .add_tool(
            McpToolBuilder::new("list_tools")
                .description("List available tools")
                .build()
        )
        .add_tool(
            McpToolBuilder::new("initialize")
                .description("Initialize counter")
                .signer_writable("counter")
                .signer("authority")
                .build()
        )
        .add_tool(
            McpToolBuilder::new("increment")
                .description("Add to counter")
                .writable("counter")
                .signer("authority")
                .arg("amount", ArgType::U64)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("decrement")
                .description("Subtract from counter")
                .writable("counter")
                .signer("authority")
                .arg("amount", ArgType::U64)
                .build()
        )
        .build()
}

fn build_complex_schema() -> mcpsol_core::McpSchema {
    McpSchemaBuilder::new("defi_amm")
        .add_tool(
            McpToolBuilder::new("list_tools")
                .description("List available MCP tools")
                .build()
        )
        .add_tool(
            McpToolBuilder::new("initialize_pool")
                .description("Create new AMM pool")
                .signer_writable_desc("pool", "Pool account to create")
                .signer_desc("authority", "Pool authority")
                .account_with_desc("token_a_mint", "Token A mint", false, false)
                .account_with_desc("token_b_mint", "Token B mint", false, false)
                .account_with_desc("system_program", "System program", false, false)
                .arg_desc("fee_bps", "Fee in basis points", ArgType::U16)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("add_liquidity")
                .description("Add liquidity to pool")
                .writable_desc("pool", "Pool to add to")
                .signer_desc("provider", "Liquidity provider")
                .writable_desc("provider_token_a", "Provider token A")
                .writable_desc("provider_token_b", "Provider token B")
                .writable_desc("pool_token_a", "Pool token A reserve")
                .writable_desc("pool_token_b", "Pool token B reserve")
                .arg_desc("amount_a", "Amount of token A", ArgType::U64)
                .arg_desc("amount_b", "Amount of token B", ArgType::U64)
                .arg_desc("min_lp", "Minimum LP tokens", ArgType::U64)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("swap")
                .description("Swap tokens via AMM")
                .writable_desc("pool", "Pool to swap through")
                .signer_desc("user", "User performing swap")
                .writable_desc("user_token_in", "User input token")
                .writable_desc("user_token_out", "User output token")
                .arg_desc("amount_in", "Amount to swap", ArgType::U64)
                .arg_desc("min_out", "Minimum output", ArgType::U64)
                .build()
        )
        .build()
}

// ============================================================================
// Benchmark Utilities
// ============================================================================

struct BenchmarkResult {
    name: String,
    iterations: u32,
    total_ns: u128,
    per_op_ns: u128,
    estimated_cu: u128,
}

impl BenchmarkResult {
    fn print(&self) {
        println!("BENCHMARK: {} iterations={} total_ns={} per_op_ns={} estimated_cu={}",
            self.name, self.iterations, self.total_ns, self.per_op_ns, self.estimated_cu);
    }
}

fn benchmark<F>(name: &str, iterations: u32, mut f: F) -> BenchmarkResult
where
    F: FnMut(),
{
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();
    let total_ns = elapsed.as_nanos();
    let per_op_ns = total_ns / iterations as u128;

    BenchmarkResult {
        name: name.to_string(),
        iterations,
        total_ns,
        per_op_ns,
        estimated_cu: per_op_ns / 10, // Heuristic: 1 CU ≈ 10ns
    }
}

// ============================================================================
// Compact Schema Benchmarks
// ============================================================================

#[test]
fn bench_compact_minimal() {
    let schema = build_minimal_schema();
    let result = benchmark("compact_minimal", 10000, || {
        let json = generate_compact_schema(&schema);
        std::hint::black_box(&json);
    });
    result.print();

    let json = generate_compact_schema(&schema);
    println!("OUTPUT_SIZE: compact_minimal bytes={}", json.len());
}

#[test]
fn bench_compact_typical() {
    let schema = build_typical_schema();
    let result = benchmark("compact_typical", 10000, || {
        let json = generate_compact_schema(&schema);
        std::hint::black_box(&json);
    });
    result.print();

    let json = generate_compact_schema(&schema);
    println!("OUTPUT_SIZE: compact_typical bytes={}", json.len());
}

#[test]
fn bench_compact_complex() {
    let schema = build_complex_schema();
    let result = benchmark("compact_complex", 10000, || {
        let json = generate_compact_schema(&schema);
        std::hint::black_box(&json);
    });
    result.print();

    let json = generate_compact_schema(&schema);
    println!("OUTPUT_SIZE: compact_complex bytes={}", json.len());
}

// ============================================================================
// Paginated Schema Benchmarks
// ============================================================================

#[test]
fn bench_paginated_direct_typical() {
    let schema = build_typical_schema();
    let result = benchmark("paginated_direct_typical", 10000, || {
        for cursor in 0..schema.tools.len() {
            let bytes = generate_paginated_schema_bytes(&schema, cursor as u8);
            std::hint::black_box(&bytes);
        }
    });
    result.print();
}

#[test]
fn bench_paginated_cached_typical() {
    let schema = build_typical_schema();
    let cached = CachedSchemaPages::from_schema(&schema);
    let result = benchmark("paginated_cached_typical", 10000, || {
        for cursor in 0..cached.num_pages() {
            let bytes = cached.get_page(cursor as u8);
            std::hint::black_box(&bytes);
        }
    });
    result.print();
}

#[test]
fn bench_paginated_direct_complex() {
    let schema = build_complex_schema();
    let result = benchmark("paginated_direct_complex", 10000, || {
        for cursor in 0..schema.tools.len() {
            let bytes = generate_paginated_schema_bytes(&schema, cursor as u8);
            std::hint::black_box(&bytes);
        }
    });
    result.print();
}

#[test]
fn bench_paginated_cached_complex() {
    let schema = build_complex_schema();
    let cached = CachedSchemaPages::from_schema(&schema);
    let result = benchmark("paginated_cached_complex", 10000, || {
        for cursor in 0..cached.num_pages() {
            let bytes = cached.get_page(cursor as u8);
            std::hint::black_box(&bytes);
        }
    });
    result.print();
}

// ============================================================================
// Cache Initialization Benchmarks
// ============================================================================

#[test]
fn bench_cache_init_typical() {
    let result = benchmark("cache_init_typical", 1000, || {
        let schema = build_typical_schema();
        let cached = CachedSchemaPages::from_schema(&schema);
        std::hint::black_box(&cached);
    });
    result.print();
}

#[test]
fn bench_cache_init_complex() {
    let result = benchmark("cache_init_complex", 1000, || {
        let schema = build_complex_schema();
        let cached = CachedSchemaPages::from_schema(&schema);
        std::hint::black_box(&cached);
    });
    result.print();
}

// ============================================================================
// Size Estimation Benchmarks
// ============================================================================

#[test]
fn bench_estimate_schema_size() {
    let schema = build_complex_schema();
    let result = benchmark("estimate_schema_size", 100000, || {
        let size = estimate_schema_size(&schema);
        std::hint::black_box(&size);
    });
    result.print();

    let estimated = estimate_schema_size(&schema);
    let actual = generate_compact_schema(&schema).len();
    println!("SIZE_ACCURACY: estimated={} actual={} diff={}", estimated, actual, (estimated as i64 - actual as i64).abs());
}

#[test]
fn bench_estimate_single_tool() {
    let schema = build_complex_schema();
    let tool = schema.tools.get(2); // add_liquidity - most complex tool
    let result = benchmark("estimate_single_tool", 100000, || {
        let size = estimate_single_tool_size(tool);
        std::hint::black_box(&size);
    });
    result.print();
}

// ============================================================================
// Summary Report
// ============================================================================

#[test]
fn summary_report() {
    println!("\n============================================================");
    println!("CU BENCHMARK SUMMARY REPORT");
    println!("============================================================\n");

    // Compact schema
    let minimal = build_minimal_schema();
    let typical = build_typical_schema();
    let complex = build_complex_schema();

    println!("Schema Sizes (compact format):");
    println!("  Minimal (1 tool):  {} bytes", generate_compact_schema(&minimal).len());
    println!("  Typical (4 tools): {} bytes", generate_compact_schema(&typical).len());
    println!("  Complex (4 tools): {} bytes", generate_compact_schema(&complex).len());

    // Paginated sizes
    println!("\nPaginated Page Sizes (typical schema):");
    let cached_typical = CachedSchemaPages::from_schema(&typical);
    for i in 0..cached_typical.num_pages() {
        println!("  Page {}: {} bytes", i, cached_typical.get_page(i as u8).len());
    }

    // Cache performance comparison
    let iterations = 5000;

    // Direct
    let start = Instant::now();
    for _ in 0..iterations {
        for cursor in 0..typical.tools.len() {
            let bytes = generate_paginated_schema_bytes(&typical, cursor as u8);
            std::hint::black_box(&bytes);
        }
    }
    let direct_ns = start.elapsed().as_nanos() / (iterations as u128 * typical.tools.len() as u128);

    // Cached
    let cached = CachedSchemaPages::from_schema(&typical);
    let start = Instant::now();
    for _ in 0..iterations {
        for cursor in 0..cached.num_pages() {
            let bytes = cached.get_page(cursor as u8);
            std::hint::black_box(&bytes);
        }
    }
    let cached_ns = start.elapsed().as_nanos() / (iterations as u128 * cached.num_pages() as u128);

    let improvement = if direct_ns > cached_ns {
        ((direct_ns - cached_ns) as f64 / direct_ns as f64 * 100.0) as u32
    } else {
        0
    };

    println!("\nPerformance Comparison (per page, {} iterations):", iterations);
    println!("  Direct generation:  {} ns (~{} CU)", direct_ns, direct_ns / 10);
    println!("  Cached generation:  {} ns (~{} CU)", cached_ns, cached_ns / 10);
    println!("  Improvement:        {}%", improvement);

    println!("\n============================================================");
    println!("Run individual benchmarks for detailed results:");
    println!("  cargo test --package mcpsol-core --test cu_benchmarks -- --nocapture");
    println!("============================================================\n");
}
