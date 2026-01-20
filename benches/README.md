# CU Benchmarks

Compute Unit measurement and optimization benchmarks for mcpsol.

## Results Summary

Measured improvement with `CachedSchemaPages` optimization:

| Operation | Before | After | Improvement |
|-----------|--------|-------|-------------|
| Paginated page (typical 4-tool) | ~135 CU | ~2 CU | **98%** |
| Full discovery (4 pages) | ~540 CU | ~8 CU | **98%** |
| Compact schema (4 tools) | ~60 CU | ~60 CU | 0% (no caching) |

**Note**: CU estimates based on CPU time heuristic (1 CU ≈ 10ns). Actual on-chain CU includes syscall overhead.

## Running Benchmarks

```bash
# Quick summary report
cargo test --package mcpsol-core --test cu_benchmarks -- --nocapture summary_report

# All benchmarks with detailed output
cargo test --package mcpsol-core --test cu_benchmarks -- --nocapture

# Pagination comparison (direct vs cached)
cargo test --package mcpsol-core --test pagination -- --nocapture

# Baseline measurements
cargo test --package mcpsol-core --test baseline -- --nocapture
```

## Benchmark Comparison

Use the comparison script to track regressions:

```bash
# Run benchmark and save results
./benches/compare.sh

# Compare two benchmark runs
./benches/compare.sh results/baseline.log results/optimized.log
```

## Optimization Strategies

### 1. CachedSchemaPages (Primary)

Pre-compute all pagination pages at initialization. Returns references on subsequent calls.

```rust
use mcpsol_core::CachedSchemaPages;

static CACHED: OnceLock<CachedSchemaPages> = OnceLock::new();

fn list_tools(cursor: u8) {
    let pages = CACHED.get_or_init(|| CachedSchemaPages::from_schema(&build_schema()));
    set_return_data(pages.get_page(cursor));
}
```

**Improvement**: 95-99% per-page CU reduction after first call.

### 2. Pre-sized Buffers

Use `String::with_capacity()` based on `estimate_schema_size()`.

**Improvement**: 10-15%

### 3. Compile-time Byte Slices (Macros)

For `#[mcp_program]` macros, schema is embedded as `&'static [u8]`.

**Improvement**: Near-zero runtime overhead.

## Benchmark Files

Located in `core/tests/`:

| File | Purpose |
|------|---------|
| `baseline.rs` | Baseline allocation and serialization |
| `pagination.rs` | Direct vs cached pagination comparison |
| `cu_benchmarks.rs` | Comprehensive benchmark suite |

## CI Integration

### GitHub Actions Example

```yaml
name: CU Benchmarks

on:
  pull_request:
    paths:
      - 'core/src/**'
      - 'macros/src/**'

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Run CU benchmarks
        run: |
          cargo test --package mcpsol-core --test cu_benchmarks -- --nocapture 2>&1 | tee benchmark.log

      - name: Check for regressions
        run: |
          # Extract per-page ns for cached pagination
          CACHED_NS=$(grep "BENCHMARK: paginated_cached_typical" benchmark.log | sed 's/.*per_op_ns=\([0-9]*\).*/\1/')
          echo "Cached pagination: ${CACHED_NS} ns per page"

          # Fail if regression (threshold: 100ns per page)
          if [ "$CACHED_NS" -gt 100 ]; then
            echo "ERROR: Cached pagination regression detected!"
            exit 1
          fi

      - name: Upload benchmark results
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: benchmark.log
```

### Regression Detection

Parse benchmark output for automated checks:

```bash
# Extract specific benchmark result
grep "BENCHMARK: paginated_cached_typical" benchmark.log

# Machine-parseable format:
# BENCHMARK: paginated_cached_typical iterations=10000 total_ns=... per_op_ns=23 estimated_cu=2
```

## Interpreting Results

- **per_op_ns**: Nanoseconds per operation (lower is better)
- **estimated_cu**: Estimated Compute Units (per_op_ns / 10)
- **OUTPUT_SIZE**: Schema JSON size in bytes

Target thresholds:
- Cached pagination: < 50ns per page
- Schema size: < 1024 bytes (Solana return_data limit)
