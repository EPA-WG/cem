# @epa-wg/cem-elements

Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>

`@epa-wg/cem-elements` provides the `<cem-element>` browser substrate for declarative light-DOM custom elements. It is
the Phase 3.1 runtime gate before the project moves into Edge/SSR support and later `@epa-wg/custom-element` adoption.

Declarations may publish an exact Semantic Version for SSR adoption:

```html
<cem-element tag="cem-card" version="1.2.3">
    <template type="text/cem-ml">...</template>
</cem-element>
```

The declaration owns the version. Each SSR data island records it as
`declarationVersion`; produced instances do not receive a public `version`
attribute or property. `runtime.declarationVersionFor(instance)` provides
collision-safe introspection. Hydration adopts caret-compatible versions. A
missing, malformed, or incompatible declaration version rerenders from an
understood island, while an unsupported island schema still fails closed.

An identical declaration can remount after its previous same-scope owners have
disconnected. It reuses the browser constructor, processing scope and managed
stylesheet nodes. Styles move to another live compatible declaration when their
owner disconnects, and detach when the last owner leaves. Concurrent new
same-scope duplicates and incompatible replacements still fail; remounts cannot
revive disposed processing scopes. See the
[registration lifecycle](../../docs/cem-element-design.md).

External declaration loading through `src="#id"`, `src="url"`, and `src="url#id"`, plus `<http-request url="...">`
resource loading, uses the [CEM-ML resource lifecycle](../../docs/cem-ml-resource-lifecycle.md) as the base contract and
the [`cem-element` external resource loading contract](../../docs/cem-element-src-loading-contract.md) as the CEM Elements
binding for resource role, acquisition policy, metadata, and expected content-type context.

The [demo teaching-point audit](docs/demo-teaching-points.md) records the lessons
preserved from the local prototype and the standalone/source-loaded checks.

For event-driven browser navigation, `location-element` accepts an optional
`trigger` token. An empty token does not write; a new nonempty token is consumed
before navigation, once per writer position in an instance. Rerendering a live
reader or editing a pending target with the same token does not reapply that
command. Slice event payloads expose a per-slice `revision` that advances even
for repeated equal-value events; use it as the token. Keep writer positions
stable, or isolate independent conditional writers in separate instances.
Omitting `trigger` preserves continuous/authoritative writer behavior. See the
[URL writer demo](demo/set-url.html) and [two-way version picker](demo/npm-versions-demo.html).

URI-backed canonical CEM-ML declarations and inline canonical declarations use the same
retained worker/fallback processing host. A host `loadSrcDocument` hook may keep returning
a complete string, or return `{ body, resolvedUrl, resolverIdentity }`, where `body` is an
`AsyncIterable<Uint8Array>`. The stream form preserves module-map/resolver identity and
makes the imported URL the base for relative resource controls inside that declaration.

A CEM-ML template can explicitly reference a separate XPath function library:

```html
<template type="text/cem-ml" xpath-functions="./fruit-functions.cemt">
{attribute @name=text}
{output | {$native:call("demo.label", text)}}
</template>
```

The library is a CEMT module containing public named functions with typed
parameters and XPath bodies; see the [interactive examples](demo/xpath-functions.html)
and [library source](demo/xpath-functions.cemt). Its reference uses the declaration's
scope-aware module resolver, including module maps and host resolution hooks. Relative
URLs resolve from the declaration document. A host redirect's final URL is preserved
as the library source URI. The library is a separate dependency, never a render-data
field, and inline function declarations do not implicitly enable it.

Each logical scope loads an immutable source snapshot once per resolved URL/policy.
The processing host shares retained companions by URL, source hash, compiler versions
and resolver policy. Worker transport carries bounded source and identity metadata;
the worker or fallback validates and compiles once, then selects its retained companion
for subsequent renders. Binary template cache hits still require that separate library.
The last evicted template consumer releases its companion; failed compilation and scope
disposal also release handles. Failed source loads may retry on the next render. Use a
new versioned library URL or a fresh scope to load changed source bytes.

This browser route accepts explicitly declared scalars and imported CEM nodes.
For example, the compatibility XML reader can be authored as:

```cem-ml
{cem-data @name=document @select=datadom.slices.source @type=xml @projection=xpath}
{cem:for-each @as=item @select='native:call("demo.items", document.root)' |
    {p | {$item}}}
```

The reader's `error` field reports invalid XML; check it before calling a function
with `document.root`. XPath selects nodes from the CEM tree, preserving source
ownership, identity and source maps. Worker and fallback hosts retain up to 16
successful imports per compiled template, keyed by source, format and projection,
and release them when the template is disposed. Each input is limited to 32 KiB,
64 levels and 4096 source events or values. Changed input gets a distinct owner;
unchanged cached input is not reparsed. Ordinary CEM reader roots also supply
native nodes. Records and source strings are not inferred as nodes.

