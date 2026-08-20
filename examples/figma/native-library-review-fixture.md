# Native Figma Library Review Fixture

This fixture governs the manual refresh checkpoint for the native `CEM Tokens`
collection. `native-library-review.json` is deliberately `pending` until a
reviewer opens the canonical CEM UI Kit. Repository generation is not evidence
that the external library changed.

## Review procedure

1. Run `yarn nx run @epa-wg/cem-theme:verify:figma-native-review`. Keep the
   generated JSON/Markdown report open while reviewing the library.
2. Open the canonical CEM UI Kit and record its starting revision in
   `refresh.startingRevision` before importing or editing anything. Record the
   review start date in `refresh.startedAt`.
3. Confirm the collection is exactly `CEM Tokens` and its modes, in order, are
   `Light`, `Dark`, `Contrast Light`, `Contrast Dark`, and `Native`. Record them
   in `refresh.confirmedCollection` and `refresh.confirmedModes`, set the status
   to `started`, leave the import/review result fields null, and rerun the gate.
   This is the evidence required to close the starting-revision checkpoint.
4. Import `cem-light.tokens.json`, `cem-dark.tokens.json`,
   `cem-contrast-light.tokens.json`, `cem-contrast-dark.tokens.json`, and
   `cem-native.tokens.json` into their matching modes. Do not import the
   aggregate or debug token files.
5. Confirm that the live variable count and COLOR/FLOAT/STRING totals match the
   generated report, no mode value is missing, and native alias values are
   present. Record live locators or screenshots that another reviewer can find.
6. Set `refresh.status` to `reviewed`, record the completion date and reviewed
   revision, fill every live result field, and rerun the verifier. Only then may
   the parent Phase 5 checkpoint be marked complete.

## Deliberate rejection cases

The review fails if a pending record claims a starting or reviewed revision; a
reviewed record omits either revision; the collection, mode order, imported mode
files, variable count, or type totals differ from generated artifacts; any mode
value is missing; native aliases or evidence locators are absent; the historical
review no longer agrees with `examples/figma/README.md`; or the record claims a
live refresh based only on repository generation.
