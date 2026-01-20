# Philosophy

## The Problem

Solana programs are black boxes. Given only a program ID, there is no standard way to discover what instructions it supports, what accounts they require, or what arguments they expect.

The ecosystem has converged on IDL (Interface Definition Language) files as the solution. Anchor generates these automatically. But IDLs have fundamental problems:

**Distribution.** IDL files must be hosted somewhere - npm, GitHub, IPFS, a CDN. This creates external dependencies. If the hosting goes down, clients can't discover the interface.

**Synchronization.** The IDL is generated at build time but distributed separately from the program. Nothing enforces that the IDL matches the deployed code. Version mismatches cause silent failures.

**Size.** Anchor IDLs are verbose. A simple counter program produces 50+ KB of JSON. Complex programs generate hundreds of kilobytes.

**Framework lock-in.** IDL formats are framework-specific. Anchor IDLs require Anchor-aware parsers. Native programs have no standard format at all.

## The Solution

Embed the interface schema directly in the program binary.

Programs expose a `list_tools` instruction that returns their schema via `set_return_data`. Clients discover interfaces through transaction simulation - the same RPC infrastructure they already use.

This inverts the dependency:

```
Traditional:  Program <-- IDL file <-- Client
MCP:          Program --> Client (via simulation)
```

The schema is compiled into the program. It cannot drift from the implementation because they are the same artifact.

## Design Principles

### On-chain truth

The schema lives in the program binary. There is exactly one source of truth, and it's the deployed code. No synchronization problems. No version mismatches.

### Zero external dependencies

Discovery requires only an RPC endpoint - the same infrastructure needed to submit transactions. No file servers. No IPFS. No registries. If you can interact with the program, you can discover its interface.

### Compute efficiency

Schema generation happens at compile time via proc-macros. Runtime cost is a single `set_return_data` call. No parsing, no allocation, no computation.

### Size constraints

Schemas are designed for Solana's 1024-byte `return_data` limit. This forces brevity:

- Single-character field keys (`n` not `name`)
- Type abbreviations (`int` not `integer`)
- Account flags as suffixes (`_sw` not `{"signer": true, "writable": true}`)
- Optional fields omitted when empty

Complex programs use pagination - each `list_tools` call returns one tool, with a cursor for the next.

### Framework agnostic

The protocol is the interface. Programs can use Pinocchio, Anchor, or raw `solana-program`. Clients parse the same JSON regardless of how it was generated.

## Trade-offs

### What we give up

**Type richness.** The compact format can't express full type information. Enum variants, struct fields, and complex nested types require supplementary documentation.

**Dynamic schemas.** The schema is fixed at compile time. Programs needing runtime-configurable interfaces need alternative approaches.

**Large interfaces.** Programs with many instructions may hit size limits even with pagination. This naturally pressures toward smaller, focused programs - arguably a feature.

### What we gain

**Reliability.** The schema is always correct because it's compiled from the same source as the program logic.

**Simplicity.** No build pipelines for IDL generation. No deployment workflows for schema files. No version management.

**Universality.** Any client that can simulate transactions can discover any MCP-enabled program. No framework-specific tooling required.

## Naming

MCP stands for Model Context Protocol. The name reflects the primary use case: enabling AI models to understand and interact with Solana programs without human-written integration code.

An AI agent given a program ID can:
1. Simulate `list_tools` to get the schema
2. Parse the JSON to understand available instructions
3. Build and submit transactions

No documentation required. No API wrappers. The program describes itself.