The [XPath XML table/tree demos](./demo/xpath-nodes.html) use the separate
`xpath-nodes.cemt` function library and explicit XML XPath reader view.
Names retain their namespace URIs, adjacent text/CDATA shares a text node, and
CEM-QL sorting preserves each native node for XPath parent/sibling navigation.
The table selects rows by unique authored `@id`. The sorting gallery below
uses standard XPath `fn:sort`; HTTP ownership uses retained CEM documents.

The [XPath sequence demos](./demo/xpath-sequences.html) use
`xpath-sequences.cemt` for rounded word windows, reversal, head/tail selection
and distinct word counts. A second example discovers XML columns and matches
native cells by kind, local name and namespace URI. Its XPath expression
explicitly preserves first-seen column order; it does not depend on
`distinct-values` result order. Live edits add new columns without new field
expressions. Missing cells display ∅, empty cells display `""`, and repeated
cells join with ` / `.

The [XPath sorting demos](./demo/xpath-sort.html) use `xpath-sort.cemt` for
stable text/numeric ordering and multiple row keys. Inline functions capture
scalar direction settings inside XPath and return retained source nodes;
selection and source-sibling navigation survive reordering. Missing/invalid-last
keys are explicitly authored. Only codepoint collation is supported, and compiled
XPath program format v3 requires recompilation of older v1/v2 programs.

The [XPath validation demos](./demo/xpath-validation.html) use
`xpath-validation.cemt` for form fields and a local IPv4 prefix-length rule.
Regex lexical checks combine with integer ranges and `some`/`every`;
regex tokenize/replace format accepted tags. The demos explain the supported
regex subset and explicit exclusions. They do not implement subnet matching
or network enforcement.

The [XPath aggregate demos](./demo/xpath-aggregates.html) use
`xpath-aggregates.cemt` for decimal-list statistics and a live XML basket.
Each newly added fruit element contributes to the sum, extrema and average.
Authored XPath validates amounts before explicitly casting to decimal;
invalid/missing amounts are errors, empty input sums to zero, and absent
extrema/averages display ∅. Terminating decimal averages are exact; repeating
averages follow the existing 18-significant-digit half-even division policy.
Nonnumeric extrema and duration aggregates remain outside this native slice.

The [XPath maps/arrays demos](./demo/xpath-maps-arrays.html) use
`xpath-maps-arrays.cemt` for an optional-entry IP-filter preview and an XML basket
selected by one-based array position. Native maps/arrays stay opaque between
named calls; only explicit XPath lookup selects their contents. Empty-valued
entries differ from absent keys, and square arrays keep empty member slots.
These demos use declared scalars and retained CEM trees imported from XML and
JSON. The third maps/arrays case imports the standard JSON-to-XML tree, retains
selected nodes in an XPath array, and distinguishes null, empty and absent
members. XML, JSON, YAML and CSV readers share one native XPath node view;
format-specific code belongs to CEM AST import. The accepted
[loader contract](../../docs/cem-data-loader-plan.md) connects HTTP XML/JSON
to that same tree through the CEM-ML library. See the
[import principle](../../docs/cem-data-import-principle.md).

