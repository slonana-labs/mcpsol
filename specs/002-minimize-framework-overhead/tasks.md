# Tasks: Minimize Framework Overhead

**Input**: Design documents from `/specs/002-minimize-framework-overhead/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Organization**: Tasks grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create benchmark baseline and prepare test infrastructure

- [X] T001 Create overhead benchmark test file at core/tests/overhead.rs
- [X] T002 [P] Add baseline CU measurements for current discriminator extraction in core/tests/overhead.rs
- [X] T003 [P] Add baseline CU measurements for current argument parsing in core/tests/overhead.rs
- [X] T004 [P] Add baseline CU measurements for current Context building in core/tests/overhead.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core utilities that MUST be complete before optimized code generation

**CRITICAL**: No user story work can begin until this phase is complete

- [X] T005 Create unsafe read helper module at sdk/src/read.rs with SAFETY-documented functions
- [X] T006 [P] Implement `read_u8_unchecked` in sdk/src/read.rs
- [X] T007 [P] Implement `read_u16_unchecked` in sdk/src/read.rs
- [X] T008 [P] Implement `read_u32_unchecked` in sdk/src/read.rs
- [X] T009 [P] Implement `read_u64_unchecked` in sdk/src/read.rs
- [X] T010 [P] Implement `read_i8_unchecked`, `read_i16_unchecked`, `read_i32_unchecked`, `read_i64_unchecked` in sdk/src/read.rs
- [X] T011 [P] Implement `read_bool_unchecked` in sdk/src/read.rs
- [X] T012 [P] Implement `read_pubkey_unchecked` in sdk/src/read.rs
- [X] T013 Export read module from sdk/src/lib.rs
- [X] T014 Create type-to-size mapping utility in macros/src/program.rs for compile-time offset calculation

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Minimize Transaction Costs (Priority: P1) MVP

**Goal**: Reduce framework overhead from ~230 CU to <50 CU for simple instructions

**Independent Test**: Run `cargo test --package mcpsol-core --test overhead -- --nocapture` and verify dispatcher overhead < 50 CU

### Implementation for User Story 1

- [X] T015 [US1] Add `get_type_size` function in macros/src/program.rs that returns byte size for known Rust types
- [X] T016 [US1] Add `calculate_expected_len` function in macros/src/program.rs that computes total instruction data length at compile time
- [X] T017 [US1] Modify `generate_dispatcher` in macros/src/program.rs to use unsafe discriminator read after bounds check
- [X] T018 [US1] Generate single `EXPECTED_LEN` const per instruction arm in macros/src/program.rs
- [X] T019 [US1] Replace `generate_arg_parsing` in macros/src/program.rs with compile-time offset calculation
- [X] T020 [US1] Generate unsafe reads using sdk/src/read.rs helpers with SAFETY comments in macros/src/program.rs
- [X] T021 [US1] Remove mutable `__offset` tracking from generated code in macros/src/program.rs
- [X] T022 [US1] Add debug_assert! for bounds verification in debug builds in macros/src/program.rs
- [X] T023 [US1] Update overhead benchmark to verify <50 CU total overhead in core/tests/overhead.rs

**Checkpoint**: At this point, User Story 1 should show <50 CU framework overhead

---

## Phase 4: User Story 2 - High-Frequency Optimization (Priority: P2)

**Goal**: Provide option to bypass Context wrapper entirely for maximum performance

**Independent Test**: Write instruction without Context, verify ~30 CU overhead (no Context building cost)

### Implementation for User Story 2

- [X] T024 [US2] Add `context = true/false` attribute parsing in macros/src/program.rs `extract_instructions` function
- [X] T025 [US2] Detect Context<T> in first parameter to auto-enable context mode in macros/src/program.rs
- [X] T026 [US2] Modify `generate_dispatcher` to skip Context building when context = false in macros/src/program.rs
- [X] T027 [US2] Generate direct handler call with (program_id, accounts, args...) when context = false in macros/src/program.rs
- [X] T028 [US2] Support accounts string attribute parsing for no-Context instructions in macros/src/program.rs
- [X] T029 [US2] Add overhead benchmark for no-Context path in core/tests/overhead.rs
- [X] T030 [US2] Update counter example to demonstrate both Context and no-Context patterns in examples/counter/src/lib.rs

**Checkpoint**: At this point, User Story 2 should show ~30 CU for no-Context instructions

---

## Phase 5: User Story 3 - Framework Transparency (Priority: P3)

**Goal**: Document CU costs and provide visibility into generated code

**Independent Test**: Documentation exists, `cargo expand` produces readable output, benchmark suite passes

### Implementation for User Story 3

- [X] T031 [P] [US3] Add CU cost table to README.md documenting overhead per component
- [X] T032 [P] [US3] Create docs/overhead.md with detailed CU breakdown and optimization rationale
- [X] T033 [P] [US3] Add `cargo expand` example to quickstart.md showing optimized generated code
- [X] T034 [US3] Add comprehensive benchmark assertions proving CU claims in core/tests/overhead.rs
- [X] T035 [US3] Document SAFETY invariants for all unsafe code in sdk/src/read.rs

**Checkpoint**: Documentation complete, all CU claims verified by benchmarks

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final validation and cleanup

- [X] T036 Run full test suite to verify backwards compatibility with existing examples
- [X] T037 [P] Verify examples/counter compiles and works with optimizations
- [X] T038 [P] Verify examples/vault compiles and works with optimizations
- [X] T039 [P] Verify examples/native-counter compiles and works with optimizations
- [X] T040 Run overhead benchmarks and record final CU measurements
- [X] T041 Update quickstart.md with migration scenarios if API changed

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational (T005-T014)
- **User Story 2 (Phase 4)**: Depends on User Story 1 completion (builds on optimized base)
- **User Story 3 (Phase 5)**: Can start after Phase 2, accelerates after US1/US2 complete
- **Polish (Phase 6)**: Depends on all user stories being complete

### Within Each User Story

- Foundation tasks MUST complete before implementation
- Implementation tasks are sequential (each builds on previous)
- Benchmarks validate each story's success criteria

### Parallel Opportunities

**Phase 1 (Setup):**
```
T002, T003, T004 can run in parallel (different benchmark sections)
```

**Phase 2 (Foundational):**
```
T006, T007, T008, T009, T010, T011, T012 can run in parallel (independent type handlers)
```

**Phase 5 (US3 Documentation):**
```
T031, T032, T033 can run in parallel (different doc files)
```

**Phase 6 (Polish):**
```
T037, T038, T039 can run in parallel (different example programs)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (baseline benchmarks)
2. Complete Phase 2: Foundational (read helpers)
3. Complete Phase 3: User Story 1 (optimized dispatcher)
4. **STOP and VALIDATE**: Verify <50 CU overhead
5. Ship optimization to users

### Incremental Delivery

1. Setup + Foundational: Foundation ready
2. Add User Story 1: Verify <50 CU (MVP!)
3. Add User Story 2: Verify ~30 CU no-Context path (Enhancement)
4. Add User Story 3: Documentation and transparency (Polish)

### Key Files Modified

| File | Changes |
|------|---------|
| `macros/src/program.rs` | Optimized dispatcher generation (T015-T028) |
| `sdk/src/read.rs` | NEW: Unsafe read helpers (T005-T012) |
| `sdk/src/lib.rs` | Export read module (T013) |
| `core/tests/overhead.rs` | NEW: Overhead benchmarks (T001-T004, T023, T029, T034) |
| `docs/overhead.md` | NEW: CU documentation (T032) |

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story
- All unsafe code MUST have SAFETY comments
- Benchmark verification is mandatory - no unsubstantiated CU claims
- Existing programs must continue to compile (backwards compatibility)
