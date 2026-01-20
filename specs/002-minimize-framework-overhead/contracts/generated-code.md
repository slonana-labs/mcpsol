# Contract: Generated Dispatcher Code

## Before (Current Implementation)

For a program with `increment(amount: u64)`:

```rust
/// Process incoming instructions
pub fn __mcpsol_process_instruction(
    program_id: &pinocchio::pubkey::Pubkey,
    accounts: &[pinocchio::account_info::AccountInfo],
    instruction_data: &[u8],
) -> pinocchio::ProgramResult {
    // OVERHEAD: ~20 CU
    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // OVERHEAD: ~50 CU (try_into + map_err)
    let discriminator: [u8; 8] = instruction_data[..8]
        .try_into()
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    let data = &instruction_data[8..];

    match discriminator {
        // list_tools
        [0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0] => {
            pinocchio::program::set_return_data(counter::MCP_SCHEMA_BYTES);
            Ok(())
        }
        // increment
        [0x0b, 0x12, 0x68, 0x09, 0x68, 0xae, 0x3b, 0x21] => {
            // OVERHEAD: ~10 CU (mutable offset)
            let mut __offset: usize = 0;

            // OVERHEAD: ~70 CU (bounds check chain)
            let amount: u64 = u64::from_le_bytes(
                data.get(__offset..__offset + 8)
                    .and_then(|s| s.try_into().ok())
                    .ok_or(ProgramError::InvalidInstructionData)?
            );
            __offset += 8;

            // OVERHEAD: ~50 CU (Context building)
            let ctx = mcpsol::context::Context::new(
                program_id,
                <Modify as mcpsol::context::Accounts>::try_accounts(program_id, accounts)?,
                &[]
            );

            counter::increment(ctx, amount)?;
            Ok(())
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

// TOTAL OVERHEAD: ~200 CU
```

## After (Optimized Implementation)

```rust
/// Process incoming instructions
pub fn __mcpsol_process_instruction(
    program_id: &pinocchio::pubkey::Pubkey,
    accounts: &[pinocchio::account_info::AccountInfo],
    instruction_data: &[u8],
) -> pinocchio::ProgramResult {
    // OVERHEAD: ~10 CU (single check)
    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // OVERHEAD: ~5 CU (direct read)
    // SAFETY: Length >= 8 verified above
    let discriminator = unsafe {
        *(instruction_data.as_ptr() as *const [u8; 8])
    };

    match discriminator {
        // list_tools
        [0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0] => {
            pinocchio::program::set_return_data(counter::MCP_SCHEMA_BYTES);
            Ok(())
        }
        // increment
        [0x0b, 0x12, 0x68, 0x09, 0x68, 0xae, 0x3b, 0x21] => {
            // OVERHEAD: ~10 CU (compile-time constant check)
            const EXPECTED_LEN: usize = 8 + 8; // disc + amount
            if instruction_data.len() < EXPECTED_LEN {
                return Err(ProgramError::InvalidInstructionData);
            }

            // OVERHEAD: ~5 CU (direct read at known offset)
            // SAFETY: Length >= EXPECTED_LEN verified above
            let amount = unsafe {
                core::ptr::read_unaligned(
                    instruction_data.as_ptr().add(8) as *const u64
                )
            };

            // NO Context overhead - direct call
            counter::increment(program_id, accounts, amount)?;
            Ok(())
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

// TOTAL OVERHEAD: ~30 CU
```

## Comparison

| Component | Before | After | Savings |
|-----------|--------|-------|---------|
| Initial length check | 20 CU | 10 CU | 50% |
| Discriminator read | 50 CU | 5 CU | 90% |
| Arg length check | 20 CU | 10 CU | 50% |
| Arg parsing (u64) | 70 CU | 5 CU | 93% |
| Context building | 50 CU | 0 CU | 100% |
| **Total** | **210 CU** | **30 CU** | **86%** |

## API Contract

### Instruction Handler Signature (Without Context)

```rust
// NEW: Default signature - no Context wrapper
#[mcp_instruction(name = "increment", accounts = "counter:mut, authority:signer")]
pub fn increment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> Result<()> {
    // User accesses accounts directly
    let counter = &accounts[0];
    let authority = &accounts[1];
    // ...
}
```

### Instruction Handler Signature (With Context)

```rust
// OPT-IN: Context wrapper when needed
#[mcp_instruction(name = "complex", context = true)]
pub fn complex(
    ctx: Context<ComplexAccounts>,
    amount: u64,
) -> Result<()> {
    // User accesses accounts via ctx.accounts
    let counter = ctx.accounts.counter;
    // ...
}
```

## Backwards Compatibility

Existing code with Context signatures continues to work:
- Macro detects `Context<T>` in first parameter
- Automatically enables context building
- No code changes required for existing programs

New code can opt out of Context:
- Use raw `&[AccountInfo]` signature
- Framework generates optimized dispatcher
- ~50 CU savings per instruction
