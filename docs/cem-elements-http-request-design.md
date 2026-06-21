# CEM Elements HTTP Request Resource Design

**Status:** Draft design.
**Primary use case:** substrate-backed `<http-request>` resource slices for
`<cem-element>` templates.
**Related docs:** [`cem-element` design](./cem-element-design.md),
[`cem-element` WASM proposal](./cem-element-wasm-proposal.md),
[`CEM-ML stack design`](./cem-ml-stack-design.md), and
[`CEM-ML UID and scoped CSS design`](./cem-ml-uid-and-scoped-css-design.md).

## 1. Problem

The legacy `http-request.js` companion custom element performs a browser fetch,
buffers the response through convenience APIs such as `response.json()` or
`response.text()`, and then writes a JavaScript object or `DOMParser` document into
an element `value`. Legacy templates then query that value through DCE/XSLT-like
selection.

That model is not the right substrate for CEM Elements:

- the response is converted into host JavaScript data before the CEM-ML engine sees
  it;
- JSON, XML, HTML, and future content types each need separate browser-side
  projection code;
- `response.json()`, `response.text()`, and `DOMParser` require a full buffered
  response;
- source-map ranges for response data are lost before template transformation;
- edge/SSR and browser runtimes cannot share one data-processing contract.

The primitive should instead treat an HTTP response as a source document. The host
opens the request, CEM-ML recognizes the response content type, parses the response
stream into source-map-bearing AST events, and the template consumes that AST stream
as data.

The Fetch API itself can expose `Response.body` as a `ReadableStream` in modern
browsers. The design rejection is specifically against the common buffered
convenience path and DOMParser/JSON object handoff, not against Fetch as a transport
mechanism.

## 2. Principle

`<http-request>` is a resource declaration, not a rendered DOM element.

When it appears inside a CEM-ML template, it declares an external input stream. The
runtime removes the helper from light-DOM output and exposes a resource slot under
the data document. The render engine consumes a CEM AST stream, not a browser `Node`,
`Response`, JavaScript object, or raw string.

This gives one path for:

- browser rendering;
- worker/WASM rendering;
- SSR and edge rendering;
- CLI and test fixtures;
- source maps from template source and data source into final UI DOM.

## 3. Authoring Surface

Initial CEM-ML resource form:

```cem
{http-request
  @slice=page
  @url="https://pokeapi.co/api/v2/pokemon?limit=6"
  @method=GET
  @header-accept="application/json"}
```

The legacy HTML spelling remains a compatibility input and lowers to the same
resource declaration:

```html
<http-request
  slice="page"
  url="https://pokeapi.co/api/v2/pokemon?limit=6"
  method="GET"
  header-accept="application/json"></http-request>
```

Initial attributes:

| Attribute | Meaning |
| --- | --- |
| `slice` | Required resource slot name under `datadom.slices`. |
| `url` | Required request URL after normal template interpolation. |
| `method` | Defaults to `GET`. Initial resource primitive supports `GET` and `HEAD`; mutating methods are policy-gated follow-up work. |
| `header-*` | Request headers. Header names are the suffix after `header-`. |
| `content-type` | Optional expected/parser content type when the response omits `Content-Type` or when the host wants strict validation. |
| `cache` | Optional cache policy hint: `default`, `reload`, `no-store`, `force-cache`; exact host support is policy-controlled. |
| `credentials` | Optional credentials hint. Browser hosts still obey Fetch/CORS rules; SSR hosts must apply explicit policy. |

The resource may also be surfaced as a future canonical `cem:resource` form, but
`http-request` stays the compatibility-friendly primitive name for this work.

### 3.1 URL Resolution And Module Maps

`url` is a resource specifier in the owning `<http-request>` scope, not merely a
browser URL string.

After template interpolation, the runtime resolves the `url` value through the same
scoped resolver model used by URI-backed `<cem-element src>` templates and
`module-url` helpers:

```text
authored/interpolated url
  -> owner declaration/resource scope
  -> base URL
  -> active module/import map or host resolver
  -> substitution rules
  -> resource policy
  -> resolved request URL
```

Supported forms:

- absolute URLs allowed by scope policy;
- document-relative URLs resolved against the declaration document or imported
  template base URL;
- module-map/package specifiers, for example `@scope/data/pokemon.json`;
- host-resolver aliases, CDN manifest entries, or bundler manifest entries exposed
  through the same resolver contract;
- fragment-only URLs only when the host policy defines a local resource provider for
  them. They are not normal network HTTP requests.

Resolution must happen in the lexical/resource scope of the `<http-request>`
declaration. If a template is imported through a module-map entry, relative request
URLs inside that imported template resolve against the imported template's base, not
against the outer page. If a nested declaration overrides the module map or base URI,
the nested resource uses that inner resolver state.

