# `@epa-wg/custom-element` Bridge Template Policy

This records the Phase 3.6 next-major policy for legacy
`<custom-element>` templates. It follows the adapter boundary in
[`custom-element-adapter-boundary.md`](custom-element-adapter-boundary.md) and
the substrate parity inventory in
[`../packages/cem-elements/docs/legacy-parity-inventory.md`](../packages/cem-elements/docs/legacy-parity-inventory.md).

## Decision

Keep `<template lang="custom-element-v0">` for the next-major migration window,
but keep it as a bridge into `CemElementRuntime`, not as permission to retain the
legacy XSLT/XPath implementation.

The bridge is intentionally fixture-bounded:

- it preserves legacy-shaped declarations, attribute defaults, host attribute
  access, simple `datadom` access, slots, focused slice/event/value wiring,
  `if`/`choose`-style conditionals, local/external `src`, and `module-url`
  resource slices where substrate fixtures already cover the behavior;
- it routes new authoring to canonical CEM-ML/CEM-QL;
- it turns broad XSLT/XPath behavior into explicit migration work instead of a
  hidden browser-only renderer inside `@epa-wg/custom-element`.

For `<cem-element>` authoring, legacy syntax remains opt-in through
`lang="custom-element-v0"`. For the `<custom-element>` adapter, untyped inline
templates may be normalized to `lang="custom-element-v0"` during the migration
window so existing package consumers can load while fixture gaps are made
visible.

## Fixture Evidence

Existing substrate coverage supports the bridge scope:

| Behavior | Evidence |
| --- | --- |
| Declaration shape, produced tag registration, payload isolation | `InlineDeclarationShape`, `DataIslandCaptureAndRender`, `PackageRuntimeSurface` |
| Attribute defaults and host overrides | `LegacyAttributeDefaultsAndHostOverridesParity`, `DeclaredAttributeWasmRenderLoop` |
| Structured `datadom` access instead of XPath | `LegacyDatadomAccessMigrationParity`, `CemQlDataDocumentRenderLoop` |
| Slots and payload projection | `LegacyNamedSlotPayloadParity`, `SlotProjectionRenderLoop`, `SlotProjectionWasmRenderLoop` |
| Focused slice events | `LegacySliceInputEventParity`, `SliceEventInvalidationRerenders` |
| Local and external `src` | `LocalSrcDeclarationLoadingParity`, `ExternalSrcDeclarationLoadingParity`, `SrcDeclarationLoadingDiagnostics` |
| Legacy bridge template surface | `LegacyBridgeTemplateParity` |
| `module-url` resources | `MaterialIconLinkParity` |

The legacy docs and demos also show behaviors that remain outside the fixture
boundary: omitted `tag` inline rendering, XSLT `for-each`/`variable`, broad XPath
functions, multi-event/multi-target slice wiring, companion resource primitives,
and true scoped CSS rewriting.

## Gap Policy

| Legacy gap | Policy | Reason |
| --- | --- | --- |
| Omitted `tag` inline rendering | Migrate | The substrate requires a produced tag. Auto-generated tags are hard to make stable for SSR, edge snapshots, diagnostics, and custom-element registry collisions. Migrate authors to an explicit `tag` plus explicit produced instance. |
| XSLT-only `for-each` and `variable` | Migrate | Do not keep `XSLTProcessor`. Move loops and local bindings to canonical CEM-ML/CEM-QL constructs as they land in the render boundary. Until then, affected demos are documented migration gaps. |
| Broad XPath functions and `//path` compatibility | Migrate | The runtime data model is a structured `DataIslandSnapshot`. Use cem-ql over `datadom.attributes`, `datadom.dataset`, `datadom.payload`, and `datadom.slices`; do not ship an XPath evaluator. |
| Multiple slice event names on one element | Keep only if promoted into substrate | This can be a small extension to substrate event binding, but it must live in `CemElementRuntime`. The adapter must not keep a separate event queue. |
| Multiple slice targets, `slice for=...`, checkbox/radio coercion | Migrate | These behaviors combine target lookup, coercion, and validation side effects from the old render loop. Prefer explicit rendered controls and cem-ql expressions until a substrate primitive is designed. |
| `module-url` resource slices | Keep | The substrate already resolves inert `module-url` helpers through `resolveModuleUrl` and stores the value under `datadom.slices.*`. |
| `http-request`, `local-storage`, `location-element` resource primitives | Defer to companion-module task | They remain published companion modules for compatibility, but the bridge-template policy does not make them substrate render primitives. |
| True scoped CSS selector rewriting | Drop as a guarantee | Current substrate renders light-DOM styles. Do not promise legacy selector rewriting unless a dedicated style-containment primitive is designed and tested. |

## Authoring Rules

During the migration window:

- existing `<custom-element>` inline templates with no `lang` or `type` are treated
  as legacy-v0 by the adapter;
- explicit `<template lang="custom-element-v0">` remains supported for migrated
  fixtures and for consumers that need to pin legacy behavior;
- explicit CEM-ML templates use `type="text/cem-ml"` or
  `type="application/cem-ml"`;
- new package examples and docs should prefer canonical CEM-ML/CEM-QL;
- legacy XPath examples should be rewritten to structured `datadom.*` access;
- omitted-`tag` examples should be rewritten to a declaration plus produced
  instance;
- scoped CSS examples should state that styles are light-DOM/global unless a
  later containment primitive is added.

After the migration window, `custom-element-v0` can be removed only when package
fixtures prove that all retained demos and downstream generator workflows have
moved to canonical CEM-ML/CEM-QL.

## Implementation TODO

- Add adapter fixtures proving untyped `<custom-element>` templates are routed to
  legacy-v0 without mutating explicitly typed CEM-ML templates.
- Add a diagnostic fixture for omitted `tag` that points to the explicit
  declaration plus instance migration.
- Add migration examples for `for-each` and `variable` once the canonical
  CEM-ML/CEM-QL loop/binding constructs are available in the browser runtime.
- Decide whether whitespace-separated `slice-event` values are promoted into
  `CemElementRuntime`; if promoted, cover them in substrate stories before the
  adapter accepts them.
- Keep `module-url` bridge coverage tied to `resolveModuleUrl`.
- Move `http-request`, `local-storage`, and `location-element` decisions to the
  companion-module/resource-primitive task.
- Add a scoped-CSS migration note that recommends explicit classes or future
  containment primitives rather than legacy selector rewriting.

## Non-Goals

This policy does not:

- reopen the adapter-boundary decision;
- retain `XSLTProcessor`, XPath, or package-local DOM merge logic;
- implement CEM-ML loop or variable support;
- decide companion-module rewrites;
- rewire downstream `cem-theme` consumers.
