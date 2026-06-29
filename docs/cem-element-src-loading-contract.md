# cem-element External Resource Loading Contract

Status: Draft · Type: Runtime Contract

This document defines how CEM Elements bind URL-bearing declaration attributes to the shared
[CEM-ML resource lifecycle](./cem-ml-resource-lifecycle.md), declaration artifacts, and AST streams. It applies to:

- `<cem-element src="...">` declaration-template resources;
- `<http-request url="...">` response AST stream resources in CEM-ML templates;
- the compatibility `<custom-element>` adapter, because that adapter registers declarations and resource helpers through
  the `CemElementRuntime` substrate.

The contract covers these authoring forms:

```html
<cem-element tag="x-card" src="#local-template"></cem-element>
<cem-element tag="x-card" src="./cards.html"></cem-element>
<cem-element tag="x-card" src="./cards.html#card-template"></cem-element>
<cem-element tag="x-card" src="@scope/cards/card.html#card-template"></cem-element>

<http-request slice="page" url="./data/page.json" method="GET"></http-request>
<http-request slice="page" url="@scope/data/page.json" method="GET"></http-request>
```

Bare specifiers such as `@scope/cards/card.html` and `@scope/data/page.json` are valid only when the active host or
module-map resolver provides an authorized mapping.

## Scope

An external resource attribute is not an arbitrary DOM include or direct browser object handoff. Loading a resource MUST
create a CEM-owned resource state, artifact boundary, or AST event stream before the template observes it. Template-visible
state MUST use the portable lifecycle defined by the CEM-ML stack.

There are two resource kinds in this contract:

- **Template resources** — `<cem-element src="...">` and compatibility `<custom-element src="...">`; loading MUST
  declare a `template-src` resource role, acquire the selected source, and pass the expected template content-type context
  to the CEM-ML engine.
- **Response resources** — `<http-request url="...">`; loading MUST produce one resource slot containing request
  metadata, optional response metadata, lifecycle state, diagnostics, and the expected response content-type context for
  the CEM-ML engine. Any queryable projection exposed to CEM-QL is produced by the CEM-ML AST stream contract, not by a
  host JavaScript object, JSON serialization step, or fully materialized payload handoff.

The supported reference forms are:

- `src="#id"` — resolve `id` in the declaring document as a template resource.
- `src="url"` — acquire the selected resource document as a template-source resource.
- `src="url#id"` — acquire `url`, then use the referenced subtree as the template-source resource.
- `url="url"` on `<http-request>` — schedule and fetch a response resource under the active resource policy, exposing
  lifecycle state immediately and passing response content to CEM-ML when available.

For a `src` fragment target:

- If the target is a `<template>`, its template content is used.
- If the target contains a direct child `<template>`, that child template is used.
- Otherwise the target element subtree itself is wrapped as the declaration template.

For `src="url"` without a fragment:

- If the fetched document body has content, the body child nodes are used.
- Otherwise the document element is used.
- If the only meaningful node is a `<template>`, that template is used directly.

For `<http-request url="...">`, fragments are not normal network HTTP selectors. Fragment-only or `url#fragment`
resource forms are valid only when the active host policy provides a local resource provider or fragment-aware loader.
Otherwise the URL, including fragment, is resolved as request metadata but the HTTP transport receives the fragment-free
network URL according to host policy.

## URL Resolution And Module Maps

Relative URL-like specifiers MUST resolve against the declaring document's `baseURI`, including any active `<base>`.
Absolute URLs use their own origin and scheme.

Bare specifiers, such as `@scope/package/card.html` or `@scope/data/page.json`, MUST NOT be guessed by the default
runtime. They require a host-provided resolver/loader. For template resources that hook is `loadSrcDocument`; for
`http-request` response resources the hooks are `resolveResourceUrl` and `loadHttpResource`. Those hooks own
import-map/module-map lookup, package policy, and resource authorization.

Resolution and scheduling order is:

1. Parse the authored attribute into a resource specifier and resource-kind-specific fragment/request metadata.
2. Create or update a `declared` CEM resource state with the authored specifier and current context identity.
3. Resolve the resource specifier through the host module-map resolver when it is a bare specifier.
4. Resolve URL-like specifiers against the declaring document base URL.
5. Move the resource state to `scheduled`, `waiting`, or `failed` under host resource policy.
6. For template resources, load when scheduled, select the full document or fragment target, and pass the selected source
   to CEM-ML with `template-src` role and expected content-type context.
7. For `http-request` resources, expose the scheduled or waiting resource slice, then open the authorized request when
   scheduled, and pass the response body stream plus request/response metadata to CEM-ML with `http-request` role and
   expected content-type context.

Template-visible resource state MUST follow the portable lifecycle names and transition rules in
[CEM-ML Resource And AST Stream Lifecycle](./cem-ml-resource-lifecycle.md). Hosts MAY expose additional internal states,
but they MUST map to the CEM-ML lifecycle before templates observe them.

