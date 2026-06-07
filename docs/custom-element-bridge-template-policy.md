# `@epa-wg/custom-element` Bridge Template Policy

This records the Phase 3.6 next-major policy for legacy
`<custom-element>` templates. It follows the adapter boundary in
[`custom-element-adapter-boundary.md`](custom-element-adapter-boundary.md) and
the substrate parity inventory in
[`../packages/cem-elements/docs/legacy-parity-inventory.md`](../packages/cem-elements/docs/legacy-parity-inventory.md).

## Decision

Keep `<template lang="custom-element-v0">` for the next-major migration window,
and add a separate CEM-ML migration marker for converted legacy templates:
`<template type="cem-ml-v0">`.

The bridge is intentionally fixture-bounded:

- it preserves legacy-shaped declarations, attribute defaults, host attribute
  access, simple `datadom` access, slots, focused slice/event/value wiring,
  `if`/`choose`-style conditionals, local/external `src`, and `module-url`
  resource slices where substrate fixtures already cover the behavior;
- it routes converted legacy authoring to CEM-ML/CEM-QL through
  `type="cem-ml-v0"`;
- it treats broad XSLT/XPath behavior as an explicit migration option, not a hidden
  default adapter behavior.

For `<cem-element>` authoring, legacy syntax remains opt-in through
`lang="custom-element-v0"`. Converted legacy templates use
`type="cem-ml-v0"` until they can move to the final canonical CEM-ML type. For the
`<custom-element>` adapter, untyped inline templates may be normalized to
`lang="custom-element-v0"` during the migration window so existing package
consumers can load while fixture gaps are made visible.

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
| XSLT-only `for-each` and `variable` | Option A keep explicitly, Option B convert | Option A keeps the legacy XSLT+XPath model with HTML and XSLT default-namespace behavior. Option B converts the logic to CEM-ML+CEM-QL under `type="cem-ml-v0"`. The recommended CSS-generator path is Option B. |
| Broad XPath functions and `//path` compatibility | Option A keep explicitly, Option B convert | Option A preserves XPath. Option B rewrites selection to CEM-QL over structured inputs. The default substrate adapter must not accidentally hide an XPath engine; if Option A is selected, name and fixture it as a legacy runtime. |
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
- converted legacy CEM-ML templates use `type="cem-ml-v0"` during migration;
- canonical CEM-ML templates use `type="text/cem-ml"` or
  `type="application/cem-ml"`;
- new package examples and docs should prefer canonical CEM-ML/CEM-QL;
- legacy XPath examples should either stay in an explicit Option A legacy runtime
  path or be rewritten to CEM-QL under `type="cem-ml-v0"`;
- omitted-`tag` examples should be rewritten to a declaration plus produced
  instance;
- scoped CSS examples should state that styles are light-DOM/global unless a
  later containment primitive is added.

After the migration window, `custom-element-v0` and `cem-ml-v0` can be removed only
when package fixtures prove that all retained demos and downstream generator
workflows have moved to canonical CEM-ML/CEM-QL or to an explicitly documented
legacy runtime.

## Implementation TODO

- Add adapter fixtures proving untyped `<custom-element>` templates are routed to
  legacy-v0 without mutating explicitly typed CEM-ML templates.
- Add a diagnostic fixture for omitted `tag` that points to the explicit
  declaration plus instance migration.
- Add CSS-generator migration fixtures using `<template type="cem-ml-v0">` and
  convert legacy `<variable>`/`<for-each>`/XPath logic to CEM-ML+CEM-QL.
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
- silently retain `XSLTProcessor`, XPath, or package-local DOM merge logic without
  choosing Option A and documenting it as a legacy runtime;
- implement CEM-ML loop or variable support;
- decide companion-module rewrites;
- rewire downstream `cem-theme` consumers.
