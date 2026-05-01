# Todo

This file tracks remaining execution tasks only. Completed token-export, Style Dictionary, native-output, and adapter
example work has been removed from this checklist; product/module sequencing lives in [`../roadmap.md`](../roadmap.md).

## Token And Figma Validation

- [x] Validate the native Figma library variables in the CEM UI Kit and record the result in the report or a fixture
      note.
- [x] Add native Figma library screenshots to `examples/figma/README.md`.
- [x] Validate visual parity against a web reference with manual screenshot comparison.

## Full Token Pipeline Smoke

- [x] Change one token in `packages/cem-theme/src/lib/tokens/cem-colors.md`.
- [ ] Run `yarn build`. Current blocker: Nx plugin-worker startup fails before targets run; targeted theme and
      token-platform builds passed. See `docs/token-pipeline-smoke.md`.
- [x] Verify propagation through CSS, canonical JSON, TypeScript metadata, Figma files, iOS Swift, Android XML, and any
      Style Dictionary outputs.
- [x] Diff all reports; only the changed token should appear unless the change intentionally affects derived aliases.

## Roadmap Follow-Ups

- [x] Wire `roadmap.md`, `docs/todo.md`, package docs, and token export docs from the root README.
- [x] Add a docs index under `docs/`.
- [x] Decide the package name for the XML/HTML/XSLT DOM library.
- [x] Create the first semantic fixture set: login, registration, profile, assets list, and message thread.
- [x] Define the component MVP list and state matrix.
- [x] Add a Figma UI Kit plan that maps components to generated token variables.
