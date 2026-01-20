# Feature Specification: Optimize Compute Unit Usage

**Feature Branch**: `001-optimize-cu`
**Created**: 2026-01-20
**Status**: Draft
**Input**: User description: "optimize cu usage but dont reduce schemas"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Program Developer Reduces CU Cost (Priority: P1)

A Solana program developer using mcpsol wants their `list_tools` instruction to consume fewer compute units so that users calling it pay less in transaction fees and the instruction is less likely to hit CU limits.

**Why this priority**: CU efficiency directly impacts transaction costs and success rates. Lower CU usage means cheaper discovery calls and higher reliability.

**Independent Test**: Can be verified by measuring CU consumption of `list_tools` before and after optimization on identical schema data.

**Acceptance Scenarios**:

1. **Given** a program with 4 tools in its schema, **When** `list_tools` is called, **Then** the CU consumption is measurably lower than the baseline implementation.
2. **Given** a program using the builder pattern at runtime, **When** `list_tools` is called multiple times, **Then** schema construction overhead occurs only once (cached).
3. **Given** a program using compile-time macros, **When** `list_tools` is called, **Then** no runtime JSON generation occurs (pre-computed constant).

---

### User Story 2 - AI Agent Discovers Large Program (Priority: P2)

An AI agent needs to discover the interface of a program with many tools. The agent iterates through paginated responses. Each page should be returned with minimal CU overhead.

**Why this priority**: Paginated discovery multiplies CU cost per page. Optimizing pagination directly reduces total discovery cost for complex programs.

**Independent Test**: Can be verified by measuring CU per paginated page and comparing total CU for full schema discovery.

**Acceptance Scenarios**:

1. **Given** a program with 10 tools, **When** an AI agent fetches all pages (cursor 0-9), **Then** total CU consumption is lower than baseline.
2. **Given** a paginated response, **When** the next page is requested, **Then** no redundant schema reconstruction occurs.

---

### User Story 3 - Maintainer Monitors CU Efficiency (Priority: P3)

A library maintainer wants visibility into CU consumption to ensure optimizations don't regress over time and to identify further optimization opportunities.

**Why this priority**: Without measurement, optimization efforts can't be validated. Benchmarks enable continuous improvement.

**Independent Test**: Can be verified by running benchmark suite and reviewing CU metrics.

**Acceptance Scenarios**:

1. **Given** the mcpsol test suite, **When** benchmarks are run, **Then** CU consumption metrics are reported for key operations.
2. **Given** a code change, **When** benchmarks are compared before/after, **Then** regressions are detectable.

---

### Edge Cases

- What happens when schema is at maximum 1024-byte limit? (Should still benefit from optimizations)
- How does CU scale with number of accounts/args per tool? (Linear scaling expected)
- What if lazy initialization structure is accessed concurrently? (Must be thread-safe)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST reduce CU consumption for `list_tools` instruction without removing any schema content
- **FR-002**: System MUST maintain identical JSON output (byte-for-byte where possible, semantically equivalent at minimum)
- **FR-003**: System MUST support compile-time schema generation via macros (zero runtime cost)
- **FR-004**: System MUST support lazy initialization for builder-pattern schemas (one-time construction cost)
- **FR-005**: System MUST avoid unnecessary string allocations during JSON generation
- **FR-006**: System MUST avoid redundant computation on repeated `list_tools` calls
- **FR-007**: System MUST provide CU benchmarks for measuring optimization effectiveness

### Key Entities

- **Schema Constant**: Pre-computed JSON bytes embedded at compile time for macro-based programs
- **Cached Schema**: Lazily-initialized schema stored in static memory for builder-pattern programs
- **JSON Generator**: Functions that serialize schema to JSON with minimal allocations

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `list_tools` CU consumption is reduced by at least 20% compared to baseline for typical 4-tool programs
- **SC-002**: Paginated `list_tools` per-page CU is reduced by at least 15% compared to baseline
- **SC-003**: Schema JSON output remains identical (no content reduction)
- **SC-004**: All existing tests continue to pass
- **SC-005**: Benchmark suite demonstrates measurable CU reduction with reproducible metrics

## Assumptions

- Baseline CU measurement will be established before optimization work begins
- "Typical" program has 3-5 tools with 2-4 accounts each
- Lazy initialization primitives are available in the target environment
- CU can be measured via Solana's compute budget or simulation logs