The resolver result should include:

- `authoredUrl`: the interpolated value from the template;
- `resolvedUrl`: the final URL passed to the transport layer;
- optional `fragment` or local-resource selector;
- `resolverIdentity`: an opaque deterministic stamp for the effective module map,
  import map, manifest, or host resolver state;
- `resourcePolicyStamp`;
- optional content-type hint and integrity expectation.

Bare specifiers that cannot be resolved through the active module map must not fall
back to browser-relative URL parsing. They produce a diagnostic such as
`cem.resource.http.unresolved_url` and the resource enters `state="error"`.

## 4. Data Document Shape

The resource slot is an envelope with request metadata, response metadata, parser
state, diagnostics, and the parsed AST handle/stream.

Logical shape:

```json
{
  "datadom": {
    "slices": {
      "page": {
        "kind": "http-request",
        "state": "pending | headers | streaming | complete | error | aborted",
        "request": {
          "authoredUrl": "@scope/data/pokemon.json",
          "url": "...",
          "resolvedUrl": "https://cdn.example.test/data/pokemon.json",
          "resolverIdentity": "resolver:...",
          "method": "GET",
          "headers": { "accept": "application/json" }
        },
        "response": {
          "url": "...",
          "status": 200,
          "statusText": "OK",
          "ok": true,
          "redirected": false,
          "headers": { "content-type": "application/json" },
          "contentType": "application/json"
        },
        "data": "<AST stream or AST document handle>",
        "diagnostics": []
      }
    }
  }
}
```

`data` is not a JavaScript object in the engine contract. It is a CEM AST stream or
an AST document/chunk handle produced by the CEM-ML parser registry. Browser adapter
debug views may project a small JSON summary, but templates and transforms consume
the AST.

Legacy-style selection such as `//results` must be implemented by querying the
resource AST. CEM-ML templates should prefer explicit data-document paths:

```cem
{cem:for-each @select="$datadom.slices.page.data.results" @as=pokemon |
  {button |
    {$pokemon.name}
  }
}
```

Compatibility conversion may rewrite legacy DCE/XSLT selectors into equivalent
CEM-QL expressions over `datadom.slices.<slice>.data`.

## 5. Streaming Pipeline

The runtime pipeline is:

```text
template resource declaration
  -> scoped URL/module-map resolution
  -> host request policy
  -> HTTP transport stream
  -> content-type recognition
  -> charset/content decoding
  -> CEM-ML parser registry
  -> source-map-bearing AST event stream
  -> resource slot
  -> template render/query engine
  -> render-plan patches
```

Host API sketch:

```ts
interface CemHttpRequest {
  authoredUrl: string;
  url: string;
  resolverIdentity: string;
  resourcePolicyStamp: string;
  method: string;
  headers: Record<string, string>;
  credentials?: string;
  cache?: string;
  expectedContentType?: string;
  signal: AbortSignal;
}

interface CemHttpResponseHead {
  url: string;
  status: number;
  statusText: string;
  ok: boolean;
  redirected: boolean;
  headers: Record<string, string>;
  contentType: string | null;
}

interface CemHttpResourceLoader {
  open(request: CemHttpRequest): Promise<{
    response: CemHttpResponseHead;
    body: AsyncIterable<Uint8Array>;
  }>;
}
```

Browser hosts should implement `body` with `response.body.getReader()` when
available. If streaming bodies are unavailable, a host may fall back to
`arrayBuffer()` only when the response size is within policy and must emit a
diagnostic such as `cem.resource.http.streaming_unavailable`.

The CEM-ML engine owns parsing:

```text
parse_resource_stream(
  source_id,
  content_type,
  decoded_byte_stream,
  source_map_mode
) -> AstEventStream
```

AST events carry source ranges as they are produced. The engine does not wait for
the whole response unless the selected parser, query, or transform requires
materialization.

## 6. Content-Type Recognition

The response `Content-Type` header is the primary parser selector. The optional
`content-type` attribute is an expectation or fallback, not an unsafe override.

Rules:

1. If the response has a recognized `Content-Type`, use it.
2. If the response has no content type and `content-type` is present, use the
   declared value.
3. If both are present and incompatible, emit a diagnostic and do not parse the
   body unless host policy explicitly allows coercion.
4. Do not sniff by default. Sniffing is host policy, not the primitive default.
5. Charset parameters are honored for text content. Source-map byte ranges are
   recorded against the decoded response byte stream visible to the parser.

Eligible data formats are exactly those registered in the CEM-ML parser/content-type
registry. Initial useful set:

