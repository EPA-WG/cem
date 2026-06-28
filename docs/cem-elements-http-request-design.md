# CEM Elements HTTP Request Resource Design

**Status:** Design accepted for staged implementation. Phase 1 is the
implementation-ready contract; later phases remain roadmap items.
**Primary use case:** substrate-backed `<http-request>` resource slices for
`<cem-element>` templates.
**Related docs:** [`cem-element` design](./cem-element-design.md),
[`cem-element` WASM proposal](./cem-element-wasm-proposal.md),
[`CEM-ML resource lifecycle`](./cem-ml-resource-lifecycle.md),
[`cem-element` external resource loading contract](./cem-element-src-loading-contract.md),
[`CEM-ML stack design`](./cem-ml-stack-design.md), and
[`CEM-ML UID and scoped CSS design`](./cem-ml-uid-and-scoped-css-design.md).

This document is the concrete `<http-request>` host-binding design. The base lifecycle, content-type negotiation,
parser/plugin dispatch, AST stream production, projections, engine diagnostics, hydration, and de-hydration are defined by
the [CEM-ML resource lifecycle](./cem-ml-resource-lifecycle.md). The
[`cem-element` external resource loading contract](./cem-element-src-loading-contract.md) defines the CEM Elements binding
layer that classifies `http-request` resources, acquires them, and passes expected content-type context plus metadata to
CEM-ML. This document only narrows that binding for HTTP request policy, resolver/loader hooks, request/response metadata,
cache identity, and verification fixtures.

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

The primitive should instead treat an HTTP response as a CEM-ML response/data resource. The host opens the request and
passes the response stream, metadata, expected content-type context, source identity, and policy identity to CEM-ML.
CEM-ML recognizes the response content type, parses the response stream into source-map-bearing AST events, and exposes
the resulting stream surface to templates.

The Fetch API itself can expose `Response.body` as a `ReadableStream` in modern
browsers. The design rejection is specifically against the common buffered
convenience path and DOMParser/JSON object handoff, not against Fetch as a transport
mechanism.

## 2. Principle

`<http-request>` is a resource declaration, not a rendered DOM element.

When it appears inside a CEM-ML template, it declares an external input stream. The
host binding removes the helper from light-DOM output, exposes a resource slot under
the data document, and hands the stream to CEM-ML. The render engine consumes CEM-ML
lifecycle state and AST stream surfaces, not a browser `Node`, `Response`,
JavaScript object, or raw string.

This gives one path for:

- browser rendering;
- worker/WASM rendering;
- SSR and edge rendering;
- CLI and test fixtures;
- source maps from template source and data source into final UI DOM.

The full streaming/source-map model is the architectural target. The first
implementation must keep the same host and CEM-ML boundaries, but it may buffer a
loaded response inside the CEM runtime while parser streaming and browser source-map
tooling are added later.

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
| `content-type` | Optional expected content-type context passed to CEM-ML when the response omits `Content-Type` or when the host wants strict validation. |
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
`cem.resource.http.unresolved_url` and the resource enters `state="failed"`.

## 4. Phase 1 Implementation Contract

Phase 1 is the browser-substrate implementation slice. It must establish the
correct resource boundary without requiring the full progressive parser, cache, SSR,
or debug-source-map stack.

### 4.1 Runtime Host API

`<http-request>` needs resource-specific host hooks rather than overloading the
existing `resolveModuleUrl(specifier, baseDocument)` helper. The runtime should
introduce a resource resolver and loader boundary with this shape:

```ts
interface CemResourceResolutionRequest {
  kind: "http-request";
  authoredUrl: string;
  baseUrl: string;
  declarationScopeId: string;
  method: string;
  headers: Record<string, string>;
  contextIdentity: string;
  expectedContentTypes?: readonly string[];
  expectedContentType?: string;
}

interface CemResourceResolution {
  authoredUrl: string;
  resolvedUrl: string;
  resolverIdentity: string;
  resourcePolicyStamp: string;
  contextIdentity: string;
  contentTypeHint?: string;
  integrity?: string;
}

interface CemHttpRequest {
  authoredUrl: string;
  resolvedUrl: string;
  resolverIdentity: string;
  resourcePolicyStamp: string;
  contextIdentity: string;
  method: "GET" | "HEAD";
  headers: Record<string, string>;
  credentials?: string;
  cache?: string;
  expectedContentTypes?: readonly string[];
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

The loader API is stream-shaped from the beginning. Phase 1 may materialize that
stream in CEM/WASM/runtime memory before CEM-ML produces a stream projection, subject
to policy limits and a diagnostic when the host cannot provide a true stream.

### 4.2 Phase 1 Resource Slot Shape

The resource slot envelope is required in Phase 1. The host binding owns request and
response metadata plus lifecycle state. The `data` field is produced by CEM-ML as a
CEM-QL-navigable AST stream handle or stream-derived projection, not as a live
browser object.

Initial CEM-ML projection expectations for fixtures:

- JSON objects expose object keys as fields; arrays expose ordered items; scalars
  expose their scalar value and type.
- XML and XHTML expose element name, attributes, text children, and child elements
  through the same AST/query surface used by parsed template/data documents.
- Text exposes a text document node with chunk/range metadata.
- Response metadata is plain serializable host metadata; no `Response`, `Headers`,
  `Document`, DOM node, or host object is stored in slices.

Phase 1 CEM-ML examples should use explicit resource paths such as
`$datadom.slices.page.data.results`. Broad legacy XPath rewrites such as `//results`
are compatibility follow-up work unless the existing CEM-QL implementation can
support them directly without a separate conversion pass.

