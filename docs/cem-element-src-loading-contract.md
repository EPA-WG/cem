# cem-element src Loading Contract

Status: Draft · Type: Runtime Contract

This document defines how `<cem-element src="...">` loads declaration templates from URLs and fragments. The same
contract applies to the compatibility `<custom-element>` adapter because that adapter registers declarations through the
`CemElementRuntime` substrate.

The contract covers these authoring forms:

```html
<cem-element tag="x-card" src="#local-template"></cem-element>
<cem-element tag="x-card" src="./cards.html"></cem-element>
<cem-element tag="x-card" src="./cards.html#card-template"></cem-element>
```

## Scope

`src` is a declaration-template input, not an arbitrary DOM include. Loading a `src` MUST produce one declaration
template artifact before any produced element instance renders.

The supported reference forms are:

- `src="#id"` — resolve `id` in the declaring document.
- `src="url"` — fetch and use the loaded document as the declaration template.
- `src="url#id"` — fetch `url`, then use the referenced subtree as the declaration template.

For a fragment target:

- If the target is a `<template>`, its template content is used.
- If the target contains a direct child `<template>`, that child template is used.
- Otherwise the target element subtree itself is wrapped as the declaration template.

For `src="url"` without a fragment:

- If the fetched document body has content, the body child nodes are used.
- Otherwise the document element is used.
- If the only meaningful node is a `<template>`, that template is used directly.

## URL Resolution And Module Maps

Relative URL-like specifiers MUST resolve against the declaring document's `baseURI`, including any active `<base>`.
Absolute URLs use their own origin and scheme.

Bare specifiers, such as `@scope/package/card.html`, MUST NOT be guessed by the default runtime. They require a
host-provided resolver/loader such as `loadSrcDocument`. That resolver owns import-map/module-map lookup and package
policy.

Resolution order is:

1. Parse `src` into document specifier and optional fragment.
2. Resolve the document specifier through the host module-map resolver when it is a bare specifier.
3. Resolve URL-like specifiers against the declaring document base URL.
4. Fetch or otherwise load the resolved document under host resource policy.
5. Parse and select the full document or fragment target.

The cache identity for a loaded source MUST include at least:

- declaring document base URI;
- original document specifier;
- resolved URL or module-map result;
- module-map/resolver identity when a host resolver is used;
- resource-policy stamp when security policy can change the bytes returned.

The current browser runtime caches external source documents per runtime instance by declaring document `baseURI` and
specifier path. Hosts that add module-map or security-policy resolution SHOULD include those identities in their
`loadSrcDocument` behavior or use separate runtime instances when policy differs.

## Content Type

External `src` loading has two content-type concepts:

- **Expected content type** — what the declaration boundary is willing to ingest.
- **Provided content type** — what the loaded resource actually supplies.

Expected type may come from:

- the declaration/runtime policy;
- a host loader policy for a module-map entry;
- a future explicit declaration marker;
- the default declaration-template expectation when no narrower type is supplied.

Provided type is derived in this order:

1. HTTP `Content-Type` response header or equivalent host loader metadata.
2. Explicit target marker such as `<template type="text/cem-ml">` or `<template lang="custom-element-xslt">`.
3. File extension fallback, only when the host permits extension fallback.
4. Structural sniffing, only for the bounded built-in cases below.

Supported built-in forms are:

- `text/cem-ml` / `application/cem-ml` — canonical CEM-ML template source.
- HTML/XHTML/XML DOM subtrees — parsed as declaration-template source.
- Legacy custom-element HTML+XSLT templates — lowered by the CEM engine compatibility path.
- SVG and MathML subtrees — accepted as DOM subtrees when selected from an HTML/XML document.

Mismatch handling MUST be explicit:

- If provided type is compatible with expected type, load continues.
- If provided type can be converted to the mandatory CEM template artifact, load continues and diagnostics MAY record the
  conversion.
- If provided type is unsupported, unsafe, or incompatible with expected type, load MUST fail closed with diagnostics.
- Silent fallback to a different content type is not allowed.

## Mandatory CEM-ML AST Load

An external source MUST cross the runtime boundary as a CEM-owned template artifact before render. There is no raw
remote-DOM append path.

The required processing shape is:

1. Load bytes/text under URL, module-map, and security policy.
2. Parse according to the selected content type/syntax.
3. Select the document, template, or subtree.
4. Convert the selected source into the CEM declaration-template model.
5. Compile or lower it to the CEM-ML/CEM-QL render boundary.
6. Render produced element instances from the compiled artifact and instance data island.

Canonical CEM-ML templates are compiled by the CEM-ML/CEM-QL engine. Legacy custom-element templates are transpiled to
canonical CEM-ML and then rendered by the same engine. DOM-authored templates are read into the serializable template
source model and projected through the substrate; they still cross the CEM processing boundary and do not execute as
remote page DOM.

Build, SSR, worker, or edge modes SHOULD persist or exchange the compiled CEM artifact rather than reparsing arbitrary
remote content at render time.

## Security Context

External `src` loading is a trust-boundary crossing. The runtime MUST treat loaded content as data until it has passed
content-type, parser, and template-artifact checks.

Security requirements:

- Fetching MUST obey browser origin, CORS, CSP, and credential rules, or stricter host resource policy.
- A host loader MUST define whether credentials are included. The default policy SHOULD be same-origin credentials only
  and no cross-origin credentials unless explicitly configured.
- Bare module-map specifiers MUST resolve only through an authorized host resolver.
- Redirects, opaque responses, and MIME mismatches SHOULD be rejected unless the host policy explicitly accepts them.
- Script elements in loaded HTML MUST NOT execute merely because the document was loaded as a template source.
- Inline event handler attributes and script-bearing URLs MUST be rejected or sanitized according to the active
  declaration security policy before render.
- Styles are template output, not loader authority. Scoped CSS processing may rewrite or diagnose styles, but loading a
  source file MUST NOT grant additional CSS privilege beyond the produced render.
- Diagnostics MUST be attached to the declaration or instance that requested the load.

The compatibility `<custom-element>` adapter may preserve author fallback/payload children while a `src` declaration
loads, but those children are not treated as declaration template source when `src` is present.

## Diagnostics

The runtime SHOULD report these categories with stable diagnostic codes:

- `cem-element.src_load_failed` — the document could not be loaded.
- `cem-element.src_target_missing` — `url#id` did not resolve to a selectable target.
- `cem-element.src_local_target_missing` — `#id` did not resolve in the declaring document.
- `cem-element.src_content_type_mismatch` — provided content type is incompatible with expected type.
- `cem-element.src_content_type_unsupported` — no parser/converter exists for the provided type.
- `cem-element.src_module_resolution_failed` — bare/module-map specifier could not be resolved.
- `cem-element.src_security_rejected` — resource policy rejected the load or selected subtree.
- `cem-element.src_artifact_compile_failed` — selected source could not be compiled/lowered to the CEM artifact boundary.

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

## Related Documents

- [`cem-element` design](./cem-element-design.md)
- [`cem-element` WASM proposal](./cem-element-wasm-proposal.md)
- [Content-type switching BRD](./content-type-switch.md)
- [Custom-element adapter boundary](./custom-element-adapter-boundary.md)
- [Custom-element bridge template policy](./custom-element-bridge-template-policy.md)
