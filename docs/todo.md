# Todo

This file tracks remaining execution tasks only. Completed token-export, Style Dictionary, native-output, and adapter
example work has been removed from this checklist; product/module sequencing lives in [`../roadmap.md`](../roadmap.md).

## Token And Figma Validation

- [ ] Validate Tokens Studio pull into a Figma test collection and record the result in the report or a fixture note.
- [ ] Add Tokens Studio pull-only screenshots to `examples/figma/README.md`.
- [ ] Validate visual parity against a web reference with manual screenshot comparison.

## Full Token Pipeline Smoke

- [ ] Change one token in `packages/cem-theme/src/lib/tokens/cem-colors.md`.
- [ ] Run `yarn build`.
- [ ] Verify propagation through CSS, canonical JSON, TypeScript metadata, Figma files, iOS Swift, Android XML, and any
      Style Dictionary outputs.
- [ ] Diff all reports; only the changed token should appear unless the change intentionally affects derived aliases.

## Roadmap Follow-Ups

- [ ] Wire `roadmap.md`, `docs/todo.md`, package docs, and token export docs from the root README.
- [ ] Add a docs index under `docs/`.
- [ ] Decide the package name for the XML/HTML/XSLT DOM library.
- [ ] Create the first semantic fixture set: login, registration, profile, assets list, and message thread.
- [ ] Define the component MVP list and state matrix.
- [ ] Add a Figma UI Kit plan that maps components to generated token variables.