## CEM-ML Resource Lifecycle Binding

This contract binds the shared CEM-ML lifecycle to two resource kinds:

- `template-src` for `<cem-element src>` and compatibility `<custom-element src>` declaration-template resources;
- `http-request` for `<http-request url>` response resources.

For `template-src`, the lifecycle state tracks declaration discovery, URL/module-map resolution, load scheduling, source
streaming, declaration-template artifact production, terminal success, and terminal failure.

For `http-request`, the lifecycle state tracks declaration discovery, request scheduling, transport progress, response
content streaming, terminal success, and terminal failure. The resource slot under `datadom.slices.<slice>` is the
template-visible lifecycle envelope for that response resource.

Hydration, de-hydration, stream sequence identity, revision rules, renderability from lifecycle state, content-type
negotiation, parser/plugin dispatch, AST stream creation, stream-derived projections, and parser/render-boundary
diagnostics are owned by the CEM-ML lifecycle contract. This loading contract adds only the `src` and `http-request`
binding details, acquisition policy, expected content-type context, cache identity, host security policy, transport
metadata, and resource-specific host diagnostics.

The cache identity for a resource revision MUST include at least:

- declaring document base URI;
- original resource specifier;
- resolved URL or module-map result;
- module-map/resolver identity when a host resolver is used;
- resource kind (`template-src`, `http-request`, or future resource kind);
- resource-policy stamp when security policy can change the bytes returned;
- request method, headers that participate in cache identity, credentials policy, and expected content type for
  `http-request` resources.

The current browser runtime caches external source documents per runtime instance by declaring document `baseURI` and
specifier path. Hosts that add module-map or security-policy resolution SHOULD include those identities in their
`loadSrcDocument` behavior or use separate runtime instances when policy differs.

## Resource Role And Expected Content-Type Context

`cem-element` and `http-request` classify resources before handing them to CEM-ML:

- `<cem-element src>` and compatibility `<custom-element src>` classify their selected source as `template-src`.
- `<http-request url>` classifies its response as `http-request`.

The binding MUST pass this context to the CEM-ML engine:

- resource kind and role;
- active context identity;
- expected content-type set for the resource role;
- selected source or response stream;
- provided content-type metadata known to the host, such as HTTP `Content-Type`, module-map metadata, target markers, or
  file extension hints;
- source identity and source-range capability;
- host security policy identity;
- cache/resource-policy stamp;
- request and response metadata for `http-request`.

For `template-src`, the expected content-type context is the active template-source accepted set supplied to the CEM-ML
engine and narrowed or extended by registered CEM-ML context policy.

For `http-request`, the expected content-type context is the active response/data accepted set supplied to the CEM-ML
engine. The optional `content-type` attribute on `<http-request>` is an expectation or fallback passed to CEM-ML, not an
unsafe override of a conflicting response `Content-Type`.

CEM-ML owns the accepted content-type registry, parser/plugin negotiation, context-tree propagation, mismatch handling,
AST stream creation, stream-derived projections, and parser/render-boundary diagnostics. This binding MUST NOT infer
support from file extension or DOM shape when the CEM-ML engine has not accepted that type.

## Host Binding Processing Shape

The required `template-src` binding shape is:

1. Create or update a `declared` resource state from the authored `src` specifier and current context.
2. Resolve, authorize, and schedule the resource under URL, module-map, and host security policy.
3. Acquire bytes/text or a host-owned selected DOM/document source when the scheduler opens the resource.
4. Select the same-document target, external document, external template, or external subtree under the `src` rules.
5. Pass the selected source, `template-src` role, expected content-type context, source identity, and host policy identity
   to the CEM-ML engine.
6. Render produced element instances only from CEM-ML lifecycle state, declaration artifacts, AST streams, and instance
   data islands.

The required `http-request` binding shape is:

1. Interpolate the authored `url` expression under the active resource scope enough to capture the resource specifier.
2. Create or update a resource slot envelope under `datadom.slices.<slice>` with `declared` lifecycle state, request
   metadata, source identity, and host diagnostics.
3. Resolve the resource specifier under URL, module-map, and host resource policy.
4. Authorize method, headers, credentials, cache policy, redirects, response size, and timeout under host policy.
5. Move the resource slot to `scheduled`, `waiting`, or `failed` according to the shared CEM-ML lifecycle.
6. Open the request through the host loader and move the resource slot through `in-progress` lifecycle state.
7. Pass the response body stream, response metadata, `http-request` role, expected content-type context, source identity,
   and host policy identity to the CEM-ML engine.
8. Rerender templates only from CEM-ML lifecycle or content revisions.

The binding MUST NOT pass raw `Response`, `Headers`, `Document`, DOM node, JavaScript object, host object graph, or raw
string values into templates. Raw host objects remain outside the CEM-ML boundary.

## Security Context

