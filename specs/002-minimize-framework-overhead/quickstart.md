# Quickstart: Framework Overhead Optimization

## Overview

This guide covers the framework overhead optimizations in mcpsol and how to take advantage of them.

## What Changed

### Before: Safe but Slow

```rust
// Old generated code (~200 CU overhead)
let discriminator: [u8; 8] = data[..8].try_into().map_err(|_| ...)?;
let amount = u64::from_le_bytes(
    args.get(0..8).and_then(|s| s.try_into().ok()).ok_or(...)?
);
let ctx = Context::new(...);
handler(ctx, amount)?;
```

### After: Fast and Still Safe

```rust
// New generated code (~30 CU overhead)
let discriminator = unsafe { *(data.as_ptr() as *const [u8; 8]) };
let amount = unsafe { read_unaligned(data.as_ptr().add(8) as *const u64) };
handler(program_id, accounts, amount)?;
```

## Migration Scenarios

### Scenario 1: Existing Program (No Changes Required)

Your existing mcpsol program automatically benefits from optimized discriminator and argument parsing. No code changes needed.

```rust
// This still works - Context detected, built automatically
#[mcp_instruction(name = "increment")]
pub fn increment(ctx: Context<Modify>, amount: u64) -> Result<()> {
    // existing code
}
```

**Benefit**: ~100 CU savings from optimized parsing

### Scenario 2: Opt Out of Context (Maximum Performance)

For simple instructions that don't need Context features:

```rust
// Before: With Context (~200 CU overhead)
#[mcp_instruction(name = "increment")]
pub fn increment(ctx: Context<Modify>, amount: u64) -> Result<()> {
    let counter = ctx.accounts.counter;
    // ...
}

// After: Without Context (~30 CU overhead)
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

**Benefit**: Additional ~50 CU savings from skipping Context

### Scenario 3: Mixed Approach

Use Context only where you need it:

```rust
// Simple instruction - no Context
#[mcp_instruction(name = "increment", accounts = "counter:mut, authority:signer")]
pub fn increment(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> Result<()> {
    let counter = &accounts[0];
    // simple logic
}

// Complex instruction - with Context for remaining_accounts
#[mcp_instruction(name = "batch_transfer", context = true)]
pub fn batch_transfer(
    ctx: Context<BatchTransfer>,
    amounts: Vec<u64>,
) -> Result<()> {
    // Access remaining_accounts for dynamic recipient list
    for (i, recipient) in ctx.remaining_accounts.iter().enumerate() {
        // ...
    }
}
```

## Benchmark Verification

After upgrading, verify the improvement:

```bash
# Run overhead benchmarks
cargo test --package mcpsol-core --test overhead -- --nocapture

# Expected output:
# Dispatcher overhead: 30 CU (was 200 CU)
# Per-argument parsing: 5 CU (was 70 CU)
```

## Safety Guarantees

The optimizations use `unsafe` but maintain safety:

1. **Bounds Checking**: Single comprehensive check before any unsafe read
2. **Debug Assertions**: Extra checks in debug builds
3. **Documentation**: All unsafe blocks have SAFETY comments

```rust
// Generated code includes safety invariants:
const EXPECTED_LEN: usize = 16;
if instruction_data.len() < EXPECTED_LEN {
    return Err(InvalidInstructionData);
}

// SAFETY: Length >= EXPECTED_LEN verified above
debug_assert!(8 + 8 <= instruction_data.len());
let amount = unsafe { ... };
```

## Troubleshooting

### Issue: Compile Error on Unknown Type

```
error: Unknown argument type `MyCustomType` - cannot determine size
```

**Solution**: Use `#[repr(C)]` and implement known size:

```rust
#[repr(C)]
pub struct MyCustomType {
    pub field: u64,  // 8 bytes
}

// Or use primitive types in instruction signature
```

### Issue: Context Not Available

```
error: `ctx.remaining_accounts` not found
```

**Solution**: Add `context = true` to the instruction attribute:

```rust
#[mcp_instruction(name = "my_ix", context = true)]
pub fn my_ix(ctx: Context<MyAccounts>, ...) -> Result<()>
```

## Viewing Generated Code

Use `cargo expand` to see the actual generated dispatcher code:

```bash
# Install cargo-expand if not already installed
cargo install cargo-expand

# Expand the minimal-counter example
cargo expand --package minimal-counter

# You'll see the optimized dispatcher:
# pub fn __mcpsol_process_instruction(
#     program_id: &Pubkey,
#     accounts: &[AccountInfo],
#     instruction_data: &[u8],
# ) -> ProgramResult {
#     if instruction_data.len() < 8 {
#         return Err(ProgramError::InvalidInstructionData);
#     }
#
#     // SAFETY: Length >= 8 verified above
#     let discriminator = unsafe {
#         *(instruction_data.as_ptr() as *const [u8; 8])
#     };
#
#     match discriminator {
#         [0x42, 0x19, ...] => { /* list_tools */ }
#         [0x0b, 0x12, ...] => {
#             const __EXPECTED_LEN: usize = 16;
#             if instruction_data.len() < __EXPECTED_LEN {
#                 return Err(ProgramError::InvalidInstructionData);
#             }
#             // SAFETY: instruction_data.len() >= __EXPECTED_LEN checked above
#             let amount: u64 = unsafe {
#                 core::ptr::read_unaligned(
#                     instruction_data.as_ptr().add(8) as *const u64
#                 )
#             };
#             // ... handler call
#         }
#         _ => Err(ProgramError::InvalidInstructionData),
#     }
# }
```

## Summary

| Approach | Overhead | When to Use |
|----------|----------|-------------|
| Auto-detect Context | ~60 CU | Existing code, no changes |
| Explicit no Context | ~30 CU | Simple instructions, max performance |
| Explicit with Context | ~60 CU | Need remaining_accounts, validation |
