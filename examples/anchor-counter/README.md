# anchor-counter

MCP integration with Anchor framework.

## What This Demonstrates

- Using `mcpsol-anchor` crate
- `McpProgram` trait implementation
- Hybrid IDL + MCP approach
- Standard Anchor patterns with MCP discovery

## When to Use

- New Anchor programs with MCP support
- Adding MCP to existing Anchor programs
- Teams familiar with Anchor wanting MCP benefits

## Code Structure

```
programs/anchor-counter/src/lib.rs
```

Key pattern - implement `McpProgram` trait:

```rust
impl McpProgram for AnchorCounter {
    fn mcp_schema() -> McpSchema {
        McpSchemaBuilder::new("anchor_counter")
            .add_tool(tool("increment").description("...").build())
            .build()
    }
}
```

## Dependencies

```toml
[dependencies]
anchor-lang = "0.30"
mcpsol-anchor = "0.1"
```

## Build

Requires Anchor CLI:

```bash
anchor build
```

Or with cargo:

```bash
cargo build-sbf -p anchor-counter
```

## Test

```bash
cargo test -p anchor-counter -- --nocapture
```

Tests verify:
- Schema generation works
- Schema fits in 1024 bytes

## list_tools Instruction

```rust
pub fn list_tools(_ctx: Context<ListToolsCtx>) -> Result<()> {
    let schema_bytes = <AnchorCounter as McpProgram>::schema_bytes();
    set_return_data(&schema_bytes);
    Ok(())
}
```

## Deploy

```bash
anchor deploy --provider.cluster devnet
```

Or:

```bash
solana program deploy target/deploy/anchor_counter.so --url devnet
```
