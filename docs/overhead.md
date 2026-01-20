# Framework Overhead

CU (compute unit) cost of using mcpsol.

## Cost Summary

| Component | CU Cost | Notes |
|-----------|---------|-------|
| Discriminator dispatch | ~5-10 CU | Fixed per instruction |
| Argument parsing | ~5-10 CU | Per argument |
| Context wrapper | ~30-50 CU | Optional |
| **Total (with Context)** | **~50-70 CU** | Default mode |
| **Total (without Context)** | **~20-30 CU** | Maximum performance |

> CU numbers are estimates for on-chain BPF. Host benchmarks show lower numbers.

## When to Use Context

| Use Case | Recommendation |
|----------|----------------|
| Simple instructions (1-2 accounts) | Skip Context (~20-30 CU) |
| Need `remaining_accounts` | Use Context (required) |
| Complex validation | Use Context (ergonomics) |
| Performance critical | Skip Context |

## Examples

### With Context (default)

```rust
#[mcp_instruction(name = "increment")]
pub fn increment(ctx: Context<Modify>, amount: u64) -> Result<()> {
    let counter = ctx.accounts.counter;
    // ...
}
```

### Without Context (faster)

```rust
#[mcp_instruction(name = "increment", accounts = "counter:mut, authority:signer")]
pub fn increment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> Result<()> {
    let counter = &accounts[0];
    let authority = &accounts[1];
    // ...
}
```

The macro detects the signature and generates the appropriate code path.
