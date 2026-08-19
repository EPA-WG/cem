# Legacy custom-element parity inventory

Scoping inventory for the Phase 3.1 legacy parity-story work in
[`../../../docs/todo.md`](../../../docs/todo.md). It maps the old
`@epa-wg/custom-element` docs and demos to explicit `<cem-element>` Storybook coverage or to a tracked migration
decision.

Sources reviewed:

- `/home/suns/aWork/custom-element/docs/attributes.md`
- `/home/suns/aWork/custom-element/docs/rendering.md`
- `/home/suns/aWork/custom-element/README.md`
- `/home/suns/aWork/custom-element/demo/{attributes,data-slices,external-template,for-each,scoped-css}.html`
- `/home/suns/aWork/custom-element/custom-element.js`

No dedicated legacy test/spec files were present in `/home/suns/aWork/custom-element`; the demos and implementation are
the behavioral reference.

## Coverage Matrix

| Behavior | Legacy source | Status in `<cem-element>` | Storybook coverage / migration decision |
| --- | --- | --- | --- |
| Declaration registers a produced custom-element tag from `tag` | `README.md` lifecycle / tag sections | Supported | `ProducedTagValidation`, `PackageRuntimeSurface` |
| Inline declaration shape requires a direct template and rejects live declaration content | `README.md` declaration lifecycle | Supported with stricter shape | `InlineDeclarationShape`, `DeclarationLiveContentRejected`, `MissingInlineTemplateRejected` |
| `src` may load local `#id`, external documents, and `url#id` templates | `README.md` `src`; `demo/external-template.html` | Supported | `LocalSrcDeclarationLoadingParity`, `ExternalSrcDeclarationLoadingParity`, and `SrcDeclarationLoadingDiagnostics`; bare module specifiers require host `loadSrcDocument` |
| Omitted `tag` renders an inline instance | `README.md` "omitting tag" | Deferred | Not part of the produced-tag substrate MVP; record as bridge/adoption migration behavior |
| Host payload is captured into a durable data island and removed from live render output | `README.md` instance lifecycle | Supported | `DataIslandCaptureAndRender`, data-island isolation stories |
| Declared attributes expose defaults and host overrides | `docs/attributes.md`; `demo/attributes.html` | Supported | `LegacyAttributeDefaultsAndHostOverridesParity`, `DeclaredAttributeWasmRenderLoop` |
| External host attribute changes rerender produced instances | `docs/attributes.md` attribute changes | Supported | `AttributeInvalidationRerenders`, `AttributeObserverRerendersOnUndeclaredAttribute` |
| `attribute select="..."` derives an exposed attribute from slice/data state | `docs/attributes.md`; `demo/attributes.html` | Partial | Use cem-ql over `datadom.*`; full legacy XPath `select` is a migration decision |
| Legacy XPath `/datadom/attributes`, `//attributes`, `//slice` access | `docs/attributes.md`; README XPath section | Replaced | Use cem-ql record access (`datadom.attributes.*`, `datadom.slices.*`) and `??`; covered by `LegacyDatadomAccessMigrationParity` |
| Text interpolation in legacy DOM/XSLT templates | README attributes/templates | Replaced | DOM parity keeps `${$name}`; canonical CEM-ML uses `{$name}`. Covered by `LegacyAttributeDefaultsAndHostOverridesParity` and `CanonicalCemMlRenderLoop` |
| Attribute value templates | README template syntax | Supported for current substrate syntax | `FormattedDomTemplateProjection`, `CanonicalCemMlRenderLoop` |
| Default and named slots project payload nodes | README Slots section | Supported | `LegacyNamedSlotPayloadParity`, `SlotProjectionRenderLoop`, `SlotProjectionWasmRenderLoop` |
| Slice updates from DOM events rerender output | README interactivity; `demo/data-slices.html` | Supported for focused event/value forms | `LegacySliceInputEventParity`, `SliceEventInvalidationRerenders` |
| Multiple event names / multiple slice targets / checkbox and radio coercion | `demo/data-slices.html` cases B, 7-13 | Partial | Current substrate supports one event name and focused value extraction; broader legacy forms remain bridge/adoption work |
| Conditional rendering with `if` / `choose` / `when` / `otherwise` | README Pokemon example; material demos | Supported in canonical CEM-ML/cem-ql | `CemQlConditionalRenderLoop`; legacy XPath spellings migrate to `datadom.*` cem-ql |
| Loops and variables (`for-each`, `variable`, XSLT 1.0) | README loops/variables; `demo/for-each.html` | Supported for the fixture-derived subset | Legacy inline variables and `for-each` lower through the shared engine; canonical `cem:for-each` demo fixtures cover inline sequences, payload records, location query entries, and JSON/XML resource records |
| Namespaced `xhtml:*` parser workaround | README troubleshooting; material input demos | Partial | Current DOM read flattens `xhtml:*` to HTML local names; material inventory tracks this as coincidental parity |
| Scoped styles in templates | README styles section; `demo/scoped-css.html` | Supported for focused light-DOM containment | Generated `data-cem-scope` and payload-specific `data-cem-instance-scope` containment are covered by the scoped CSS demo fixture and processing-boundary tests |
| Nested produced custom elements | README embedded CE rendering | Supported | Works when nested declarations are registered, including through local/external `src`; covered by material parity stories |
| Resource slices (`module-url`, `http-request`, `local-storage`, `location-element`) | README extension primitives; demos | Supported for focused primitives | `module-url`, `http-request`, `local-storage`, and `location-element` resource slices are covered by executable demo fixtures; broad legacy XPath rewrites and progressive streaming remain deferred |
| Legacy `<template lang="custom-element-v0">` bridge | Migration window item | Supported through shared engine | `LegacyBridgeTemplateParity`; the exact annotation is the sole browser selector for the `legacy-xslt` engine path, with markup sniffing and the browser-only projection branch retired |

