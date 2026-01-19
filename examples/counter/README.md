# counter

Manual MCP integration using the builder pattern. Use this when you need full control.

## What This Demonstrates

- `McpSchemaBuilder` for explicit schema construction
- `McpToolBuilder` for detailed tool definitions with descriptions
- Manual instruction dispatch
- Paginated schema responses for complex programs

## When to Use This Pattern

- Custom schema formatting requirements
- Non-standard discriminator derivation
- Paginated responses (schemas exceeding single-page limit)
- Gradual migration of existing programs

## Code Structure

```
src/lib.rs    # Full implementation with explicit wiring
```

Key components:
- `build_schema()` - Constructs schema using builder API
- Manual discriminator constants
- Explicit entrypoint and dispatch logic

## Build

```bash
cargo build-sbf -p counter
```

## Test

```bash
cargo test -p counter -- --nocapture
```

## Schema Construction

```rust
fn build_schema() -> McpSchema {
    McpSchemaBuilder::new("counter")
        .add_tool(
            CoreToolBuilder::new("increment")
                .description("Add amount to counter")
                .writable_desc("counter", "Counter account to modify")
                .signer_desc("authority", "Must match counter authority")
                .arg_desc("amount", "Value to add", ArgType::U64)
                .build()
        )
        .build()
}
```

## Pagination

For large schemas, use cursor-based pagination:

```rust
let cursor = data.get(8).copied().unwrap_or(0);
let schema_bytes = generate_paginated_schema_bytes(get_schema(), cursor);
```

Client iterates pages until `nextCursor` is absent from response.

## Deploy

```bash
solana program deploy target/deploy/counter.so --url devnet
```
