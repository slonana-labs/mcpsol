# Quickstart: CU-Optimized mcpsol

## Verifying CU Optimization

After implementation, verify the optimizations are working:

### For Macro-based Programs

```rust
// Your program using #[mcp_program] automatically benefits
// No code changes required - optimization is in the macro

#[mcp_program(name = "my_program", description = "...")]
pub mod my_program {
    #[mcp_instruction(name = "my_action", ...)]
    pub fn my_action(ctx: Context<...>) -> Result<()> { ... }
}
```

The macro now generates `MCP_SCHEMA_BYTES: &'static [u8]` instead of a string, eliminating the `.as_bytes()` conversion at runtime.

### For Builder-pattern Programs

```rust
use mcpsol_core::{McpSchemaBuilder, McpToolBuilder, CachedSchemaPages};
use std::sync::OnceLock;

// Cache schema pages at first access
static CACHED_PAGES: OnceLock<CachedSchemaPages> = OnceLock::new();

fn get_cached_pages() -> &'static CachedSchemaPages {
    CACHED_PAGES.get_or_init(|| CachedSchemaPages::from_schema(&build_schema()))
}

pub fn process_list_tools(cursor: u8) {
    // Returns &[u8] - no allocation on subsequent calls
    let page = get_cached_pages().get_page(cursor);
    pinocchio::program::set_return_data(page);
}

// Alternative: Manual caching (existing pattern, still works)
static SCHEMA: OnceLock<McpSchema> = OnceLock::new();

fn get_schema() -> &'static McpSchema {
    SCHEMA.get_or_init(build_schema)
}
```

### Running CU Benchmarks

```bash
# Run the benchmark suite
cargo test --package mcpsol-core --test cu_benchmarks -- --nocapture

# Expected output:
# list_tools (4 tools, first call): 450 CU
# list_tools (4 tools, cached): 120 CU
# list_tools (10 tools, page 0): 130 CU
```

## Measuring CU in Your Program

Add compute budget logging to verify CU consumption:

```rust
use pinocchio_log::log;

pub fn process_list_tools(cursor: u8) {
    // Log CU at start
    log!("list_tools start");

    let page = get_cached_page(cursor);
    pinocchio::program::set_return_data(page);

    // Log CU at end (delta shows instruction cost)
    log!("list_tools end");
}
```

Then check logs in your test:

```bash
solana logs | grep "list_tools"
```

## Migration Guide

### From Pre-optimization mcpsol

No code changes required. Simply update your `Cargo.toml`:

```toml
[dependencies]
mcpsol = "0.2"  # CU-optimized version
```

Rebuild and redeploy. The optimization is automatic.

### Verifying No Schema Changes

Ensure your schema output is identical before and after:

```bash
# Before upgrade
solana program invoke <PROGRAM_ID> --data "42195e6a55fd41c0" > schema_before.json

# After upgrade
solana program invoke <PROGRAM_ID> --data "42195e6a55fd41c0" > schema_after.json

# Compare
diff schema_before.json schema_after.json
# Should show no differences
```

## Troubleshooting

### CU Not Reduced

1. Ensure you're using the latest mcpsol version
2. For builder-pattern: verify you're using `CachedSchemaPages` or `OnceLock`
3. Check that `build_schema()` is not being called on every request

### Schema Output Changed

This should not happen. If it does:
1. File a bug report
2. Pin to the previous version until fixed:
   ```toml
   mcpsol = "=0.1.x"
   ```