## Migration Decisions

- XPath is not reimplemented as a browser host engine. The legacy-XSLT bridge lowers the fixture-bounded XPath subset to
  cem-ql over flat host bindings and the structured `datadom` record.
- Browser templates enter the bridge only through exact
  `lang="custom-element-v0"`; untyped legacy markup remains DOM, while
  `custom-element-xslt` remains the native engine/CLI content-type identity.
- Legacy DOM text interpolation `${$name}` remains only for DOM-parity templates; canonical CEM-ML uses `{$name}`.
- `src`, `module-url`, and external dependency resolution are host-policy driven. `src` uses `loadSrcDocument`;
  `module-url` uses `resolveModuleUrl`; bare module specifiers require host-provided resolver hooks.
- The supported XSLT subset is pull-style and fixture-derived: `if`, `choose`, `when`, `otherwise`, `value-of`,
  inline `variable`, and `for-each` over an inline node-set variable lower to CEM-ML. Push-style XSLT and standalone
  stylesheet constructs remain Tier 3 handoff/deferred work.
- Scoped CSS uses generated light-DOM containment attributes. Template CSS is scoped by `data-cem-scope`, while projected
  payload CSS is scoped by `data-cem-instance-scope` so sibling same-tag instances do not share payload styles.

## Production Gate Status

Legacy parity is now part of the browser-substrate production gate. `yarn nx run cem-elements:verify` runs the
file-backed legacy fixture manifest through `cem-elements:verify-legacy-fixtures`, the substrate roundtrip gate,
Phase 2 CLI validation/e2e checks, the `cem_ml:bench` performance suite, unit tests, and Storybook browser parity
stories. The CLI and benchmark gates read this manifest directly, extract inline
and external template bodies, lower each legacy side through the shared Rust
converter, and validate/measure both sides of all 12 pairs. CLI validation uses
the package-owned `cem-element-template/v1` schema/content-type identity rather
than the generic CEM-ML profile.

Each of the 12 manifest pairs is also imported directly into a named `File*Parity` Storybook case. Those cases
register the declarations from the checked-in HTML, render every produced instance, compare normalized legacy and
CEM-ML light DOM one-to-one, exercise the fixture's invalidation/event action where present, and verify the
declaration-shape rejection cases. The fixture directory is an explicit `cem-elements:test` and `build-storybook`
Nx input, so an HTML fixture edit invalidates the browser evidence cache.

Each legacy and CEM-ML side passes the same Phase 3 runtime accessibility audit at initial render and again after
the fixture's mutation or event checkpoint. Across the inventory it enforces accessible names, native roles and
focusability, single-tab-stop ownership, unique IDs, resolved label/ARIA references, valid reflected ARIA values,
and image alternatives.

The current bridge proves that fixture-bounded legacy HTML+XSLT can be compiled through the shared CEM engine path to
canonical CEM-ML and rendered through `cem_ql` WASM. Passing the aggregate gate means the `<cem-element>` browser
substrate is eligible for the Phase 3.5 Edge/SSR follow-up; it does not mean the old `@epa-wg/custom-element` package
has adopted this implementation.

## Bridge / Adoption Deferrals

Keep `@epa-wg/custom-element` as a thin adapter:

- normalize untyped legacy templates to `lang="custom-element-v0"`;
- delegate parsing/conversion/rendering to the shared engine path;
- preserve copied demo/material modules as executable fixtures;
- reject or explicitly hand off Tier 3 XSLT rather than expanding the bridge by accident.

The actual `@epa-wg/custom-element` adoption remains a later Phase 3.6 handoff after the Edge/SSR runtime-support
boundary is in place.
