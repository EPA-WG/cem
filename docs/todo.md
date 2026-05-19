# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).

## Phase 2 - Schema-Defined Parser And Document Runtime (`@epa-wg/cem-ml` / `@epa-wg/cem-ml-cli`)

Acceptance criteria: [`cem-ml-ac.md`](cem-ml-ac.md). Plan: [`cem-ml-library-plan.md`](cem-ml-library-plan.md).
Component vocabulary: [`component-mvp.md`](component-mvp.md). Research input:
[`../parsing-algorithms-research.md`](../parsing-algorithms-research.md).

### Remaining Execution Items

- [ ] Add the performance benchmark harness and policy for AC-N-*: budget ownership, CI tolerance rules, and
      memory-limit proof fixtures.
- [ ] Add compatibility and distribution gates for AC-C-*: browser, Node, Rust, WASM, package artifacts, and release
      checks.
- [ ] Create `cem-ql-stack-design.md` and `cem-ql-stack-design-impl.md` before CEM-QL work moves beyond exploratory
      tests. Cover grammar, parser architecture, evaluator IR, stdlib module layout, type checker, cost model, and
      binary artifact layout.
