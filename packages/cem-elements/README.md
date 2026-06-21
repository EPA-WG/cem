# @epa-wg/cem-elements

Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>

`@epa-wg/cem-elements` provides the `<cem-element>` browser substrate for declarative light-DOM custom elements. It is
the Phase 3.1 runtime gate before the project moves into Edge/SSR support and later `@epa-wg/custom-element` adoption.

## Production-Ready Trigger

The package is considered browser-substrate production-ready when this command passes:

```bash
yarn nx run cem-elements:verify
```

That aggregate gate runs:

- `cem_ml_cli:validate-fixtures`
- `cem_ml_cli:e2e`
- `cem_ml:bench`
- `cem-elements:verify-substrate`
- `cem-elements:verify-legacy-fixtures`
- `cem-elements:verify-material-fixtures`
- `cem-elements:test:unit`
- `cem-elements:test`

The gate covers file-backed legacy fixtures in `tests/parity/legacy/`, material parity fixtures in
`tests/parity/material/`, substrate CEM fixtures in `../../examples/cem-elements/`, unit coverage, and Storybook
browser parity stories.

## Fixture Locations

- `tests/parity/legacy/` — legacy `<custom-element>` behavior mapped to CEM-ML/browser substrate fixtures.
- `tests/parity/material/` — the eight material reference components: `action`, `autocomplete`, `badge`, `dropdown`,
  `icon`, `icon-link`, `input`, and `menu`.
- `docs/legacy-parity-inventory.md` — legacy behavior support matrix and bridge/adoption deferrals.
- `docs/material-parity-inventory.md` — material feature support matrix and production-gate caveats.

## Handoff Condition

Passing `yarn nx run cem-elements:verify` means the `<cem-element>` browser substrate is ready for the Phase 3.5
Edge/SSR follow-up. It does not mean the legacy `@epa-wg/custom-element` package has adopted this implementation.
That adoption remains a later Phase 3.6 handoff after Edge/SSR boundaries are in place.

## Known Deferrals

- Full legacy XPath and broad XSLT behavior remain bridge/adoption work. The supported migration path is CEM-ML plus
  CEM-QL over structured `datadom.*` records.
- Scoped template styles intentionally render as page-global light-DOM styles for this gate; selector containment is a
  separate bridge/adoption primitive.
- Host-owned resolution remains explicit for bare module specifiers and external resource policy hooks.

## Building

Run `yarn nx run cem-elements:build` to build the library.

## Testing

Run `yarn nx run cem-elements:test` to execute the runtime stories through Storybook Test.

Run `yarn nx run cem-elements:storybook` to open the interactive Storybook runtime fixture surface.
