<!--
Sync Impact Report
==================
Version change: 0.0.0 → 1.0.0 (initial ratification)
Modified principles: N/A (initial)
Added sections:
  - Core Principles (5 principles)
  - Technical Constraints
  - Development Workflow
  - Governance
Removed sections: N/A
Templates requiring updates:
  - .specify/templates/plan-template.md ✅ (Constitution Check section compatible)
  - .specify/templates/spec-template.md ✅ (requirements structure compatible)
  - .specify/templates/tasks-template.md ✅ (phase structure compatible)
Follow-up TODOs: None
-->

# mcpsol Constitution

## Core Principles

### I. On-Chain Truth

The schema MUST be compiled into the program binary. There MUST be exactly one source of truth: the deployed code.

- Interface schemas are embedded at compile time via proc-macros or builder pattern
- No external IDL files, registries, or distribution channels
- Schema cannot drift from implementation because they are the same artifact

**Rationale**: External IDL distribution creates synchronization problems, hosting dependencies, and version mismatches. Embedding eliminates these failure modes entirely.

### II. Zero External Dependencies

Interface discovery MUST require only an RPC endpoint. No file servers, IPFS, CDNs, or registries.

- Discovery happens via simulated `list_tools` transaction
- Uses existing Solana RPC infrastructure
- If you can submit a transaction, you can discover the interface

**Rationale**: External dependencies are points of failure. The fewer systems involved in discovery, the more reliable the protocol.

### III. Size Discipline

All schemas MUST fit within Solana's 1024-byte `return_data` limit per response.

- Use abbreviated field names (`n` not `name`, `d` not `discriminator`)
- Use type abbreviations (`int` not `integer`)
- Use account flag suffixes (`_sw`) instead of verbose objects
- Omit optional fields when empty
- Complex programs MUST use pagination (one tool per page with cursor)

**Rationale**: The constraint forces brevity and pressures toward smaller, focused programs. This is a feature, not a limitation.

### IV. Framework Agnostic

The protocol MUST work with any Solana development framework.

- Pinocchio, Anchor, native `solana-program` are all supported
- Clients parse identical JSON regardless of how it was generated
- No framework-specific parsing logic required

**Rationale**: Framework lock-in fragments the ecosystem. A universal protocol enables universal tooling.

### V. Compute Efficiency

Schema generation MUST happen at compile time. Runtime cost MUST be minimal.

- Proc-macros generate schema JSON as compile-time constants
- Runtime cost is a single `set_return_data` call (~100 CUs)
- No parsing, allocation, or computation at runtime

**Rationale**: On-chain compute is expensive. Moving work to compile time is always preferable.

## Technical Constraints

**Discriminator Format**: Anchor-compatible SHA256 sighash for cross-framework interoperability.
- Instructions: `SHA256("global:<name>")[..8]`
- Accounts: `SHA256("account:<Name>")[..8]`
- Reserved: `list_tools` uses fixed discriminator `42195e6a55fd41c0`

**Schema Version**: Date-based format `YYYY-MM-DD` (current: `2024-11-05`).

**Wire Format**: JSON with abbreviated keys. See `docs/schema.md` for specification.

**Account Layout**: Anchor-compatible with 8-byte discriminator prefix.

## Development Workflow

**Testing Requirements**:
- All examples MUST include tests verifying schema generation
- Schema size MUST be validated against 1024-byte limit
- Discriminator values MUST be verified against expected constants

**Build Verification**:
```bash
cargo test --workspace      # All tests pass
cargo clippy --workspace    # No warnings
cargo fmt --all             # Formatted
cargo build-sbf             # SBF build succeeds
```

**Documentation Requirements**:
- Each crate MUST have rustdoc comments
- Each example MUST have a README explaining what it demonstrates
- Public API changes MUST update relevant docs

## Governance

This constitution defines non-negotiable principles for mcpsol development. All contributions MUST comply.

**Amendment Process**:
1. Propose change with rationale
2. Demonstrate that change maintains or improves reliability, simplicity, and universality
3. Update all affected documentation and templates
4. Version bump according to semantic rules

**Compliance Review**:
- PRs MUST be checked against Core Principles
- Violations require explicit justification and constitution amendment
- Size limit violations are never acceptable without pagination

**Version**: 1.0.0 | **Ratified**: 2026-01-20 | **Last Amended**: 2026-01-20
