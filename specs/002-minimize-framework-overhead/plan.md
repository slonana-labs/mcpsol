# Implementation Plan: Minimize Framework Overhead

**Branch**: `002-minimize-framework-overhead` | **Date**: 2026-01-20 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-minimize-framework-overhead/spec.md`

## Summary

Reduce mcpsol framework overhead from ~230 CU to <50 CU per instruction call by:
1. Replacing safe-but-slow bounds checking with single upfront validation + unsafe direct reads
2. Eliminating Option chains in argument parsing with compile-time offset calculation
3. Making Context wrapper optional (zero-cost when not used)

## Technical Context

**Language/Version**: Rust 1.75+ (stable, no nightly required)
**Primary Dependencies**: pinocchio 0.8, proc-macro2, syn 2.0, quote 1.0
**Storage**: N/A (on-chain program, no persistent storage beyond accounts)
**Testing**: cargo test, CU benchmark suite in `core/tests/`
**Target Platform**: Solana BPF/SBF runtime
**Project Type**: Rust workspace (proc-macro + library crates)
**Performance Goals**: <50 CU framework overhead per instruction, <10 CU per argument parse
**Constraints**: no_std compatible, 4KB stack limit, deterministic execution
**Scale/Scope**: Affects all programs using mcpsol macros

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. On-Chain Truth | PASS | No change - schema still embedded at compile time |
| II. Zero External Dependencies | PASS | No change - still RPC-only discovery |
| III. Size Discipline | PASS | No change - 1024-byte limit maintained |
| IV. Framework Agnostic | PASS | Optimizations apply to all framework integrations |
| V. Compute Efficiency | **IMPROVES** | This feature directly addresses this principle |

**Gate Status**: PASS - Feature aligns with and improves constitutional compliance.

## Project Structure

### Documentation (this feature)

```text
specs/002-minimize-framework-overhead/
├── plan.md              # This file
├── research.md          # Phase 0: Unsafe patterns research
├── data-model.md        # Phase 1: Generated code structures
├── quickstart.md        # Phase 1: Migration guide
├── contracts/           # Phase 1: Before/after code examples
└── tasks.md             # Phase 2: Implementation tasks
```

### Source Code (repository root)

```text
macros/src/
├── lib.rs               # Proc macro entry points
├── program.rs           # Dispatcher generation (PRIMARY TARGET)
├── discriminator.rs     # Discriminator calculation
└── mcp_gen.rs           # Schema generation

core/src/
├── lib.rs               # Core exports
├── schema.rs            # Schema types
└── json.rs              # JSON generation

sdk/src/
├── lib.rs               # SDK exports
├── context.rs           # Context wrapper
├── read.rs              # NEW: Unsafe read helpers (OPTIMIZE)
├── account.rs           # Account helpers
└── error.rs             # Error types

core/tests/
├── baseline.rs          # Existing baseline benchmarks
├── pagination.rs        # Existing pagination benchmarks
├── cu_benchmarks.rs     # Existing comprehensive benchmarks
└── overhead.rs          # NEW: Framework overhead benchmarks
```

**Structure Decision**: Existing workspace structure. Changes focused on `macros/src/program.rs` (dispatcher generation) and `sdk/src/read.rs` (unsafe read helpers). Context is made optional via code generation, not by modifying context.rs.

## Complexity Tracking

No constitution violations - this feature reduces complexity by eliminating unnecessary runtime checks.
