# @epa-wg/cem-elements

Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>

`@epa-wg/cem-elements` provides the `<cem-element>` browser substrate for declarative light-DOM custom elements. It is
the Phase 3.1 runtime gate before the project moves into Edge/SSR support and later `@epa-wg/custom-element` adoption.

External declaration loading through `src="#id"`, `src="url"`, and `src="url#id"`, plus `<http-request url="...">`
resource loading, uses the [CEM-ML resource lifecycle](../../docs/cem-ml-resource-lifecycle.md) as the base contract and
the [`cem-element` external resource loading contract](../../docs/cem-element-src-loading-contract.md) as the CEM Elements
binding for resource role, acquisition policy, metadata, and expected content-type context.

URI-backed canonical CEM-ML declarations and inline canonical declarations use the same
retained worker/fallback processing host. A host `loadSrcDocument` hook may keep returning
a complete string, or return `{ body, resolvedUrl, resolverIdentity }`, where `body` is an
`AsyncIterable<Uint8Array>`. The stream form preserves module-map/resolver identity and
makes the imported URL the base for relative resource controls inside that declaration.

`<http-request>` is lowered by CEM-QL to a clone-safe host-control descriptor before the
worker render plan is diffed. URL resolution, policy, response streaming, `AbortSignal`,
and stale-resource revisions remain main-thread host responsibilities. Template-visible
states use the portable lifecycle vocabulary: `scheduled`, `in-progress`, `loaded`, and
`failed` for the implemented Phase 1 transitions. JSON/XML projections are available at
`datadom.slices.<name>.data` and can be consumed by `cem:for-each`.

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
- `cem-elements:verify-cemt-pipeline-story`
- `cem-elements:verify-package`
- `cem-elements:test:unit`
- `cem-elements:test`

The gate covers file-backed legacy fixtures in `tests/parity/legacy/`, material parity fixtures in
`tests/parity/material/`, substrate CEM fixtures in `../../examples/cem-elements/`, unit coverage, Storybook browser
parity stories, and a Playwright screenshot check for the CEMT formatter/coloring/writer pipeline story.

The Phase 2 engine legs read both parity manifests directly. They extract every
inline or external declaration template, lower legacy bodies through the shared
Rust converter, and validate all 40 source sides under the package-owned
`https://cem.dev/ns/template/cem-element/1` profile. The same inputs run through
CLI roundtrip e2e, while `cem_ml:bench` applies the AC-N-1 aggregate budget to
each source side.

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

The build vendors the exact `cem_ql:build:wasm` browser module, declarations,
and WASM binary under `dist/lib/internal/runtime-support/vendor/`; published
runtime modules never import a monorepo-relative `packages/cem_ql` path. Run
`yarn nx run cem-elements:verify-package` to compare those bytes and verify the
real npm archive and clean-consumer import.

## Testing

Run `yarn nx run cem-elements:test` to execute the runtime stories through Storybook Test.

Run `yarn nx run cem-elements:verify-cemt-pipeline-story` to build Storybook and visually verify the CEMT output
pipeline story's formatted CEM tree, colored CEM tree, and writer output stages.

Run `yarn nx run cem-elements:storybook` to open the interactive Storybook runtime fixture surface.
