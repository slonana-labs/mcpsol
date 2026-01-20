# Research: Optimize Compute Unit Usage

## Current Implementation Analysis

### Macro-based Programs (compile-time)

**Location**: `macros/src/mcp_gen.rs`, `macros/src/program.rs`

The `#[mcp_program]` macro currently generates schema JSON at compile time via `generate_schema_json()`. The generated JSON is embedded as a string constant in the program binary.

**Current behavior**:
- Schema JSON is built at compile time (good)
- `MCP_SCHEMA_JSON` constant holds the complete schema string
- `list_tools` handler calls `set_return_data(MCP_SCHEMA_JSON.as_bytes())` (single syscall)

**CU breakdown** (estimated):
- Instruction discriminator check: ~5 CU
- `set_return_data` syscall: ~100 CU (fixed cost)
- Total: ~105 CU

**Optimization opportunity**: Already optimal for macro-based programs. The schema is pre-computed at compile time with zero runtime overhead beyond the syscall.

### Builder-pattern Programs (runtime)

**Location**: `core/src/json.rs`, `examples/counter/src/lib.rs`

Programs using the builder pattern construct schemas at runtime via `McpSchemaBuilder` and `generate_paginated_schema_bytes()`.

**Current behavior**:
```rust
static SCHEMA: std::sync::OnceLock<McpSchema> = std::sync::OnceLock::new();

fn get_schema() -> &'static McpSchema {
    SCHEMA.get_or_init(build_schema)
}

// In list_tools handler:
let schema_bytes = generate_paginated_schema_bytes(get_schema(), cursor);
pinocchio::program::set_return_data(&schema_bytes);
```

**CU breakdown** (estimated for first call):
- OnceLock check: ~5 CU
- Schema construction (if first call): ~500-1000 CU
  - String allocations
  - Vec operations
  - Builder method calls
- JSON serialization: ~200-500 CU
  - String formatting
  - Character escaping
  - Buffer management
- `set_return_data` syscall: ~100 CU
- Total first call: ~800-1600 CU
- Total subsequent calls: ~400-700 CU (no schema construction)

**Optimization opportunities**:
1. **Cache serialized bytes, not just schema struct** - Avoid re-serializing on every call
2. **Pre-allocate JSON buffer** - Avoid repeated allocations during serialization
3. **Use byte slices instead of String** - Reduce allocation overhead

### JSON Serialization Analysis

**Location**: `core/src/json.rs`

Current `generate_paginated_schema_bytes()` implementation:
```rust
pub fn generate_paginated_schema_bytes(schema: &McpSchema, cursor: u8) -> Vec<u8> {
    generate_paginated_schema(schema, cursor).into_bytes()
}
```

Issues identified:
1. Creates new `String` on every call
2. Calls `.into_bytes()` which consumes the String (allocation transfer, not copy)
3. For paginated schemas, each page re-generates the header (`{"v":"...","name":"...","tools":[`)

## Optimization Strategies

### Strategy 1: Pre-compute Paginated Responses

**Decision**: Cache all paginated page bytes at initialization time

**Rationale**: For builder-pattern programs, compute the byte representation of each page once and cache it. Subsequent calls simply copy from the cached slice.

**Alternatives considered**:
- Cache only schema struct (current approach) - Still requires serialization overhead per call
- No caching - Worst case, but simplest

**Implementation**:
```rust
struct CachedSchema {
    pages: Vec<Vec<u8>>,  // Pre-serialized pages
}

static CACHED_SCHEMA: OnceLock<CachedSchema> = OnceLock::new();

fn get_cached_page(cursor: u8) -> &'static [u8] {
    let cached = CACHED_SCHEMA.get_or_init(|| {
        let schema = build_schema();
        let mut pages = Vec::new();
        for i in 0..=schema.tools.len() {
            pages.push(generate_paginated_schema_bytes(&schema, i as u8));
        }
        CachedSchema { pages }
    });
    cached.pages.get(cursor as usize).map(|v| v.as_slice()).unwrap_or(&[])
}
```

**Expected CU reduction**: 200-400 CU per call after first call (~30-50% reduction)

### Strategy 2: Optimize JSON Generation

**Decision**: Use pre-sized buffers and avoid intermediate allocations

**Rationale**: The current implementation creates multiple intermediate Strings. Pre-sizing the buffer based on `estimate_schema_size()` eliminates reallocation.

**Alternatives considered**:
- Use `serde_json` - Heavier dependency, more CU
- Use raw byte writes - More complex, marginal gains

**Implementation**:
```rust
pub fn generate_paginated_schema(schema: &McpSchema, cursor: u8) -> String {
    let estimated_size = estimate_single_tool_size(&schema.tools.get(cursor as usize));
    let mut json = String::with_capacity(estimated_size + 100);  // +100 for envelope
    // ... rest of generation with pre-sized buffer
}
```

**Expected CU reduction**: 50-100 CU per call (~10-15% reduction)

### Strategy 3: Compile-time Schema Bytes for Macros

**Decision**: Generate schema as `&'static [u8]` instead of `&'static str`

**Rationale**: Eliminates the `.as_bytes()` call in `set_return_data()`.

**Alternatives considered**:
- Keep as str - Marginal overhead from as_bytes()
- Generate both - Increases binary size unnecessarily

**Implementation**:
```rust
// In macro-generated code:
const MCP_SCHEMA_BYTES: &[u8] = b"{\"v\":\"2024-11-05\",...}";

// In list_tools handler:
pinocchio::program::set_return_data(MCP_SCHEMA_BYTES);
```

**Expected CU reduction**: 5-10 CU (~5% reduction)

### Strategy 4: Add CU Benchmarks

**Decision**: Create benchmark suite measuring CU consumption

**Rationale**: Cannot verify optimization effectiveness without measurement.

**Implementation**:
- Add `benches/` directory with Solana program tests
- Use `solana-program-test` to measure CU via logs
- Create comparison benchmarks: before/after optimization
- Include in CI for regression detection

## Summary of Decisions

| Strategy | Decision | Expected Impact | Priority |
|----------|----------|-----------------|----------|
| Pre-compute pages | Cache serialized bytes | 30-50% reduction | P1 |
| Optimize JSON generation | Pre-sized buffers | 10-15% reduction | P2 |
| Compile-time bytes | Static byte slice | 5% reduction | P3 |
| CU benchmarks | Add benchmark suite | Measurement | P1 (enabler) |

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Increased memory for cached pages | Pages are <1KB each; negligible for typical programs |
| OnceLock not available in no_std | Use `core::sync::OnceLock` (Rust 1.70+) or custom spin-lock |
| Benchmark accuracy | Use multiple runs, controlled environment |

## Next Steps

1. Establish baseline CU measurements
2. Implement Strategy 1 (page caching)
3. Measure improvement
4. Implement Strategy 2 (buffer optimization)
5. Measure cumulative improvement
6. Implement Strategy 3 (compile-time bytes)
7. Final measurement and documentation