### 4.3 Policy Defaults

The default implementation must be conservative:

- only `GET` and `HEAD` are accepted;
- unresolved bare specifiers fail with `cem.resource.http.unresolved_url`;
- unsupported response content types are reported as CEM-ML engine diagnostics and
  move the resource to `failed`;
- direct network access is host-policy controlled;
- test/demo fixtures should use host-provided local resources or explicitly
  allowed same-origin URLs, not live third-party network dependencies;
- response size, parse time, redirect count, credentials, and exposed metadata are
  bounded by host policy.

### 4.4 Async Render And Lifecycle Contract

An `http-request` declaration creates or updates one resource slice for the active
render revision. Its template-visible state MUST follow the portable lifecycle in
the [CEM-ML resource lifecycle](./cem-ml-resource-lifecycle.md).

Required Phase 1 behavior:

1. Initial render records a `declared`, `scheduled`, or `waiting` resource slot.
2. The host resolves the URL, authorizes the request, and opens it when scheduled.
3. Header metadata may update the slice while the resource is `in-progress`.
4. The host passes response bytes and metadata to CEM-ML; CEM-ML moves the resource
   through `streaming` when validated AST events are available and writes the AST
   stream handle or stream-derived projection.
5. Terminal success moves the resource to `loaded`; terminal failure moves it to
   `failed` with diagnostics.
6. The declaration rerenders against lifecycle or content revisions.
7. If request inputs change, the old request is aborted and later frames from it
   are ignored by revision id.
8. Render-tree diff/no-op protection must prevent DOM mutation when visible output
   is unchanged.

Tests must wait on an explicit resource-settled signal or runtime hook. They must
not rely on timing sleeps.

### 4.5 Source-Map Minimum

Phase 1 must preserve enough identity to add full data source maps later:

- each response resource gets a `SourceId` record;
- diagnostics include the `SourceId` and response/parser location when available;
- CEM-ML AST events or stream-derived projections may carry opaque source-map references;
- production DOM output is not required to expose source maps;
- browser debug sidecars and mixed template/data source-map trees are deferred to
  Phase 3.

### 4.6 SSR Boundary

Phase 1 is browser-substrate work. SSR and hydration behavior must not be broken,
but executing requests during SSR, preloaded resource ASTs, and client
revalidation are Phase 4 unless a narrower fixture is needed to prevent a browser
regression.

## 5. Resource Slot Shape

The resource slot is an envelope with lifecycle state, revision identity, request
metadata, response metadata, diagnostics, and the AST stream handle or stream-derived
projection produced by CEM-ML.

Logical shape:

```json
{
  "datadom": {
    "slices": {
      "page": {
        "kind": "http-request",
        "revision": "resource-revision-id",
        "state": "declared | scheduled | waiting | in-progress | streaming | loaded | failed",
        "contextIdentity": "context:...",
        "resourcePolicyStamp": "policy:...",
        "expectedContentTypes": ["application/json"],
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
        "data": "<AST stream handle or stream-derived projection>",
        "diagnostics": []
      }
    }
  }
}
```

`data` is not a JavaScript object in the engine contract. It is a CEM AST stream
handle or stream-derived projection produced by CEM-ML. Browser adapter debug views
may project a small JSON summary, but templates and transforms consume the AST stream
surface.

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

## 6. HTTP Binding Pipeline

The host binding pipeline is:

```text
http-request declaration
  -> scoped URL/module-map resolution
  -> host request policy
  -> HTTP transport stream
  -> response metadata and byte stream
  -> CEM-ML resource handoff
  -> CEM-ML lifecycle, content-type negotiation, parser/plugin dispatch, and AST stream
  -> resource slot revision
  -> template render/query engine
  -> render-plan patches
```

