//! Pagination CU benchmark tests for mcpsol
//!
//! Compares CU consumption between:
//! - Direct generation: Regenerating JSON on each list_tools call
//! - Cached generation: Pre-computed pages via CachedSchemaPages
//!
//! Run with: cargo test --package mcpsol-core --test pagination -- --nocapture

use mcpsol_core::{
    ArgType, CachedSchemaPages, McpSchemaBuilder, McpToolBuilder,
    generate_paginated_schema_bytes,
};
use std::time::Instant;

/// Build a 10-tool schema for pagination benchmarks
fn build_10_tool_schema() -> mcpsol_core::McpSchema {
    McpSchemaBuilder::new("defi_protocol")
        .add_tool(
            McpToolBuilder::new("list_tools")
                .description("List available MCP tools. Pass cursor byte to paginate.")
                .build()
        )
        .add_tool(
            McpToolBuilder::new("initialize_pool")
                .description("Create a new liquidity pool")
                .signer_writable_desc("pool", "Pool account to create")
                .signer_desc("authority", "Pool authority")
                .account_with_desc("token_a_mint", "Token A mint", false, false)
                .account_with_desc("token_b_mint", "Token B mint", false, false)
                .arg_desc("fee_rate", "Fee rate in basis points", ArgType::U16)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("add_liquidity")
                .description("Add liquidity to a pool")
                .writable_desc("pool", "Pool to add liquidity to")
                .signer_desc("provider", "Liquidity provider")
                .writable_desc("provider_token_a", "Provider's token A account")
                .writable_desc("provider_token_b", "Provider's token B account")
                .arg_desc("amount_a", "Amount of token A", ArgType::U64)
                .arg_desc("amount_b", "Amount of token B", ArgType::U64)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("remove_liquidity")
                .description("Remove liquidity from a pool")
                .writable_desc("pool", "Pool to remove from")
                .signer_desc("provider", "Liquidity provider")
                .writable_desc("lp_tokens", "LP token account to burn")
                .arg_desc("lp_amount", "Amount of LP tokens to burn", ArgType::U64)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("swap")
                .description("Swap tokens through the AMM")
                .writable_desc("pool", "Pool to swap through")
                .signer_desc("user", "User performing swap")
                .writable_desc("user_token_in", "User's input token account")
                .writable_desc("user_token_out", "User's output token account")
                .arg_desc("amount_in", "Amount to swap", ArgType::U64)
                .arg_desc("min_out", "Minimum output (slippage)", ArgType::U64)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("stake")
                .description("Stake LP tokens for rewards")
                .writable_desc("stake_account", "User's stake account")
                .signer_desc("user", "Staking user")
                .writable_desc("lp_tokens", "LP tokens to stake")
                .arg_desc("amount", "Amount to stake", ArgType::U64)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("unstake")
                .description("Unstake LP tokens")
                .writable_desc("stake_account", "User's stake account")
                .signer_desc("user", "Unstaking user")
                .arg_desc("amount", "Amount to unstake", ArgType::U64)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("claim_rewards")
                .description("Claim staking rewards")
                .writable_desc("stake_account", "User's stake account")
                .signer_desc("user", "Claiming user")
                .writable_desc("reward_account", "Account to receive rewards")
                .build()
        )
        .add_tool(
            McpToolBuilder::new("get_pool_info")
                .description("Get pool state via return_data")
                .account_with_desc("pool", "Pool to query", false, false)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("get_stake_info")
                .description("Get stake state via return_data")
                .account_with_desc("stake_account", "Stake account to query", false, false)
                .build()
        )
        .build()
}

#[test]
fn benchmark_direct_vs_cached_pagination() {
    let schema = build_10_tool_schema();
    let num_pages = schema.tools.len();

    const ITERATIONS: u32 = 5000;

    // Direct generation: regenerate JSON on each call
    let start_direct = Instant::now();
    for _ in 0..ITERATIONS {
        for cursor in 0..num_pages {
            let bytes = generate_paginated_schema_bytes(&schema, cursor as u8);
            std::hint::black_box(&bytes);
        }
    }
    let direct_elapsed = start_direct.elapsed();

    // Cached generation: pre-compute once, return references
    let cached = CachedSchemaPages::from_schema(&schema);
    let start_cached = Instant::now();
    for _ in 0..ITERATIONS {
        for cursor in 0..num_pages {
            let bytes = cached.get_page(cursor as u8);
            std::hint::black_box(&bytes);
        }
    }
    let cached_elapsed = start_cached.elapsed();

    let direct_per_page_ns = direct_elapsed.as_nanos() / (ITERATIONS as u128 * num_pages as u128);
    let cached_per_page_ns = cached_elapsed.as_nanos() / (ITERATIONS as u128 * num_pages as u128);

    let improvement_pct = if direct_per_page_ns > cached_per_page_ns {
        ((direct_per_page_ns - cached_per_page_ns) as f64 / direct_per_page_ns as f64 * 100.0) as u32
    } else {
        0
    };

    println!("\n=== Pagination Benchmark: Direct vs Cached ({} pages) ===", num_pages);
    println!("Iterations: {} (x {} pages = {} total calls)", ITERATIONS, num_pages, ITERATIONS * num_pages as u32);
    println!();
    println!("Direct generation:");
    println!("  Total: {:?}", direct_elapsed);
    println!("  Per-page: {} ns", direct_per_page_ns);
    println!("  Est. CU/page: ~{}", direct_per_page_ns / 10);
    println!();
    println!("Cached generation:");
    println!("  Total: {:?}", cached_elapsed);
    println!("  Per-page: {} ns", cached_per_page_ns);
    println!("  Est. CU/page: ~{}", cached_per_page_ns / 10);
    println!();
    println!("Improvement: {}% reduction per page", improvement_pct);

    // Target: 15%+ reduction per page (per spec)
    assert!(improvement_pct >= 15,
        "Expected 15%+ CU reduction, got {}%", improvement_pct);
}

