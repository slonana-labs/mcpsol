# native-counter

MCP integration with raw `solana-program` (no Pinocchio).

## What This Demonstrates

- Using `mcpsol-native` crate
- Standard Solana program patterns
- Adding MCP to existing native programs

## When to Use

- Existing native programs you want to MCP-enable
- Projects that can't use Pinocchio
- Maximum compatibility requirements

## Code Structure

```
src/lib.rs    # Native solana-program implementation
```

Uses standard patterns:
- `solana_program::entrypoint!`
- `next_account_info()` iteration
- `set_return_data()` for schema

## Dependencies

```toml
[dependencies]
mcpsol-native = "0.1"
solana-program = "2.0"
bytemuck = { version = "1.14", features = ["derive"] }
```

## Build

```bash
cargo build-sbf -p native-counter
```

## Test

```bash
cargo test -p native-counter -- --nocapture
```

Tests verify paginated schema generation across all pages.

## Account Layout

```
[0..8]   discriminator
[8..40]  authority pubkey
[40..48] count (u64)
```

Total: 48 bytes

## Deploy

```bash
solana program deploy target/deploy/native_counter.so --url devnet
```
