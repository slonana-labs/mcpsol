# Feature Specification: Minimize Framework Overhead

**Status**: Draft
**Created**: 2026-01-20
**Author**: mcpsol team

## Problem Statement

mcpsol is a framework that other Solana programs depend on. The generated dispatcher code runs on **every instruction call**, adding overhead that users pay for in real SOL. Current implementation prioritizes safety over performance, resulting in ~230 CU overhead per instruction when the actual business logic might only be ~100 CU.

**Every CU mcpsol wastes, the user pays for.**

## Current State Analysis

### Overhead Breakdown (per instruction call)

| Component | Current CU | Description |
|-----------|------------|-------------|
| Length check | ~20 | `instruction_data.len() < 8` |
| Discriminator extraction | ~50 | `try_into()` + `map_err()` |
| Argument parsing (per arg) | ~70 | `get()` + `and_then()` + `ok_or()` |
| Offset tracking | ~10 | Mutable `__offset` variable |
| Context building | ~60 | `Context::new()` + `try_accounts()` |
| Match dispatch | ~30 | N-way discriminator comparison |
| **Total (1 u64 arg)** | **~230** | Framework overhead alone |

### Example: Simple Increment

```
User calls: increment(amount: u64)

CU Budget:
├── mcpsol overhead:     ~230 CU  (70% of total)
├── User business logic: ~100 CU  (30% of total)
└── Total:               ~330 CU
```

For a simple counter increment, the framework overhead is **2.3x the actual work**.

## Goals

### Primary Goal
Reduce mcpsol framework overhead to **< 50 CU** per instruction call.

### Success Criteria

| Metric | Current | Target | Improvement |
|--------|---------|--------|-------------|
| Dispatcher overhead | ~100 CU | < 30 CU | 70%+ |
| Arg parsing (per u64) | ~70 CU | < 10 CU | 85%+ |
| Context building | ~60 CU | 0 CU (optional) | 100% |
| Total framework overhead | ~230 CU | < 50 CU | **78%+** |

## User Stories

### US1: Program Developer Minimizes Transaction Costs
**As a** Solana program developer using mcpsol,
**I want** the framework to add minimal overhead to my instructions,
**So that** my users pay the lowest possible transaction fees.

**Acceptance Criteria:**
- Framework overhead < 50 CU for simple instructions
- No heap allocations in hot path
- Zero-cost abstractions where possible

### US2: High-Frequency Program Optimization
**As a** developer of a high-frequency trading program,
**I want** mcpsol overhead to be negligible compared to business logic,
**So that** I can compete with hand-optimized native programs.

**Acceptance Criteria:**
- Framework overhead < 10% of typical instruction CU budget
- Deterministic CU consumption (no variable overhead)
- Option to bypass Context wrapper entirely

### US3: Framework Transparency
**As a** program developer,
**I want** to understand exactly what CU overhead mcpsol adds,
**So that** I can make informed decisions about using the framework.

**Acceptance Criteria:**
- Documentation of CU cost per framework component
- Compile-time visibility of generated code overhead
- Benchmark suite proving overhead claims

## Functional Requirements

### FR-001: Optimized Discriminator Extraction
The discriminator must be extracted with minimal overhead after a single bounds check.

**Current:**
```rust
if instruction_data.len() < 8 { return Err(...); }
let discriminator: [u8; 8] = instruction_data[..8]
    .try_into()
    .map_err(|_| ...)?;
```

**Required:**
```rust
if instruction_data.len() < TOTAL_EXPECTED_LEN { return Err(...); }
let discriminator = unsafe { *(instruction_data.as_ptr() as *const [u8; 8]) };
```

### FR-002: Single Bounds Check for All Arguments
Argument length must be validated once at dispatch time, not per-argument.

**Current:** Each argument performs its own bounds check via `.get()`.

**Required:** Calculate total expected instruction data length at compile time, check once.

### FR-003: Zero-Copy Argument Parsing
Arguments must be read directly from instruction data without intermediate allocations or Option chains.

**Current:**
```rust
let amount: u64 = u64::from_le_bytes(
    data.get(__offset..__offset + 8)
        .and_then(|s| s.try_into().ok())
        .ok_or(...)?
);
```

**Required:**
```rust
let amount = u64::from_le_bytes(
    unsafe { *(data.as_ptr().add(offset) as *const [u8; 8]) }
);
```

### FR-004: Optional Context Wrapper
Context building must be opt-in, not mandatory. Programs that don't need it shouldn't pay for it.

**Required:** Support direct account slice access without Context overhead.

### FR-005: Compile-Time Offset Calculation
Argument offsets must be computed at compile time, not tracked with mutable runtime variables.

**Current:** `let mut __offset: usize = 0; ... __offset += 8;`

**Required:** Generate fixed offsets: `const ARG0_OFFSET: usize = 8; const ARG1_OFFSET: usize = 16;`

## Non-Functional Requirements

### NFR-001: Safety
All unsafe code must be sound given the bounds check precondition. Document safety invariants.

### NFR-002: Backwards Compatibility
Existing programs using mcpsol must continue to compile. Optimization should be transparent.

### NFR-003: Auditability
Generated code must be readable and auditable. Provide `cargo expand` examples in documentation.

### NFR-004: Benchmark Verification
All CU claims must be verifiable via the benchmark suite. No unsubstantiated performance claims.

## Out of Scope

- Optimizing user business logic
- Account loading CU (Solana runtime, not framework)
- Schema generation (already optimized, runs in simulation)
- Client-side code

## Constraints

### C-001: Solana BPF Environment
- No standard library in program code
- Limited stack size (4KB)
- No heap by default (opt-in 32KB)

### C-002: Pinocchio Compatibility
Must work with Pinocchio's account types and error handling.

### C-003: Macro Hygiene
Generated code must not conflict with user-defined names.

## Assumptions

- Programs using mcpsol prioritize performance over verbose error messages
- Unsafe code is acceptable when provably sound
- Compile-time computation is preferable to runtime computation

## Dependencies

- Rust nightly features may be needed for const generics optimization
- Pinocchio 0.8+ for AccountInfo compatibility

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Unsafe code introduces bugs | Medium | High | Extensive testing, formal review |
| Breaking existing programs | Low | High | Feature flag for optimized path |
| Compiler optimizations vary | Medium | Medium | Benchmark across rustc versions |

## Success Metrics

1. **CU Reduction**: Measure before/after on standardized instruction set
2. **Adoption**: Existing mcpsol users upgrade without issues
3. **Competitive**: Overhead comparable to hand-written native programs

## Appendix: Benchmark Comparison Target

| Framework | Dispatcher Overhead | Notes |
|-----------|---------------------|-------|
| Hand-written native | ~20 CU | Baseline |
| Anchor | ~150 CU | With IDL overhead |
| mcpsol (current) | ~230 CU | Too high |
| **mcpsol (target)** | **< 50 CU** | Competitive with native |
