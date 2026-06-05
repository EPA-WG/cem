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
| Loops and variables (`for-each`, `variable`, XSLT 1.0) | README loops/variables; `demo/for-each.html` | Deferred | Not in the C2 substrate; bridge support may preserve legacy XSLT during migration |
| Namespaced `xhtml:*` parser workaround | README troubleshooting; material input demos | Partial | Current DOM read flattens `xhtml:*` to HTML local names; material inventory tracks this as coincidental parity |
| Scoped styles in templates | README styles section; `demo/scoped-css.html` | Partial | Styles render into light DOM but are not scoped; material inventory tracks containment as open |
| Nested produced custom elements | README embedded CE rendering | Supported | Works when nested declarations are registered, including through local/external `src`; covered by material parity stories |
| Resource slices (`module-url`, `http-request`, `local-storage`, `location-element`) | README extension primitives; demos | Partial | Focused `module-url` URL resolution is supported through `resolveModuleUrl` and material parity coverage; `http-request`, `local-storage`, and `location-element` remain later primitive/resource slices |
| Legacy `<template lang="custom-element-v0">` bridge | Migration window item | Blocked by design | `LegacyBridgeTemplateBlockedParity`; bridge support stays a separate todo item |

## Migration Decisions

- XPath is not reimplemented in the browser host. Functional parity uses cem-ql over the structured `datadom` record.
- Legacy DOM text interpolation `${$name}` remains only for DOM-parity templates; canonical CEM-ML uses `{$name}`.
- `src`, `module-url`, and external dependency resolution are host-policy driven. `src` uses `loadSrcDocument`;
  `module-url` uses `resolveModuleUrl`; bare module specifiers require host-provided resolver hooks.
- XSLT-only constructs (`for-each`, `variable`, full XPath functions) are bridge/adoption concerns unless promoted
  into cem-ql/cem-ml explicitly.
- Scoped CSS currently renders as light-DOM CSS. True scoping/containment is a material parity gap, not a hidden
  substrate guarantee.
