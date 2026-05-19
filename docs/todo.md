# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).

## Phase 2 - Schema-Defined Parser And Document Runtime (`@epa-wg/cem-ml` / `@epa-wg/cem-ml-cli`)

Acceptance criteria: [`cem-ml-ac.md`](cem-ml-ac.md). Plan: [`cem-ml-library-plan.md`](cem-ml-library-plan.md).
Component vocabulary: [`component-mvp.md`](component-mvp.md). Research input:
[`../parsing-algorithms-research.md`](../parsing-algorithms-research.md).

### Remaining Execution Items

- [x] Replace the XML tokenizer profile stub in `packages/cem_ml/src/tokenizer/xml.rs` with the Phase 11 XML 1.0
      tokenizer and wire XML parity fixtures through the same event-normalizer, schema, AST, validation, and transform
      checks used by the canonical CEM-ML and HTML parity fixtures.
- [x] Complete the reverse HTML/XML -> CEM-ML projection path described in
      [`../packages/cem_ml/docs/cross-surface-conversion.md`](../packages/cem_ml/docs/cross-surface-conversion.md),
      including byte-stable canonical formatter expectations and source-map preservation across the content-type
      transform boundary.
- [x] Build the tree-sitter scanner/parser and add the planned parity check that every `examples/cem-ml/*.cem` fixture
      parses equivalently in Rust and tree-sitter.
- [x] Add a dedicated relaxed-boundary lint once the parser surfaces the structural distinction unambiguously instead
      of relying on tokenizer diagnostics such as `cem.tokenizer.unterminated_node` / `bare_brace_text`.
- [x] Design and implement the public observability API for `onParseEvent`, `onValidate`, and `onTransform`, including
      Rust/WASM types, event payload schema, CLI report projection, and runnable projection tests.
- [x] Design and implement the plugin runtime for AC-PL-* descriptors, chaining, sandboxing, source-map stitching,
      budgets, lifecycle hooks, and verification fixtures.
- [ ] Add engine scheduling design and implementation for AC-A-4..AC-A-7 / AC-O-2: worker pools, bounded queues,
      external-I/O queue handling, cancellation, AbortSignal support, deterministic trace shape, and resource defaults.
- [ ] Add scoped registry data structures and lookup algorithms for AC-R-*: inheritance, collision detection, and
      DCE/custom-element integration contracts.
- [ ] Add the performance benchmark harness and policy for AC-N-*: budget ownership, CI tolerance rules, and
      memory-limit proof fixtures.
- [ ] Add compatibility and distribution gates for AC-C-*: browser, Node, Rust, WASM, package artifacts, and release
      checks.
- [ ] Create `cem-ql-stack-design.md` and `cem-ql-stack-design-impl.md` before CEM-QL work moves beyond exploratory
      tests. Cover grammar, parser architecture, evaluator IR, stdlib module layout, type checker, cost model, and
      binary artifact layout.
