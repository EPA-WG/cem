# `@epa-wg/custom-element` Bridge Template Policy

Status: Accepted 2026-08-19

This records the Phase 3.6 next-major policy for legacy
`<custom-element>` templates. It follows the adapter boundary in
[`custom-element-adapter-boundary.md`](custom-element-adapter-boundary.md) and
the substrate parity inventory in
[`../packages/cem-elements/docs/legacy-parity-inventory.md`](../packages/cem-elements/docs/legacy-parity-inventory.md).

## Decision

Keep `<template lang="custom-element-v0">` for the next-major migration window,
and add a separate CEM-ML migration marker for converted legacy templates:
`<template type="cem-ml; version=0.0">`.

The bridge is intentionally fixture-bounded:

- it preserves legacy-shaped declarations, attribute defaults, host attribute
  access, simple `datadom` access, slots, focused slice/event/value wiring,
  `if`/`choose`-style conditionals, local/external `src`, and `module-url`
  resource slices where substrate fixtures already cover the behavior;
- it routes converted legacy authoring to CEM-ML/CEM-QL through
  `type="cem-ml; version=0.0"`;
- it treats broad XSLT/XPath behavior as an explicit migration option, not a hidden
  default adapter behavior.

For `<cem-element>` authoring, legacy syntax remains opt-in through
`lang="custom-element-v0"`. Converted legacy templates use
`type="cem-ml; version=0.0"` until they can move to the final canonical CEM-ML type. For the
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

The fixture boundary has since expanded through the Phase 3.6 public-adapter
matrix and executable demo/material gates. The accepted dispositions below
replace the original planning gaps without expanding the adapter into an engine.

## Accepted Gap Dispositions

| Legacy gap | Policy | Reason |
| --- | --- | --- |
| Omitted `tag` inline rendering | Keep in the package adapter | Existing consumers receive a private generated tag and inline instance; new authoring should prefer an explicit declaration and instance. The adapter delegates registration/rendering to `CemElementRuntime`. |
| XSLT `for-each` and `variable` | Convert the fixture-bounded subset | The native converter lowers accepted legacy forms to CEM-ML/CEM-QL. Unsupported Tier 3 forms produce diagnostics rather than selecting another runtime. |
| Broad XPath functions and `//path` compatibility | Convert only the locked subset | Accepted expressions lower to CEM-QL over structured inputs; unsupported expressions diagnose. No browser XPath engine is retained. |
| Multiple slice event names on one element | Keep in the substrate | `CemElementRuntime` owns the event fan-in and render invalidation; the adapter has no separate queue. |
| Multiple slice targets, `slice for=...`, checkbox/radio coercion | Keep the fixture-bounded substrate behavior | Public source/dist cases prove fan-out and control coercion through the shared event/data state machine. |
| `module-url` resource slices | Keep | The substrate already resolves inert `module-url` helpers through `resolveModuleUrl` and stores the value under `datadom.slices.*`. |
| `http-request`, `local-storage`, `location-element` resource primitives | Keep focused compatibility | Public companion shims and substrate resource controls cover the locked lifecycle/data projections. |
| Scoped CSS selector rewriting | Keep focused light-DOM containment | Declaration and per-instance payload scopes are runtime-owned and source/dist tested; arbitrary legacy rewriting outside that boundary is not promised. |

## Authoring Rules

During the migration window:

- existing `<custom-element>` inline templates with no `lang` or `type` are treated
  as legacy-v0 by the adapter;
- explicit `<template lang="custom-element-v0">` remains supported for migrated
  fixtures and for consumers that need to pin legacy behavior;
- converted legacy CEM-ML templates use `type="cem-ml; version=0.0"` during migration;
- canonical CEM-ML templates use `type="text/cem-ml"` or
  `type="application/cem-ml"`;
- new package examples and docs should prefer canonical CEM-ML/CEM-QL;
- legacy XPath examples stay within the documented converter subset or are
  rewritten to CEM-QL under `type="cem-ml; version=0.0"`;
- omitted-`tag` remains a compatibility form, while new examples use an explicit
  declaration plus produced instance;
- scoped CSS examples use the runtime's focused light-DOM declaration and
  per-instance payload containment contract.

Deprecation does not itself authorize removal. After the next-major migration
window, `custom-element-v0` and `cem-ml; version=0.0` can be removed only when all
of these exit conditions hold:

- every retained demo, material component, and downstream generator workflow has
  moved to canonical CEM-ML/CEM-QL or an explicitly documented legacy runtime;
- the twelve manifest-backed legacy/CEM-ML pairs retain their canonical side as
  executable replacement evidence;
- the public source/dist and packed-consumer gates no longer require untyped
  legacy normalization or the explicit selector;
- FF-5 reports no non-fixture runtime consumers and its registry is deliberately
  advanced through the governed removal phase.

## Decision Evidence

- The twelve manifest-backed legacy/CEM-ML pairs execute one-to-one in the
  substrate browser gate, including the exact explicit selector and negative
  routing cases.
- The shared `@epa-wg/custom-element` browser fixture proves untyped normalization
  plus explicit-selector rendering against both source and generated `dist/`.
- Both paths retain the substrate data island and pass without adapter diagnostics;
  the package verifier continues to forbid `XSLTProcessor`, `createXsltFromDom`,
  and a package-local produced-element engine.
- Canonical `text/cem-ml` and `application/cem-ml` keep precedence, and
  `custom-element-xslt` remains a native converter/CLI identity rather than a
  browser selector.

## Non-Goals

This policy does not:

- reopen the adapter-boundary decision;
- retain `XSLTProcessor`, a browser XPath engine, or package-local render loop;
- treat deprecation as approval to remove a selector with active consumers;
- alias the native `custom-element-xslt` identity into browser authoring;
- allow new package examples to choose the deprecated selector over canonical
  CEM-ML/CEM-QL.
