# Examples

Working examples demonstrating mcpsol integration patterns, ordered by complexity.

## Building

All examples compile to SBF (Solana Bytecode Format):

```bash
# Build all examples
cargo build-sbf --workspace

# Build specific example
cargo build-sbf -p minimal-counter
```

## Running Tests

Each example includes unit tests verifying schema generation:

```bash
# Run all example tests
cargo test -p minimal-counter -p counter -p vault

# Run with output
cargo test -p minimal-counter -- --nocapture
```

## Examples

### minimal-counter

**Recommended starting point.** Demonstrates macro-based integration with minimal boilerplate.

- ~50 lines of business logic
- Uses `#[mcp_program]` and `#[mcp_instruction]` macros
- Automatic schema generation, discriminators, and entrypoint

```rust
#[mcp_program(name = "minimal_counter", description = "Minimal MCP counter")]
pub mod minimal_counter {
    #[mcp_instruction(name = "increment", accounts = "counter:mut, authority:signer")]
    pub fn increment(ctx: Context<Modify>, amount: u64) -> Result<()> {
        // business logic only
    }
}
```

Build: `cargo build-sbf -p minimal-counter`
Test: `cargo test -p minimal-counter`

### counter

Manual integration example showing explicit schema construction via builder pattern.

- Full control over schema generation
- Explicit discriminator constants
- Manual instruction dispatch

Use this pattern when you need:
- Custom schema formatting
- Paginated schema responses
- Non-standard discriminator derivation

Build: `cargo build-sbf -p counter`
Test: `cargo test -p counter`

### vault

Demonstrates PDA (Program Derived Address) documentation in MCP schemas.

- PDA seeds documented in tool descriptions for AI agent consumption
- Multiple instruction types (initialize, deposit, withdraw, query)
- Complex account validation

Key pattern - documenting seeds for AI agents:
```rust
McpToolBuilder::new("initialize")
    .description("Create vault PDA. seeds=[\"vault\", owner, mint]")
```

Build: `cargo build-sbf -p vault`
Test: `cargo test -p vault`

### native-counter

Integration with raw `solana-program` (no Pinocchio).

- Uses `mcpsol-native` crate
- Standard Solana program structure
- Suitable for existing native programs

Build: `cargo build-sbf -p native-counter`

### anchor-counter

Anchor framework integration.

- Uses `mcpsol-anchor` crate
- Demonstrates hybrid IDL + MCP approach
- Add MCP to existing Anchor programs

Build: `anchor build` (requires Anchor CLI)

## Schema Verification

Each example generates a schema that fits within Solana's 1024-byte `return_data` limit:

```bash
cargo test -p minimal-counter test_schema_generated -- --nocapture
```

Output shows the generated JSON and byte count:
```
Generated schema (412 bytes):
{"v":"2024-11-05","name":"minimal_counter","tools":[...]}
```

## Deployment

After building, deploy to devnet:

```bash
# Deploy
solana program deploy target/deploy/minimal_counter.so --url devnet

# Note the program ID, then discover interface:
# (using ts-client)
cd ts-client && npm install
npx ts-node -e "
import { discoverTools } from './src';
discoverTools('PROGRAM_ID', 'https://api.devnet.solana.com')
  .then(console.log);
"
```

## Directory Structure

```
examples/
  minimal-counter/     # Start here - macro-based
  counter/             # Manual builder pattern
  vault/               # PDA documentation
  native-counter/      # solana-program integration
  anchor-counter/      # Anchor integration
```
