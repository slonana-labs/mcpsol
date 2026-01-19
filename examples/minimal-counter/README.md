# minimal-counter

Minimal MCP-enabled counter using proc-macro integration. Start here.

## What This Demonstrates

- `#[mcp_program]` macro for automatic entrypoint and schema generation
- `#[mcp_instruction]` macro for instruction registration
- `#[mcp_account]` derive macro for account discriminators
- Zero boilerplate - just define your business logic

## Code Structure

```
src/lib.rs    # ~50 lines of actual code
```

The macros generate:
- Program entrypoint
- Instruction dispatcher with discriminator matching
- `list_tools` instruction returning JSON schema
- All discriminator constants

## Build

```bash
cargo build-sbf -p minimal-counter
```

Output: `target/deploy/minimal_counter.so`

## Test

```bash
cargo test -p minimal-counter

# With output
cargo test -p minimal-counter -- --nocapture
```

Tests verify:
- Schema JSON is generated correctly
- Schema fits within 1024-byte limit
- Discriminators match expected values

## Schema Output

```json
{
  "v": "2024-11-05",
  "name": "minimal_counter",
  "tools": [
    {"n": "increment", "d": "0b12680968ae3b21", ...},
    {"n": "decrement", "d": "6ae3a83bf81b9665", ...},
    {"n": "list_tools", "d": "42195e6a55fd41c0"}
  ]
}
```

## Deploy

```bash
solana program deploy target/deploy/minimal_counter.so --url devnet
```

## Usage Pattern

```rust
#[mcp_program(name = "my_program", description = "Description")]
pub mod my_program {
    #[mcp_instruction(
        name = "my_instruction",
        description = "What it does",
        accounts = "account1:mut, account2:signer"
    )]
    pub fn my_instruction(ctx: Context<Accounts>, arg: u64) -> Result<()> {
        // your logic
    }
}
```