| Content type | AST surface |
| --- | --- |
| `application/json`, `text/json`, `*/*+json` | JSON object/array/scalar AST with key/value source ranges. |
| `application/xml`, `text/xml`, `*/*+xml` | XML AST with element/attribute/text source ranges. |
| `text/html` | HTML AST through the existing HTML tokenizer/lowering path. |
| `application/xhtml+xml` | XML/XHTML AST. |
| `text/cem-ml` | CEM-ML AST. |
| `text/plain` | Text document AST with one or more text chunks. |

Unsupported types still populate request/response metadata and set `state="error"`
with `cem.resource.http.unsupported_content_type`.

## 7. Streaming Query And Rendering Semantics

Not every query can produce stable UI before the whole response is available.

The engine classifies resource consumers:

| Consumer shape | Streaming behavior |
| --- | --- |
| Forward-only `cem:for-each` over records/items in source order | May render incrementally as items arrive. |
| Simple field reads on the current streamed item | May render incrementally. |
| Whole-document reads, `count`, `last`, sorting, grouping, broad reverse lookups | Wait for resource completion or materialize a chunked AST store. |
| Error/pending/headers-only UI | May render before body data arrives. |

When a query is not streaming-safe, the implementation should still avoid JS-level
buffering. It may materialize the response inside the CEM/WASM AST chunk store and
render after completion.

Render output is still transactional. Incremental resource output is delivered as
render-plan patch transactions tied to the active render revision. Stale transactions
from superseded requests are dropped.

## 8. Source Maps For Data

Each response is a source document with a `SourceId`.

A response `SourceId` includes:

- authored resource URL/specifier before resolver rewriting;
- final response URL after redirects;
- original requested URL;
- resolver identity and resource policy stamp;
- method;
- recognized content type;
- response identity hash when available;
- policy-controlled redaction state.

Every parsed node, key, value, attribute, and text segment receives a source-map
stack rooted in that response `SourceId`. The stack can include:

1. `HttpResourceFetch` frame: request URL, final URL, status, and response body
   byte range.
2. `ContentTypeTransform` frame: JSON/XML/HTML/CEM-ML parser and decoded range.
3. `CemAstBuilder` frame: AST node construction.
4. Template expression frame when data is inserted into rendered output.

Mixed output may have a source-map tree instead of a single linear stack. Example:

```cem
{p | {$pokemon.name} from {$datadom.slices.page.response.url}}
```

The rendered text node has:

- template literal ranges for `" from "`;
- data ranges for `pokemon.name`;
- response metadata ranges or synthetic metadata ranges for `response.url`;
- the template expression ranges that caused each insertion.

In dev/debug mode, rendered DOM nodes should expose source-map information through a
sidecar keyed by `data-cem-render-node-id` or by a runtime property such as
`node.cemSourceMap`. Existing lightweight attributes such as
`data-cem-source-frame` may keep a compact summary, but full mixed-content maps
belong in the sidecar to avoid large DOM attributes.

This is the key developer experience goal: final UI DOM can be traced both to the
template that rendered it and to the response byte ranges that supplied its data.

## 9. Request Lifecycle

State transitions:

```text
idle
  -> pending       request accepted and authorized
  -> headers       response headers received
  -> streaming     parser is producing AST events
  -> complete      body parsed and all resource-dependent render work settled

pending|headers|streaming
  -> aborted       request was superseded or host signal aborted
  -> error         network, policy, content-type, parse, or transform failure
```

A declaration instance has one active request per resource slice and render
revision. If `url`, `method`, headers, or policy-relevant inputs change:

- the prior request is aborted;
- its later frames are ignored by revision id;
- a new resource slot state is created;
- render no-op protection prevents DOM mutation if the visible output remains the
  same.

Request metadata may render before response data. Response headers may render before
the body completes. Body-derived UI renders according to the streaming query rules.

## 10. Cache And Identity

Resource identity is distinct from render identity.

Resource cache key inputs:

- method;
- authored URL/specifier;
- resolved URL;
- resolver identity or module-map identity;
- request headers that affect representation;
- credentials mode;
- cache mode;
- response `Vary` handling where available;
- content type;
- policy stamp.

Render-plan identity includes:

- template artifact id;
- data revision;
- resource revision;
- source-map mode;
- privacy/export policy stamp.

The engine must not use raw response bodies as public IDs. Content hashes may be
used internally or in debug sidecars, subject to privacy policy.

## 11. Security And Privacy

Browser hosts must obey platform Fetch, CORS, mixed-content, CSP, and credentials
rules. SSR/edge hosts must not silently bypass browser restrictions. They need an
explicit resource policy hook before opening requests.