The normative Phase 1 host API is defined in
[4.1 Runtime Host API](#41-runtime-host-api). Later phases may add parser
capability flags, cache handles, or preload handles, but they must preserve the
same resolver/loader separation.

Browser hosts should implement `body` with `response.body.getReader()` when
available. If streaming bodies are unavailable, a host may fall back to
`arrayBuffer()` only when the response size is within policy and must emit a
diagnostic such as `cem.resource.http.streaming_unavailable`.

The host handoff to CEM-ML is logically:

```text
process_resource_stream(
  kind = "http-request",
  lifecycle_revision,
  source_id,
  expected_content_type_context,
  response_metadata,
  byte_stream,
  source_map_mode,
  host_policy_identity
) -> AstEventStream
```

CEM-ML owns parser selection, decoding, AST event production, stream-derived
projections, and materialization decisions. The HTTP binding only supplies the
stream, metadata, source identity, expected content-type context, and policy
identity.

## 7. HTTP Content-Type Metadata

The response `Content-Type` header is host metadata passed to CEM-ML. The optional
`content-type` attribute is expected content-type context passed to CEM-ML, not an
unsafe override.

Host binding rules:

1. Capture response `Content-Type` and charset parameters when exposed by the host.
2. Capture the authored `content-type` attribute as expected content-type context.
3. Preserve module-map or resolver content-type metadata when supplied.
4. Do not sniff in the host binding. Sniffing is a CEM-ML registered sniffer and host-policy decision.
5. Pass all metadata to CEM-ML with source identity and policy identity.

Eligible response formats are exactly those accepted by the active CEM-ML context and
registered parser/content-type set. Initial useful fixture coverage:

| Content type | AST surface |
| --- | --- |
| `application/json`, `text/json`, `*/*+json` | JSON object/array/scalar AST with key/value source ranges. |
| `application/xml`, `text/xml`, `*/*+xml` | XML AST with element/attribute/text source ranges. |
| `text/html` | HTML AST through the existing HTML tokenizer/lowering path. |
| `application/xhtml+xml` | XML/XHTML AST. |
| `text/cem-ml` | CEM-ML AST. |
| `text/plain` | Text document AST with one or more text chunks. |

Unsupported or mismatched types still populate request/response metadata. CEM-ML owns
the unsupported-content-type or mismatch diagnostic and moves the resource revision to
`failed`.

## 8. Streaming Query And Rendering Semantics

Not every query can produce stable UI before the whole response is available.

CEM-ML classifies resource consumers:

| Consumer shape | Streaming behavior |
| --- | --- |
| Forward-only `cem:for-each` over records/items in source order | May render incrementally as items arrive. |
| Simple field reads on the current streamed item | May render incrementally. |
| Whole-document reads, `count`, `last`, sorting, grouping, broad reverse lookups | Wait for resource completion or materialize a chunked AST store. |
| Failed, scheduled, waiting, or metadata-only UI | May render before body data arrives. |

When a query is not streaming-safe, the implementation should still avoid JS-level
buffering. CEM-ML may materialize the response inside the CEM/WASM AST chunk store
and render after `loaded`.

Render output is still transactional. Incremental resource output is delivered as
render-plan patch transactions tied to the active render revision. Stale transactions
from superseded requests are dropped.

## 9. Source Maps For Data

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

CEM-ML AST events and stream-derived projection nodes receive source-map stack
entries rooted in that response `SourceId`. The stack can include:

1. `HttpResourceFetch` frame: request URL, final URL, status, and response body
   byte range.
2. `ContentTypeTransform` frame: CEM-ML parser/plugin identity and decoded range.
3. `CemAstBuilder` frame: AST event or projection-node construction.
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

## 10. HTTP Binding To CEM-ML Resource Lifecycle

`http-request` uses the portable lifecycle from the
[CEM-ML resource lifecycle](./cem-ml-resource-lifecycle.md). HTTP-specific events map
to that lifecycle as follows:

```text
declared      authored request is captured in the resource slice
scheduled     request is accepted and queued by host policy
waiting       scheduler, dependency, cache, or transport availability blocks progress
in-progress   resolver, transport, cache read, or CEM-ML handoff is active
streaming     CEM-ML is producing validated AST events
loaded        CEM-ML reached end-of-stream and resource-dependent render work settled
failed        network, policy, content-type, parse, abort, or transform failure
```

A declaration instance has one active request per resource slice and render
revision. If `url`, `method`, headers, or policy-relevant inputs change:

- the prior request is aborted;
- its later frames are ignored by revision id;
- a new resource revision starts at `declared`;
- render no-op protection prevents DOM mutation if the visible output remains the
  same.

Request metadata may render before response data. Response headers may render before
the body reaches `loaded`. Body-derived UI renders according to the streaming query
rules.

## 11. Cache And Identity

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
- expected/provided content-type identity;
- policy stamp.

Render-plan identity includes:

- template artifact id;
- resource content revision;
- resource revision;
- source-map mode;
- privacy/export policy stamp.

The engine must not use raw response bodies as public IDs. Content hashes may be
used internally or in debug sidecars, subject to privacy policy.

## 12. Security And Privacy

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
- maximum response bytes and maximum CEM-ML processing time;
- maximum redirect count;
- allowed response content-type policy;
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

## 13. SSR And Hydration

SSR may handle `http-request` in two ways:

1. Execute the request during SSR, hand the response to CEM-ML, and render from the
   same resource lifecycle plus AST stream/projection model.
2. Receive a preloaded resource lifecycle state plus AST stream handle or
   stream-derived projection from the host and treat it as the response resource.

Hydrated browser runtime should trust SSR output when the data-island/resource
evidence is runtime-owned and valid. `connectedCallback` must not re-fetch merely
because an `http-request` declaration exists in a hydrated body.

Revalidation is explicit policy. If enabled, client revalidation creates a new
resource revision and patches only real DOM differences. If the response is the same
and the render plan is unchanged, no browser DOM mutation should occur.

Dynamic remote data is one of the allowed SSR/browser output differences. Tests
should use fixture URLs or host-supplied preloaded streams to prove identical output.

## 14. Legacy Migration

Legacy behavior:

```html
<http-request slice="page" url="..."></http-request>
<for-each select="//results">...</for-each>
```

New behavior:

- `<http-request>` lowers to a resource declaration.
- Its `url` resolves through the active scoped module-map resolver before any
  request opens.
- The response body is handed to CEM-ML as response content with expected
  content-type context, then exposed as an AST stream/projection.
- The resource is exposed under `datadom.slices.page`.
- Compatibility conversion rewrites broad legacy selectors to CEM-QL over the
  resource AST where possible.
- Unsupported legacy XPath constructs emit diagnostics instead of falling back to
  browser DOMParser objects.

The old standalone `http-request.js` companion element can remain published as a
browser shim. It is not the substrate path for CEM Elements templates.

## 15. Implementation Roadmap

### Phase 1: Loaded-response AST stream resource

- Implement the contract in [4. Phase 1 Implementation Contract](#4-phase-1-implementation-contract).
- Add resource declaration parsing for `http-request`.
- Add scoped URL/module-map resolution for `http-request @url`, including
  diagnostics for unresolved bare specifiers.
- Add host policy, resource resolver, and request loader hooks.
- Stream bytes through the loader boundary when possible.
- Materialize the loaded AST stream projection in CEM/WASM/runtime memory before
  rendering when the CEM-ML parser/query path is not streaming-capable yet.
- Expose request/response metadata and loaded `data` AST stream projection to CEM-QL.
- Add JSON and XML fixtures with diagnostic/source-id coverage.

This phase already removes JS object/DOMParser handoff and establishes the correct
contract, even if UI rendering waits for completion.

### Phase 2: Progressive AST stream consumption

- Replace Phase 1 buffering internals with CEM-ML-owned AST event streams where
  supported.
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

## 16. Verification Matrix

Required gates before Phase 1 is considered implemented:

- JSON response renders a `cem:for-each` list from the resource AST stream
  projection.
- XML response renders equivalent content through the same resource slot contract.
- Module-map URL resolution fixture proves `@scope/data/file.json` resolves in the
  owning `<http-request>` scope and that imported-template relative URLs use the
  imported template base URL.
- Response metadata renders before or with data without exposing live `Response`
  objects.
- Unsupported content type produces a diagnostic and stable `failed` state.
- Abort on URL change drops stale response frames.
- Resource-settled tests use runtime hooks instead of sleeps.
- Source-id and CEM-ML diagnostics are preserved internally for response data.
- Existing standalone `http-request.js` companion export/registration smoke tests
  continue to pass.

Required gates before the full design is considered implemented:

- Forward-only streamed resource records can render incrementally.
- Browser and SSR fixture with identical static response produce identical output.
- Debug mode can map rendered text/attribute output to both template source and
  response data source ranges.
- Production mode can omit or redact response source-map details by policy.
- Existing standalone `http-request.js` companion export/registration smoke tests
  continue to pass.