External resource loading is a trust-boundary crossing. The host binding MUST treat external content as untrusted input
until CEM-ML accepts it through the shared resource lifecycle, content-type, parser, and artifact rules.

Security requirements:

- Fetching MUST obey browser origin, CORS, CSP, and credential rules, or stricter host resource policy.
- A host loader MUST define whether credentials are included. The default policy SHOULD be same-origin credentials only
  and no cross-origin credentials unless explicitly configured.
- Bare module-map specifiers MUST resolve only through an authorized host resolver.
- Redirects, opaque responses, and MIME mismatches SHOULD be rejected unless the host policy explicitly accepts them.
- `http-request` mutating methods, non-simple headers, credentials, cache mode, redirect mode, and maximum response size
  MUST be governed by resource policy.
- Script elements in loaded HTML MUST NOT execute merely because the document was loaded as a template source.
- Script-like response bodies loaded through `http-request` MUST be passed to CEM-ML only as response content under the
  active resource policy; they MUST NOT execute as page code.
- Inline event handler attributes and script-bearing URLs MUST be rejected or sanitized according to the active
  declaration security policy before render.
- Styles are template output, not loader authority. Scoped CSS processing may rewrite or diagnose styles, but loading a
  source file MUST NOT grant additional CSS privilege beyond the produced render.
- Diagnostics MUST be attached to the declaration or instance that requested the load.

The compatibility `<custom-element>` adapter follows this contract for both its `src` declarations and the package's
`<http-request>` helper. The adapter may preserve author fallback/payload children while a `src` declaration loads, but
those children are not treated as declaration template source when `src` is present.

## Host Binding Diagnostics

The host binding SHOULD report these categories with stable diagnostic codes:

- `cem-element.src_load_failed` — the document could not be loaded.
- `cem-element.src_target_missing` — `url#id` did not resolve to a selectable target.
- `cem-element.src_local_target_missing` — `#id` did not resolve in the declaring document.
- `cem-element.src_module_resolution_failed` — bare/module-map specifier could not be resolved.
- `cem-element.src_security_rejected` — resource policy rejected the load or selected subtree.
- `cem.resource.http.unresolved_url` — `http-request url` could not be resolved under the active resolver policy.
- `cem.resource.http.security_rejected` — request or response was rejected by resource policy.
- `cem.resource.http.load_failed` — request transport failed or returned a disallowed response.

Content-type mismatch, unsupported parser, AST parse, artifact compile, projection, render-boundary, hydration, and
de-hydration diagnostics are CEM-ML engine diagnostics. The host binding attaches them to the resource state and owning
declaration or instance, but it does not define their semantics.

Existing runtimes may emit the currently implemented subset while this contract is in draft.

## Examples

Same-document template:

```html
<template id="local-card-template" type="text/cem-ml">
    {article @class=demo-card | {h2 | {$datadom.attributes.title}} {slot | Local fallback}}
</template>

<cem-element tag="cem-local-src-card" src="#local-card-template"></cem-element>
<cem-local-src-card title="Local src">Loaded from a same-document template.</cem-local-src-card>
```

External template selected by id:

```html
<cem-element
    tag="cem-external-src-card"
    src="./external-template-templates.html#external-card-template"></cem-element>
<cem-external-src-card title="External src">Loaded from a fetched support file.</cem-external-src-card>
```

External document as template:

```html
<cem-element tag="cem-external-document-card" src="./external-template-document.html"></cem-element>
<cem-external-document-card>Loaded from the whole external document.</cem-external-document-card>
```

External subtree selected by id:

```html
<cem-element
    tag="cem-external-subtree-card"
    src="./external-template-templates.html#external-subtree-template"></cem-element>
<cem-external-subtree-card>Loaded from an external subtree.</cem-external-subtree-card>
```

HTTP response AST stream:

```html
<cem-element tag="pokemon-list">
    <template type="text/cem-ml">
        {http-request @slice=page @url="./pokemon.json" @method=GET @content-type="application/json"}
        {cem:for-each @select="$datadom.slices.page.data.results" @as=pokemon |
            {button @type=button | {$pokemon.name}}
        }
    </template>
</cem-element>
<pokemon-list></pokemon-list>
```

The `datadom.slices.page.data` path addresses the queryable projection exposed by the resource slot. It is a
stream-derived CEM view over the AST stream/chunk contract, not a host JavaScript object, JSON serialization step, or
fully materialized AST payload.

## Related Documents

- [`cem-element` design](./cem-element-design.md)
- [`cem-element` WASM proposal](./cem-element-wasm-proposal.md)
- [CEM-ML resource lifecycle](./cem-ml-resource-lifecycle.md)
- [CEM Elements HTTP request resource design](./cem-elements-http-request-design.md)
- [Content-type switching BRD](./content-type-switch.md)
- [Custom-element adapter boundary](./custom-element-adapter-boundary.md)
- [Custom-element bridge template policy](./custom-element-bridge-template-policy.md)