#[test]
fn benchmark_full_discovery_cycle() {
    // Simulates an AI agent discovering all tools via pagination
    let schema = build_10_tool_schema();
    let num_pages = schema.tools.len();

    const ITERATIONS: u32 = 1000;

    // Direct: Each page regenerated
    let start_direct = Instant::now();
    for _ in 0..ITERATIONS {
        let mut cursor: u8 = 0;
        loop {
            let bytes = generate_paginated_schema_bytes(&schema, cursor);
            // Check if more pages (simplified - in real usage, parse nextCursor)
            if cursor as usize >= num_pages - 1 {
                break;
            }
            cursor += 1;
            std::hint::black_box(&bytes);
        }
    }
    let direct_elapsed = start_direct.elapsed();

    // Cached: Pre-computed pages
    let cached = CachedSchemaPages::from_schema(&schema);
    let start_cached = Instant::now();
    for _ in 0..ITERATIONS {
        let mut cursor: u8 = 0;
        loop {
            let bytes = cached.get_page(cursor);
            if cursor as usize >= cached.num_pages() - 1 {
                break;
            }
            cursor += 1;
            std::hint::black_box(&bytes);
        }
    }
    let cached_elapsed = start_cached.elapsed();

    let direct_per_cycle_us = direct_elapsed.as_micros() / ITERATIONS as u128;
    let cached_per_cycle_us = cached_elapsed.as_micros() / ITERATIONS as u128;

    println!("\n=== Full Discovery Cycle ({} pages per cycle) ===", num_pages);
    println!("Iterations: {}", ITERATIONS);
    println!();
    println!("Direct: {} µs per full discovery", direct_per_cycle_us);
    println!("Cached: {} µs per full discovery", cached_per_cycle_us);

    let improvement = if direct_per_cycle_us > cached_per_cycle_us {
        ((direct_per_cycle_us - cached_per_cycle_us) as f64 / direct_per_cycle_us as f64 * 100.0) as u32
    } else {
        0
    };
    println!("Total CU savings per discovery: {}%", improvement);
}

#[test]
fn verify_cached_output_correctness() {
    let schema = build_10_tool_schema();
    let cached = CachedSchemaPages::from_schema(&schema);

    println!("\n=== Cached Output Verification ===");

    for cursor in 0..schema.tools.len() {
        let direct_bytes = generate_paginated_schema_bytes(&schema, cursor as u8);
        let cached_bytes = cached.get_page(cursor as u8);

        assert_eq!(
            direct_bytes, cached_bytes,
            "Page {} differs between direct and cached generation",
            cursor
        );

        let _json = String::from_utf8_lossy(cached_bytes);
        println!("Page {}: {} bytes - OK", cursor, cached_bytes.len());

        // Verify fits in return_data
        assert!(cached_bytes.len() <= 1024,
            "Page {} ({} bytes) exceeds 1024 limit", cursor, cached_bytes.len());
    }

    println!("All {} pages verified identical and within size limits", schema.tools.len());
}

#[test]
fn measure_cache_initialization_overhead() {
    const ITERATIONS: u32 = 1000;

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let schema = build_10_tool_schema();
        let cached = CachedSchemaPages::from_schema(&schema);
        std::hint::black_box(&cached);
    }
    let elapsed = start.elapsed();

    let per_init_us = elapsed.as_micros() / ITERATIONS as u128;

    println!("\n=== Cache Initialization Overhead ===");
    println!("Iterations: {}", ITERATIONS);
    println!("Total: {:?}", elapsed);
    println!("Per-init: {} µs", per_init_us);
    println!();
    println!("Note: Initialization happens once at program startup.");
    println!("Amortized over many list_tools calls, this overhead is negligible.");
}