Policy decisions:

- allowed URL schemes and origins;
- allowed module-map/package specifier scopes;
- resolver aliases that may issue network requests;
- allowed methods;
- allowed request headers;
- credentials mode;
- maximum response bytes and maximum parse time;
- maximum redirect count;
- allowed content types;
- cache/write policy;
- whether response URLs, query strings, headers, and source ranges can be exposed in
  DOM debug metadata or exported SSR artifacts.

Source-map data can reveal sensitive URLs and response content locations. Production
output may omit source-map sidecars or use redacted/hashed `SourceId` values.
Debug/dev output may expose full locations only when policy allows it.

The `authoredUrl` can reveal package aliases, private route names, tenant ids, or
local file layout. Hosts that expose source maps or SSR data islands should redact or
hash `authoredUrl`, `resolvedUrl`, and `resolverIdentity` according to the same
privacy policy.

## 12. SSR And Hydration

SSR may handle `http-request` in two ways:

1. Execute the request during SSR, parse the response into the same resource AST
   stream/document model, and render the output.
2. Receive a preloaded resource AST stream/document from the host and treat it as
   the response.

Hydrated browser runtime should trust SSR output when the data-island/resource
evidence is runtime-owned and valid. `connectedCallback` must not re-fetch merely
because an `http-request` declaration exists in a hydrated body.

Revalidation is explicit policy. If enabled, client revalidation creates a new
resource revision and patches only real DOM differences. If the response is the same
and the render plan is unchanged, no browser DOM mutation should occur.

Dynamic remote data is one of the allowed SSR/browser output differences. Tests
should use fixture URLs or host-supplied preloaded streams to prove identical output.

## 13. Legacy Migration

Legacy behavior:

```html
<http-request slice="page" url="..."></http-request>
<for-each select="//results">...</for-each>
```

New behavior:

- `<http-request>` lowers to a resource declaration.
- Its `url` resolves through the active scoped module-map resolver before any
  request opens.
- The response body is parsed as a content-typed AST stream.
- The resource is exposed under `datadom.slices.page`.
- Compatibility conversion rewrites broad legacy selectors to CEM-QL over the
  resource AST where possible.
- Unsupported legacy XPath constructs emit diagnostics instead of falling back to
  browser DOMParser objects.

The old standalone `http-request.js` companion element can remain published as a
browser shim. It is not the substrate path for CEM Elements templates.

## 14. Phased Implementation

### Phase 1: Completed-response AST resource

- Add resource declaration parsing for `http-request`.
- Add scoped URL/module-map resolution for `http-request @url`, including
  diagnostics for unresolved bare specifiers.
- Add host policy and request loader hooks.
- Stream bytes into the CEM-ML parser when possible.
- Materialize the AST in CEM/WASM memory before rendering.
- Expose request/response metadata and completed `data` AST to CEM-QL.
- Add JSON and XML fixtures with source-map-bearing diagnostics.

This phase already removes JS object/DOMParser handoff and establishes the correct
contract, even if UI rendering waits for completion.

### Phase 2: Progressive AST stream consumption

- Classify streaming-safe CEM-QL consumers.
- Render forward-only `cem:for-each` items incrementally.
- Emit batched render-plan patch transactions as AST items arrive.
- Add stale-response abort and no-op patch fixtures.

### Phase 3: Debug source-map UI

- Add resource source-map sidecars keyed by render-node id.
- Expose template plus data source-map stacks/trees to browser dev tooling.
- Add fixture coverage proving rendered DOM maps to both template source ranges and
  HTTP response data ranges.

### Phase 4: Cache, SSR, and service-worker integration

- Add resource cache identity and policy stamps.
- Add SSR preloaded-resource support.
- Add hydration trust/revalidation fixtures.
- Add optional service-worker/content-addressed artifact integration.

## 15. Verification Matrix

Required gates before the TODO is considered implemented:

- JSON response renders a `cem:for-each` list from streamed AST data.
- XML response renders equivalent content through the same resource slot contract.
- Module-map URL resolution fixture proves `@scope/data/file.json` resolves in the
  owning `<http-request>` scope and that imported-template relative URLs use the
  imported template base URL.
- Response metadata renders before or with data without exposing live `Response`
  objects.
- Unsupported content type produces a diagnostic and stable error state.
- Abort on URL change drops stale response frames.
- Browser and SSR fixture with identical static response produce identical output.
- Debug mode can map rendered text/attribute output to both template source and
  response data source ranges.
- Production mode can omit or redact response source-map details by policy.
- Existing standalone `http-request.js` companion export/registration smoke tests
  continue to pass.
