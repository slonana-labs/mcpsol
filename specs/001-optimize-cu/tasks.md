# Tasks: Optimize Compute Unit Usage

**Input**: Design documents from `/specs/001-optimize-cu/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md

**Tests**: Tests are included as they directly support the feature's success criteria (CU measurement and benchmarking).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Rust workspace** structure per plan.md:
  - `core/src/` - Core library (json.rs, schema.rs)
  - `macros/src/` - Proc macros (mcp_gen.rs, program.rs)
  - `sdk/src/` - SDK library
  - `examples/` - Example programs
  - `benches/` - CU benchmarks (new)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish baseline measurements and benchmark infrastructure

- [x] T001 Create benchmark directory structure at benches/
- [x] T002 [P] Add solana-program-test dev-dependency to core/Cargo.toml
- [x] T003 [P] Create baseline CU measurement test in benches/baseline.rs

**Checkpoint**: Benchmark infrastructure ready, baseline CU measurements established

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**CRITICAL**: No user story work can begin until this phase is complete

- [x] T004 Document baseline CU measurements for 4-tool schema in benches/README.md
- [x] T005 Add CachedSchemaPages struct definition to core/src/schema.rs
- [x] T006 Add estimate_single_tool_size() helper to core/src/json.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Program Developer Reduces CU Cost (Priority: P1)

**Goal**: Reduce `list_tools` CU consumption by 20%+ for builder-pattern and macro-based programs

**Independent Test**: Measure CU before/after on identical 4-tool schema; verify 20%+ reduction

### Implementation for User Story 1

- [x] T007 [US1] Add CachedSchemaPages::new() constructor in core/src/schema.rs
- [x] T008 [US1] Add CachedSchemaPages::get_page() method in core/src/schema.rs
- [x] T009 [US1] Implement pre-sized buffer allocation in generate_paginated_schema() in core/src/json.rs
- [x] T010 [US1] Update generate_compact_schema() with pre-sized buffer in core/src/json.rs
- [x] T011 [US1] Update macro to generate MCP_SCHEMA_BYTES as &[u8] in macros/src/mcp_gen.rs
- [x] T012 [US1] Update macro list_tools handler to use byte slice in macros/src/program.rs
- [x] T013 [US1] Add test verifying identical JSON output before/after in core/src/json.rs
- [x] T014 [US1] Update examples/counter to use CachedSchemaPages in examples/counter/src/lib.rs

**Checkpoint**: User Story 1 complete - CU reduction verified for typical programs

---

## Phase 4: User Story 2 - AI Agent Discovers Large Program (Priority: P2)

**Goal**: Optimize paginated discovery for programs with 10+ tools

**Independent Test**: Measure total CU for paginated discovery (10 pages); verify 15%+ reduction per page

### Implementation for User Story 2

- [x] T015 [US2] Add page pre-computation in CachedSchemaPages initialization in core/src/schema.rs
- [x] T016 [US2] Optimize generate_paginated_schema() to avoid header regeneration in core/src/json.rs
- [x] T017 [US2] Update examples/vault to demonstrate pagination optimization in examples/vault/src/lib.rs
- [x] T018 [US2] Add pagination benchmark test in benches/pagination.rs

**Checkpoint**: User Story 2 complete - paginated discovery optimized

---

## Phase 5: User Story 3 - Maintainer Monitors CU Efficiency (Priority: P3)

**Goal**: Provide benchmark suite for CU measurement and regression detection

**Independent Test**: Run benchmark suite; verify CU metrics are reported with reproducible results

### Implementation for User Story 3

- [x] T019 [P] [US3] Create comprehensive CU benchmark suite in benches/cu_benchmarks.rs
- [x] T020 [P] [US3] Add benchmark comparison script in benches/compare.sh
- [x] T021 [US3] Document benchmark usage in benches/README.md
- [x] T022 [US3] Add CI integration example for regression detection in .github/workflows/ (if exists) or benches/README.md

**Checkpoint**: User Story 3 complete - benchmark suite available for maintainers

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Finalization, documentation, and validation

- [x] T023 [P] Update docs/architecture.md with CU optimization details
- [x] T024 [P] Update examples/README.md with CU-optimized patterns
- [x] T025 Run cargo test --workspace to verify all existing tests pass
- [x] T026 Run cargo clippy --workspace to verify no warnings
- [x] T027 Validate quickstart.md scenarios work end-to-end
- [x] T028 Final CU measurement comparison: document before/after in specs/001-optimize-cu/results.md

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Phase 2 - core optimization work
- **User Story 2 (Phase 4)**: Depends on Phase 2; benefits from Phase 3 work
- **User Story 3 (Phase 5)**: Depends on Phase 2; can run parallel to Phases 3-4
- **Polish (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Builds on US1 patterns but independently testable
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Fully independent of US1/US2

### Within Each User Story

- Core library changes before example updates
- Schema changes before JSON generation changes
- Macro changes are independent of core changes (can parallelize)

### Parallel Opportunities

- T002, T003 can run in parallel (different files)
- T019, T020 can run in parallel (different files)
- T023, T024 can run in parallel (different files)
- User Stories 2 and 3 can run in parallel after US1 core changes land

---

## Parallel Example: User Story 1

```bash
# Phase 3 parallelization opportunities:

# Group 1: Core library changes (sequential within group)
Task: T007 "Add CachedSchemaPages::new() constructor in core/src/schema.rs"
Task: T008 "Add CachedSchemaPages::get_page() method in core/src/schema.rs"

# Group 2: JSON optimization (sequential within group, parallel to Group 3)
Task: T009 "Implement pre-sized buffer allocation in generate_paginated_schema()"
Task: T010 "Update generate_compact_schema() with pre-sized buffer"

# Group 3: Macro updates (parallel to Group 2)
Task: T011 "Update macro to generate MCP_SCHEMA_BYTES as &[u8]"
Task: T012 "Update macro list_tools handler to use byte slice"

# After Groups 1-3 complete:
Task: T013 "Add test verifying identical JSON output"
Task: T014 "Update examples/counter to use CachedSchemaPages"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (baseline measurements)
2. Complete Phase 2: Foundational (CachedSchemaPages struct)
3. Complete Phase 3: User Story 1 (core optimizations)
4. **STOP and VALIDATE**: Run benchmarks, verify 20%+ CU reduction
5. Merge/deploy if target met

### Incremental Delivery

1. Setup + Foundational → Benchmark infrastructure ready
2. User Story 1 → 20%+ CU reduction achieved → MVP complete
3. User Story 2 → Pagination optimized → Enhanced for large programs
4. User Story 3 → Benchmark suite → Maintainability improved
5. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (core + macros)
   - Developer B: User Story 3 (benchmarks) - can start immediately
3. After US1 core lands:
   - Developer A or B: User Story 2 (pagination)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- All changes must maintain identical JSON output (FR-002)
- CU reduction targets: 20% overall, 15% per paginated page
- Commit after each task or logical group
- Run `cargo test --workspace` frequently to catch regressions
