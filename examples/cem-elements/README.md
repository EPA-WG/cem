# `<cem-element>` substrate fixtures

Canonical CEM-ML templates that exercise the `@epa-wg/cem-elements` substrate authoring
vocabulary (`cem:if`/`cem:choose` conditionals, `<slot>`, `<attribute>`, `<data>`/`<option>`
payloads, `module-url`, and `{$datadom…}` host expressions with `??`). They are the C2.6
verification-gate fixtures and Phase 3.1 production-gate material performance fixtures
(`docs/todo.md` §3.1).

## How they ride the verification gate

This vocabulary is **not** Tier-A HTML/SVG, and its host bindings (`datadom`, declared
attributes) resolve only at render time, so the semantic `cem-ml validate` gate intentionally
rejects it. These fixtures instead ride the **structural** leg:
`nx run cem-elements:verify-substrate` drives each fixture through the real `cem-ml` CLI
`convert cem->cem` projection and asserts (1) structural success — it tokenizes, builds an AST,
and serializes back to canonical CEM-ML — and (2) roundtrip idempotence (the canonical form is
stable), the same property `cem_ml_cli:e2e` checks for base fixtures.

Semantic/render correctness of the same constructs is covered by the `cem_ql` render tests
(`packages/cem_ql/tests/template_render.rs`) and the `@epa-wg/cem-elements` Storybook parity
stories.

The `material-*.cem` fixtures also ride the AC-N-1 benchmark suite through
`nx run cem_ml:bench`. They measure first-paint parser/schema-machine/AST-builder budget for
the material parity substrate vocabulary; render-time host bindings and module-map resolution
remain covered by Storybook parity stories.

## The composite gate

`nx run cem-elements:verify` composes the full C2.6 gate:

- `cem_ml_cli:validate-fixtures` — base canonical CEM-ML + HTML parity fixtures,
- `cem_ml_cli:e2e` — validate + roundtrip + cross-surface convert,
- `cem-elements:verify-substrate` — the substrate fixtures here, and
- `cem-elements:test` — the Storybook parity stories.

The gate must be green before the temporary C1.5 TypeScript CEM-ML fallback adapter is retired.

## Fixtures

| Fixture                  | Constructs                                                                 |
| ------------------------ | -------------------------------------------------------------------------- |
| `conditionals.cem`       | `cem:if` / `cem:choose` / `cem:when` / `cem:otherwise` with `@test` selection |
| `data-document.cem`      | `<attribute>` defaults, `{$datadom…}` selection, `??` coalescing, AVT       |
| `material-parity.cem`    | eight material component substrate shapes plus slots, slices, payloads, and `module-url` |
| `slots-and-payload.cem`  | named + default `<slot>` with fallback, `<data>` / `<option>` payloads      |