Library source and URI are each limited to 32 KiB, libraries to 64 XPath functions,
and each scope to 64 loaded
libraries; the WASM companion registry also enforces its count/byte limits. Calls share a 16,777,216-unit XPath work counter across nested expressions and
limit intermediate/final text to 1 MiB of UTF-8 atomic lexical bytes per value
or sequence. Atomizing native XML also obeys these limits; retained document
owners are not serialized or counted as output text. Limit failures cannot be
caught as CEM-QL data errors. The [live counts](demo/dom-merge.html) and
[string comparisons](demo/functions/str.html) demonstrate XML whitespace,
Unicode codepoints, tokenization and joining through [a text library](demo/xpath-text.cemt).
Library imports and templates using the older non-retained resource-render path are not
supported. Generic template binaries and
ordinary JSON data cannot install functions. Native-owner bindings remain available
at the Rust API described in the [CEM-QL contract](../cem_ql/README.md#xpath-function-companions).

Strict XSLT 3.0 declarations use `src="./view.xslt"` and
`xslt-template="entrypoint"`, with direct `<xslt-param name="..." select="...">`
children containing independent CEM-QL scalar selectors. Unmapped parameters
keep their XSLT defaults. Named entrypoints may run without initial focus;
source parsing stays in CEM-ML import. Imported stylesheets and native bundles
share the existing resolver, worker/fallback and disposal lifecycle.
The environment sets `CemElementRuntime({ controlInputBytes })` (default 8 MiB).
An explicit declaration scope may lower this through
`createCemDeclarationScope({ document, parent, controlInputBytes })`; omitted
values inherit and increases above a parent or environment ceiling fail.
Anonymous tags distinguish declaration runtimes and policies; a fixed public
tag registered under another policy is an incompatible declaration.
The effective limit is retained with the native component and checked in UTF-8
bytes before control decoding, independently of document import limits.
See the [control-input policy](../../docs/xslt-runtime-lowering.md#browser-control-envelope-decision)
and [parameter contract](../../docs/xslt-runtime-lowering.md#browser-state-binding-decision).

Phase 3B shares lazily allocated worker slots across compatible logical roots without
changing their scheduling semantics. The current `cem-processing-host-v4` contract
carries explicit native XSLT compilation options and retained host/scope limits;
its `document` retention/release remains alongside `compile`, `renderDiff`, `cancel` and `dispose`. The default
pool uses browser hardware concurrency capped at eight workers, a 64-operation queue per
slot, FIFO ordering per root, and round-robin cross-root dispatch. Compiled artifacts and
render plans use bounded content-addressed LRU retention; an evicted previous plan safely
falls back to a full scope replacement. `processingPoolPolicy` can lower the worker and
queue limits, while `onProcessingTrace` observes sequence-only scheduling decisions.

`<http-request>` is lowered by CEM-QL to a clone-safe host-control descriptor before the
worker render plan is diffed. URL resolution, policy, response streaming, `AbortSignal`,
and stale-resource revisions remain main-thread host responsibilities. Template-visible
states use the portable lifecycle vocabulary: `scheduled`, `in-progress`, `loaded`, and
`failed` for the implemented transitions. CEM-ML imports bounded response bytes
into a retained CEM tree. During native rendering, `datadom.slices.<name>.data`
is a CEM document node accepted by CEM-QL and XPath functions; JavaScript
snapshots carry lifecycle metadata with `data: null`. Worker messages carry
bytes and explicit execution-local bindings, and fallback re-imports those bytes
through the same library. Owners are released on replacement, disconnect and
scope disposal. See the [loader contract](../../docs/cem-data-loader-plan.md)
and [HTTP examples](demo/http-request.html). Progressive CEM-ML AST streaming
remains a later phase.

`<repository-query>` and `<storage-status>` use the same transient-control boundary for
logical repository reads. Applications register host-owned ports with
`CemRepositoryRegistry` and inject `registry.readOnly()` through the runtime's
`repositoryRegistry` option. The facade exposes only `query`, `subscribe`, and `status`;
it deliberately has no `execute` capability.

```cem
{repository-query
    @slice=projects
    @repository=studio-projects
    @operation=list-projects
    @parameters="{$datadom.attributes.query-parameters}"
    @live=true
    @cursor=0}
{storage-status @slice=storage @repository=studio-projects @live=true @cursor=0}
```

`parameters` is optional JSON; the example reads it from an instance attribute such as
`query-parameters='{"includeTrash":false}'` so the JSON reaches the declaration as one
interpolated value. `live` subscribes from the optional non-negative durable change cursor,
and every query revision is runtime-owned. Both resources project
`scheduled`, `loaded`, or `failed` envelopes under `datadom.slices.<name>`, reject stale
completions, and release subscriptions and abortable queries when superseded, removed,
or disconnected. `storage-status` reports the port's existing quota/persistence state;
rendering cannot invoke a repository mutation or call `navigator.storage.persist()`.

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

### Native JSON storage

`{local-storage @key=preferences @slice=preferences @type=json @live=true}`
imports stored bytes through CEM-ML. `datadom.slices.preferences` is a native
CEM document; select its children and use `dom:text` for text. The
[storage demo](demo/local-storage.html) shows generic-data object properties,
arrays, scalars, and a basket edited through native `slice-value` attributes.
CEMT constructs replacement nodes without modifying the imported tree.

Native slice event values travel as portable CEM artifacts across workers,
fallback and saved hydration. Explicit writes export compact JSON through
CEM-ML, preserving property order, duplicate keys and exact number text.
Unchanged reads preserve original source bytes. Invalid reads keep raw storage
and publish a diagnostic with no native tree; invalid writes leave storage
unchanged. Empty/null slice writes remove the key, while a native JSON-null
node writes the literal `null`. Scalar storage types retain their established
browser-input coercion. Native event values target slices; native component
attributes use CEMT attribute construction.
