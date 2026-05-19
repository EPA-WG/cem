# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).

## Phase 2 - Schema-Defined Parser And Document Runtime (`@epa-wg/cem-ml` / `@epa-wg/cem-ml-cli`)

Acceptance criteria: [`cem-ml-ac.md`](cem-ml-ac.md). Plan: [`cem-ml-library-plan.md`](cem-ml-library-plan.md).
Component vocabulary: [`component-mvp.md`](component-mvp.md). Research input:
[`../parsing-algorithms-research.md`](../parsing-algorithms-research.md).

### Remaining Execution Items

_No outstanding Phase 2 execution items. Phase-2 follow-up work is tracked through the design-document AC-alignment
appendices (`cem-ml-stack-design.md §21`, `cem-ql-stack-design.md §21`)._

### Recently Closed

- AC-N-* perf benchmark harness and policy. Budget ownership in `cem_ml::benchmark::BenchmarkBudget`, CI tolerance via
  `CEM_ML_PERF_TOLERANCE`, 10 MB and depth-200 proof fixtures in `packages/cem_ml/tests/perf_budgets.rs`, Nx entry
  point `yarn nx run cem_ml:bench`. Documented in `cem-ml-stack-design.md §17`.
- AC-C-* compatibility / distribution gates. Support matrix, crate surface, CLI boundary, and release checks documented
  in `cem-ml-stack-design.md §18`.
- CEM-QL stack design. `cem-ql-stack-design.md` (high-level: pipeline layers, grammar, evaluator IR, type system,
  stdlib module layout, cost model, binary artifact layout) and `cem-ql-stack-design-impl.md` (concrete Rust module
  map, surface AST, IR shapes, diagnostic table, stdlib function tables).
