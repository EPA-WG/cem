# CEM ML Library Plan

## Package Name

Use `cem-ml` for the parser/runtime library and `cem-ml-cli` for the command-line interface.

Rationale:

- The active parser/runtime implementation is Rust-first and already lives in `packages/cem_ml`.
- The CLI boundary already lives in `packages/cem_ml_cli`.
- It leaves room for future packages such as CEM fixtures, adapters, or npm/WASM wrappers without overloading the core
  crate.

## Initial Responsibility

`cem-ml` should own:

- Typed semantic document nodes for screens, regions, forms, fields, lists, assets, profiles, and messages.
- Parser helpers for canonical curly CEM-ML into a normalized DOM model, plus XML/HTML
  parity adapters that lower into the same model.
- Query helpers for semantic roles, state, validation messages, relationships, and labels.
- Validation reports for invalid state combinations, broken references, and missing accessible names.
- Schema-driven transform helpers from canonical and parity semantic fixtures into
  canonical CEM-ML snapshots and light-DOM custom-element markup.

`cem-ml-cli` should own:

- CLI argument parsing and process exit behavior.
- File input/output and report destination handling.
- Fixture validation and round-trip workflows.
- Planned CLI capabilities summarized in [`cem-ml-cli-contract.md`](./cem-ml-cli-contract.md).

## Non-Goals

- It does not own visual styling; generated token CSS remains in `@epa-wg/cem-theme`.
- It does not own component implementations; rendered custom elements remain in `@epa-wg/cem-components`.
- It does not create runtime client-side behavior for applications that are meant to stay declarative.
