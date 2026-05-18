# Schema-Scoping Fixtures

Per `docs/cem-ml-ac.md` §AC-F-2 and
`packages/cem_ml/docs/cross-surface-conversion.md` §5/§9. Each fixture
exercises one schema-scoping form covered by the
`packages/cem_ml/src/schema/scoping.rs` module and the corresponding
schema-machine integration tests in
`packages/cem_ml/tests/schema_scoping_fixtures.rs`.

| Fixture                                | Form                                                    | Expected outcome |
| -------------------------------------- | ------------------------------------------------------- | ---------------- |
| `inline-declaration.cem`               | `{cem:schema @cem:name="..." \| body}` inline           | Declaration resolvable in the parent scope's descendants. |
| `wrapping-switch.cem`                  | `{cem:schema @src="..." \| body}` element form          | Active schema switches inside the body; parent unaffected. |
| `select-switch.cem`                    | `{cem:schema @select="..." \| body}` cem-ql variant     | Active source recorded as `SchemaSource::Select`. |
| `host-node-switch.cem`                 | `{element @cem:schema-src="..." \| body}` host form     | Active schema switches on the host element only. |
| `src-select-exclusivity.cem`           | Element form with both `@src` and `@select`             | Emits `cem.schema.scoping.exclusive_src_select`. |
| `host-src-select-exclusivity.cem`      | Host form with both `@cem:schema-src` and `@cem:schema-select` | Emits `cem.schema.scoping.exclusive_src_select`. |
| `name-shadowing.cem`                   | Nested `cem:name="X"` declarations                      | Outer references resolve to outer; inner references resolve to inner. |
