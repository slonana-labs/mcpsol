//! Baseline CU measurement tests for mcpsol
//!
//! These tests establish baseline measurements for `list_tools` CU consumption.
//! Run with: cargo test --package mcpsol-core --test baseline -- --nocapture
//!
//! Note: Actual CU measurement requires Solana program test infrastructure.
//! These tests measure the Rust-side overhead (allocations, serialization).

use std::time::Instant;

// Re-create the schema building logic to measure overhead
fn build_4_tool_schema() -> String {
    // Simulates the JSON generation for a typical 4-tool program
    let mut json = String::with_capacity(800);
    json.push_str(r#"{"v":"2024-11-05","name":"counter","tools":["#);

    // Tool 1: list_tools
    json.push_str(r#"{"n":"list_tools","d":"42195e6a55fd41c0"},"#);

    // Tool 2: initialize
    json.push_str(r#"{"n":"initialize","i":"Create counter","d":"afaf6d1f0d989bed","p":{"counter_sw":"pubkey","authority_s":"pubkey"},"r":["counter_sw","authority_s"]},"#);

    // Tool 3: increment
    json.push_str(r#"{"n":"increment","i":"Add to counter","d":"0b12680968ae3b21","p":{"counter_w":"pubkey","authority_s":"pubkey","amount":"u64"},"r":["counter_w","authority_s","amount"]},"#);

    // Tool 4: decrement
    json.push_str(r#"{"n":"decrement","i":"Subtract from counter","d":"6ae3a83bf81b9665","p":{"counter_w":"pubkey","authority_s":"pubkey","amount":"u64"},"r":["counter_w","authority_s","amount"]}"#);

    json.push_str("]}");
    json
}

fn build_paginated_page(cursor: u8) -> String {
    let mut json = String::with_capacity(500);
    json.push_str(r#"{"v":"2024-11-05","name":"counter","tools":["#);

    match cursor {
        0 => {
            json.push_str(r#"{"name":"list_tools","description":"List MCP tools","discriminator":"42195e6a55fd41c0"}"#);
            json.push_str(r#"],"nextCursor":"1"}"#);
        }
        1 => {
            json.push_str(r#"{"name":"initialize","description":"Create counter","discriminator":"afaf6d1f0d989bed","parameters":{"counter":{"type":"pubkey","signer":true,"writable":true},"authority":{"type":"pubkey","signer":true}}}"#);
            json.push_str(r#"],"nextCursor":"2"}"#);
        }
        2 => {
            json.push_str(r#"{"name":"increment","description":"Add to counter","discriminator":"0b12680968ae3b21","parameters":{"counter":{"type":"pubkey","writable":true},"authority":{"type":"pubkey","signer":true},"amount":{"type":"u64"}}}"#);
            json.push_str(r#"],"nextCursor":"3"}"#);
        }
        _ => {
            json.push_str(r#"{"name":"decrement","description":"Subtract from counter","discriminator":"6ae3a83bf81b9665","parameters":{"counter":{"type":"pubkey","writable":true},"authority":{"type":"pubkey","signer":true},"amount":{"type":"u64"}}}"#);
            json.push_str("]}");
        }
    }

    json
}

#[test]
fn baseline_compact_schema_generation() {
    const ITERATIONS: u32 = 10000;

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let json = build_4_tool_schema();
        std::hint::black_box(&json);
    }
    let elapsed = start.elapsed();

    let per_call_ns = elapsed.as_nanos() / ITERATIONS as u128;
    let json = build_4_tool_schema();

    println!("\n=== Baseline: Compact Schema Generation ===");
    println!("Schema size: {} bytes", json.len());
    println!("Iterations: {}", ITERATIONS);
    println!("Total time: {:?}", elapsed);
    println!("Per-call: {} ns", per_call_ns);
    println!("Estimated CU equivalent: ~{} (based on 1 CU ≈ 10ns heuristic)", per_call_ns / 10);

    // Verify schema fits in return_data limit
    assert!(json.len() <= 1024, "Schema {} bytes exceeds 1024 limit", json.len());
}

#[test]
fn baseline_paginated_schema_generation() {
    const ITERATIONS: u32 = 10000;
    const PAGES: u8 = 4;

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for cursor in 0..PAGES {
            let json = build_paginated_page(cursor);
            std::hint::black_box(&json);
        }
    }
    let elapsed = start.elapsed();

    let per_page_ns = elapsed.as_nanos() / (ITERATIONS as u128 * PAGES as u128);

    println!("\n=== Baseline: Paginated Schema Generation ===");
    println!("Pages: {}", PAGES);
    println!("Iterations: {} (x {} pages)", ITERATIONS, PAGES);
    println!("Total time: {:?}", elapsed);
    println!("Per-page: {} ns", per_page_ns);
    println!("Estimated CU per page: ~{}", per_page_ns / 10);

    // Verify each page fits
    for cursor in 0..PAGES {
        let json = build_paginated_page(cursor);
        println!("Page {}: {} bytes", cursor, json.len());
        assert!(json.len() <= 1024, "Page {} ({} bytes) exceeds limit", cursor, json.len());
    }
}

#[test]
fn baseline_allocation_overhead() {
    // Measure allocation overhead by comparing pre-sized vs dynamic
    const ITERATIONS: u32 = 10000;

    // Dynamic allocation (current approach)
    let start_dynamic = Instant::now();
    for _ in 0..ITERATIONS {
        let mut json = String::new();
        json.push_str(r#"{"v":"2024-11-05","name":"test","tools":[]}"#);
        std::hint::black_box(&json);
    }
    let dynamic_elapsed = start_dynamic.elapsed();

    // Pre-sized allocation (optimized approach)
    let start_presized = Instant::now();
    for _ in 0..ITERATIONS {
        let mut json = String::with_capacity(100);
        json.push_str(r#"{"v":"2024-11-05","name":"test","tools":[]}"#);
        std::hint::black_box(&json);
    }
    let presized_elapsed = start_presized.elapsed();

    let improvement = if dynamic_elapsed > presized_elapsed {
        ((dynamic_elapsed.as_nanos() - presized_elapsed.as_nanos()) as f64
            / dynamic_elapsed.as_nanos() as f64
            * 100.0) as u32
    } else {
        0
    };

    println!("\n=== Baseline: Allocation Overhead ===");
    println!("Dynamic allocation: {:?}", dynamic_elapsed);
    println!("Pre-sized allocation: {:?}", presized_elapsed);
    println!("Improvement: {}%", improvement);
}
