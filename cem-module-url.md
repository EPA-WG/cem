# `cem-module-url` scoped module resolution

**Status:** Shared resolver, CEM-QL/XPath one- and two-argument functions, browser
WASM bridge, page import-map root, static local `module-map` prelude, selector bridge
for descendant browser contexts, and canonical template resource control implemented.
The standalone runtime-resolution schema identity and automatic CLI/SSR capability
construction remain integration work.

**Related contracts:**

- [`docs/cem-ml-stack-design.md`](docs/cem-ml-stack-design.md) defines context-scope
  inheritance, owner overrides, URL policy, and resource limits.
- [`docs/cem-element-design.md`](docs/cem-element-design.md) defines declaration
  source identity, imported-template base URLs, transient resource helpers, and
  light-DOM rendering.
- [`docs/cem-elements-http-request-design.md`](docs/cem-elements-http-request-design.md)
  defines the corresponding scoped resource-resolution boundary for HTTP resources.
- [`packages/cem_ml/schema-packages/module-map-v3/v1/README.md`](packages/cem_ml/schema-packages/module-map-v3/v1/README.md)
  is the latest accepted deployment module-map contract at the time of this design.

## 1. Purpose

`cem-module-url` resolves a module or resource specifier in the context that owns the
instruction and publishes the resulting absolute URL into a template slice:

```cem
{cem-module-url @slice=logoUrl @src="@example/assets/logo.svg"}
```

The XML/HTML parity spelling is:

```html
<cem-module-url slice="logoUrl" src="@example/assets/logo.svg"></cem-module-url>
```

An optional typed `referrer` selects an explicit module referrer:

```cem
{cem-module-url
  @slice=workerAsset
  @src="./worker.wasm"
  @referrer="@example/workers/worker.js"}
```

```html
<cem-module-url
  slice="workerAsset"
  src="./worker.wasm"
  referrer="@example/workers/worker.js"></cem-module-url>
```

A literal or interpolated `referrer` is a string. A whole CEM-QL attribute expression
may instead return exactly one string, `anyURI`, or CEM AST node. A node referrer uses
that node's recorded resolution scope; it is not stringified to a node identifier.

Browser-authored CEM-ML also has a clone-safe selector bridge for the node case:

```cem
{cem-module-url
  @slice=childAsset
  @src="@child/asset"
  @referrer-selector="cem-child-card"}
```

`referrer-selector` is evaluated against the current produced DCE's committed light-DOM
subtree. It MUST match exactly one rendered descendant with a live CEM resolution
context. It is mutually exclusive with `referrer`; invalid selectors, zero/multiple
matches, and matches without a live context fail resolution. Selecting a descendant
does not bypass context authorization: the shared resolver still proves that the
selected context is current-or-descendant. The selector is an authoring bridge for the
browser processing protocol, not a CEM-QL/XPath node identity and not the component's
CSS `scope` attribute.

`cem-module-url` is the only canonical name. This design does not define `module-url`
as an alias.

The instruction is transient. It is resolved after template interpolation, removed
from visible output, and represented in the instance resource state. A successful
resolution writes the serialized absolute URL string to
`datadom.slices.<slice>`. A failed resolution writes no value and removes a stale value
from a previous resource revision; an authored unresolved specifier is never exposed as
if it were a usable URL.

The resolver MUST NOT call `import.meta.resolve()` as its primary or fallback
implementation. In a browser, `import.meta.resolve()` uses the URL and hidden import-map
state of the JavaScript module containing the call. Those are not necessarily the URL
or module-map stack of the CEM template that owns `cem-module-url`. The equivalent CEM
operation therefore needs an explicit, scope-aware resolver.

## 2. Module maps and browser import maps

The terms are related but not interchangeable:

- A **CEM module map** is the portable, host-neutral resolution model. It can carry
  module imports, typed non-module resources, URL-scoped mappings, source identity,
  content type and integrity metadata, and a place in the CEM context hierarchy.
- A browser **import map** is a page-level JSON input and browser projection. Its
  `imports` and `scopes` participate in JavaScript module resolution, but it cannot
  express CEM's typed `resources` collection or the lexical CEM context stack.

There are not two competing resolution systems. The browser adapter parses page import
maps into the outermost CEM module-map frame. CLI and SSR hosts load the equivalent
root frame through `rootScope.moduleMap`. Template-local maps add inner frames. The
same resolver consumes every frame.

This normalization explains global override behavior: the page import map does not
override a separate module map after resolution. It *is* the outer module-map frame,
and outer frames are authoritative over inner frames.

Existing module-map schemas v1 through v3 retain their deployment meanings:

- their exact `imports` entries can participate in runtime URL resolution;
- v2/v3 typed `resources` entries can participate in `cem-module-url` resolution;
- their graph lowering, copying, rewriting, and no-discovery rules do not change;
- prefix mappings and browser-style `scopes` remain invalid in those schema versions.

A new versioned **runtime-resolution module-map profile** is required for lexical
preludes, prefix mappings, and `scopes`. Its proposed schema identity is
`https://cem.dev/ns/data/module-map/runtime-resolution/1`. Using a distinct profile
prevents runtime resolution features from silently broadening the v1-v3 deployment
contract. The profile remains under the module-map family and is accepted anywhere a
root or context `moduleMap` identity is accepted.

## 3. Context and base URL

Every CEM parsing, template, handoff, or embedded-content context has an effective
resolution frame:

```text
CemResolutionFrame {
  contextId,
  parentContextId?,
  baseUrl,
  moduleMap?,
  moduleMapBaseUrl?,
  moduleMapIdentity?,
  resolverIdentity,
  resourcePolicyStamp
}
```

The base URL is source-owned:

| Owning context | Effective base URL |
| --- | --- |
| Browser document root | `document.baseURI`, frozen when the CEM root is created |
| Inline declaration/template in the page | The browser document root base URL |
| Same-document fragment declaration | The browser document root base URL |
| External template or declaration | The final resolved response URL, without its selector fragment |
| Nested imported content | The nested resource's final resolved source URL |
| CLI/SSR root | Effective `rootScope.baseUri` |

Redirects therefore affect the base of relative references inside the loaded resource.
A host loader MUST retain its final URL; the authored `src` alone is insufficient.

The active base for a `cem-module-url` request is the innermost owning context's
`baseUrl`. A mapping target, however, is resolved against the source URL of the map
that declared the mapping. This matches browser import-map address handling and keeps a
shared outer map stable when it is used by templates from different locations.
`moduleMapBaseUrl` records that final map source URL when it differs from the context's
`baseUrl`; inline maps leave it absent and inherit `baseUrl`.

Resolution scope is separate from `CemDeclarationScope`. The latter owns declaration
registration and lifetime. It MUST NOT be expanded into a bag of live resolver or
processing state. A compiled template or resource control instead carries an opaque
resolution-context handle associated with the CEM context stack.

Hosts also retain a parent-linked resolution-context tree and an opaque association
from every scope-bearing AST node to its owning resolution-context handle. The
association is representation metadata, not an inference from the node's source URL:
multiple inline scopes may share one base URL while declaring different local maps.
Attribute, text, and other non-scope nodes inherit the handle of their containing
scope. Synthetic, detached, or foreign nodes may have no handle and cannot be used as
module referrers.

## 4. Local module-map declaration

### 4.1 CEM-ML prelude

The default local form is a `module-map` prelude in CEM-ML:

```cem
{module-map |
  {import @specifier="@example/ui/" @target="./vendor/ui/"}
  {resource
    @specifier="@example/logo"
    @target="./assets/logo.svg"
    @content-type="image/svg+xml"}
  {scope @prefix="./feature/" |
    {import @specifier="chart" @target="./feature/chart.js"}
  }
}

{cem-module-url @slice=uiModule @src="@example/ui/button.js"}
{cem-module-url @slice=logoUrl @src="@example/logo"}
```

The prelude modifies its containing context; it does not produce a DOM node and does
not create a second implicit child context. Imported templates and other established
CEM handoffs already create child contexts, where another prelude can be declared.

Rules:

- A context has at most one `module-map` prelude.
- The prelude MUST occur with the context's other header/prelude declarations and
  before its first output or resource instruction.
- `import` and `resource` require non-empty `specifier` and `target` attributes.
- A `resource` may additionally carry `content-type` and `integrity` metadata.
- `scope @prefix` is resolved against the module map's own base URL and applies only
  when it is an exact match for the request referrer URL or a trailing-slash URL prefix
  of that referrer.
- A duplicate key within `imports`, within `resources`, or across both collections in
  one specifier map is invalid. Source order does not select a winner.
- An exact key may be any normalized specifier. A prefix key MUST end in `/`, and its
  target MUST also end in `/`.
- A target MUST resolve to a URL. Targets are not bare specifiers and are not
  recursively passed through the module-map stack.

### 4.2 Optional JSON form

JSON is permitted when a host already owns an import-map/module-map document or when an
inner context explicitly selects the JSON content handoff. The governed runtime shape
is equivalent to:

```json
{
  "$schema": "https://cem.dev/ns/data/module-map/runtime-resolution/1",
  "imports": {
    "@example/ui/": "./vendor/ui/"
  },
  "resources": {
    "@example/logo": {
      "path": "./assets/logo.svg",
      "contentType": "image/svg+xml"
    }
  },
  "scopes": {
    "./feature/": {
      "imports": {
        "chart": "./feature/chart.js"
      }
    }
  }
}
```

An inner context may embed such content through an explicitly typed CEM content
handoff or reference it with `module-map @src`. Relative entries in an external JSON
map resolve against the map resource's final URL, not the template that references the
map.

Page `<script type="importmap">` content retains the standard flat browser shape under
each `scopes` member. The browser adapter normalizes those flat members into the
runtime profile's scoped `imports`; a page import map has no `resources` projection.

### 4.3 Required demonstration/use-case matrix

[`packages/cem-elements/demo/module-url.html`](packages/cem-elements/demo/module-url.html)
is the executable browser contract for the distinctions below:

| Input | Practical use | Required observation |
| --- | --- | --- |
| Relative `src` | Asset colocated with a declaration/template file | Resolves against the effective source or explicit referrer base |
| Bare/module `src` | Stable package/resource name whose deployed CDN, version, or hashed filename is host-owned | Resolves through applicable scopes and the owner-first frame stack |
| Absolute `src` | Already-published URL or non-hierarchical URL such as `data:` | Remains absolute when no normalized URL key maps it |
| Relative URL `referrer` | Model a referring module beside/under the current source | Resolves once in the current stack, then selects URL scopes |
| Bare/module `referrer` | Model a referring module whose deployment location is itself mapped | Resolves the referrer once, then uses its URL for base and scopes |
| Absolute URL `referrer` | Model an external or virtual referring module without granting another local context | Uses that URL directly for base and scopes |
| Descendant node referrer | Resolve as code/data owned by a rendered child DCE | Selects the child's full frame stack and base after descendant authorization |

The fixture therefore contains the scalar `3 src × 3 referrer` cross-product and a
node-referrer row for all three `src` forms. It also renders one local-map component
naked and inside a wrapper that maps the same key differently. The naked instance uses
its inner map; the wrapped instance uses the wrapper's winning outer entry; and a
wrapper lookup for an inner-only key succeeds only after selecting the child node.

These cases correspond to native import-map uses such as stable logical names for
hashed assets and referrer-sensitive dependency versions. Native import-map scope
selection is most-specific within one map. CEM retains that behavior within each frame
and adds owner-first precedence between frames.

## 5. Resolution algorithm

The algorithm intentionally combines native import-map matching *within* each frame
with CEM's owner-authoritative context hierarchy *between* frames.

### 5.1 Select the current context and referrer

Every request has an owning context handle and MAY carry one explicit referrer whose
value is either a URL-like scalar or a scope-bearing AST node.

The current context is surface-specific:

- a `cem-module-url` instruction uses the instruction node's owning template context;
- CEM-QL and XPath use the dynamic context node's recorded resolution context;
- when query focus is absent, atomic, or not scope-bearing, the evaluation host's
  owning context is the fallback.

An explicit string or `anyURI` referrer does not select a different CEM context:

1. Trim the referrer. An empty value is invalid.
2. If it parses as an absolute URL, preserve that URL as the effective referrer without
   applying a module-map entry to the referrer itself.
3. Otherwise resolve it once as a module specifier using the current context and its
   default base. This permits both relative and bare mapped referrers.
4. Use the resulting absolute URL as `activeBaseUrl` for target normalization and as
   the referrer URL for module-map `scopes`. Keep the current context's frame stack;
   the URL does not grant access to a different context's local maps.

An explicit node referrer selects its recorded context. The host MUST prove that the
selected context is the current context or its descendant in the same registered
context tree. Ancestor, sibling, foreign-tree, detached, destroyed, and unscoped nodes
are rejected. On success, the selected context's complete root-to-node frame stack is
used and its innermost base URL becomes `activeBaseUrl`.

For `cem-module-url`, `referrer-selector` supplies this node after the current render
patch is committed, so a newly connected descendant DCE has installed its compile-time
local map before selection. CEM-QL and XPath continue to pass typed node values
directly and do not evaluate CSS selectors.

The intermediate URL produced for a non-absolute scalar referrer is subject to the
effective resource policy before it can be used. Referrer resolution is bounded to one
contextual module-resolution pass; it cannot recursively supply another referrer and
never fetches the referenced module.

When no explicit referrer is supplied, the current context's innermost base URL is
`activeBaseUrl`.

### 5.2 Normalize the target specifier

After template interpolation and referrer selection:

1. Trim `src` or the language-level specifier. An empty result is invalid.
2. Classify and URL-parse the specifier:
   - a valid scheme form starts with `scheme:`; `://` is not required;
   - `//`, `/`, `./`, and `../` are URL-like;
   - query-only and fragment-only references are URL-like CEM resource references;
   - every other value is initially a bare specifier.
3. Resolve a URL-like value against `activeBaseUrl` and serialize it as
   `normalizedSpecifier`. A bare value keeps its authored spelling as
   `normalizedSpecifier`.

Query-only and fragment-only references are an intentional CEM resource extension to
the narrower JavaScript module-specifier spelling. They still enter map matching after
normalization, so an outer map can remap them by their absolute normalized URL.

### 5.3 Walk frames outer first

Walk the active frames from the document/run root toward the owning inner context. For
each frame that has a module map:

1. Use `activeBaseUrl`, not the map's own URL, as the referrer for selecting applicable
   `scopes`.
2. Visit applicable scoped specifier maps from most-specific to least-specific, then
   visit the frame's unscoped map.
3. Within one specifier map, consult `imports`, then `resources`.
4. Within a collection, try an exact key before the longest matching trailing-slash
   prefix.
5. On a valid match, resolve and return immediately. Append the suffix after a prefix
   key to the prefix target and reject a result that backtracks outside the mapped
   target prefix.
6. If a matched address is null, invalid, explicitly blocked by the source format, or
   denied by any frame in the selected context's effective policy stack, fail
   immediately. A blocked outer match MUST NOT fall through to another collection or
   an inner frame.
7. If the frame contains no match, continue inward.

The hierarchy rule is stronger than match specificity across frames. For example:

```cem
<!-- Outer/page frame -->
{module-map |
  {import @specifier="pkg/" @target="https://cdn.example/safe/"}
}

<!-- Inner/template frame -->
{module-map |
  {import @specifier="pkg/special" @target="./local-special.js"}
}
```

Resolving `pkg/special` uses `https://cdn.example/safe/special`. The outer prefix match
wins before the resolver considers the inner exact match. This is deliberate and
implements the general CEM rule that an owner context may constrain or override its
descendants.

This differs from applying all maps as one browser import map. The WHATWG algorithm
orders applicable URL scopes and prefixes by specificity. CEM preserves those rules
inside a frame, but evaluates context ownership first. See the
[HTML module-specifier resolution algorithm](https://html.spec.whatwg.org/multipage/webappapis.html#resolve-a-module-specifier)
for the native matching behavior being reused.

### 5.4 Fallback and policy

If every frame misses:

- return the normalized absolute URL for a URL-like input;
- fail an unmapped bare specifier.

There is no package-manager search, registry lookup, Node `node_modules` walk, browser
`import.meta.resolve()` call, or other implicit host fallback. A host that needs such a
mapping must materialize it into the root module map so browser and SSR behavior stays
portable and inspectable.

Before publishing the URL, apply the effective resource policy, including allowed
schemes, origin restrictions, substitution rules, and parent-enforced denials. Policy
failure publishes no slice value.

## 6. Browser root contract

The browser adapter creates one immutable root resolution context:

1. Capture `document.baseURI`.
2. Read inline `<script type="importmap">` elements in document order.
3. Parse and normalize their `imports`, `scopes`, and applicable integrity metadata.
4. Merge them with first-definition-wins conflict behavior, matching native multiple
   import-map merging where it is observable from authored maps.
5. Hash the normalized result and install it as the root module-map identity.

The snapshot is frozen when the CEM root runtime is created. Import-map elements added
or changed later do not mutate that root, invalidate compiled artifacts, or change
previous resolutions. A host must create a new root context to use a new map.

Automatic capture cannot inspect the browser's hidden resolved-module set. Pages that
interleave module execution and later import maps can therefore make rules that the
browser itself ignored but that remain visible in DOM text. Portable pages SHOULD put
their CEM-visible import maps before module execution. A host needing exact control for
an interleaved page MUST pass an explicit normalized root module map; explicit root
input replaces, rather than ambiguously merges with, automatic page capture.

The same constraint applies to dynamic `<base>` mutation: the browser adapter freezes
the effective `document.baseURI` and map addresses at root creation.

## 7. CLI and SSR root contract

CLI and SSR use the existing root-scope concepts:

```text
rootScope.baseUri
rootScope.moduleMap
```

`moduleMap` remains an explicit map resource identity. It may reference:

- an existing module-map v1-v3 document, with its existing exact-entry semantics; or
- the runtime-resolution profile defined here.

The `moduleMap` resource is loaded through `ResolvePurpose::ModuleMap`. Its final
resolved URI becomes the base for its relative entries. Its normalized entries hash,
resolver identity, diagnostics, and provenance are retained in
`NormalizedModuleMapIdentity` and the run context.

SSR MUST use the effective deployed/runtime map rather than assuming that a source
deployment map and page destination map have interchangeable target URLs. Given
equivalent root base and normalized map inputs, browser and SSR resolution MUST produce
the same URL, match provenance, diagnostic disposition, and resolver identity.

Serialized server render/hydration state includes the effective resolution-context
identity. A client with a different base, module-map hash, or resource-policy stamp
must not silently reuse the server's resolved resource values.

## 8. Shared language-level resolution

Module URL resolution is a language capability, not behavior owned by the
`cem-module-url` rendering instruction. CEM-QL queries and CEM-owned XPath expressions
need to resolve the same specifier under the same active context. All three surfaces
MUST call one operation and MUST NOT reproduce matching, fallback, or policy logic in
their adapters.

### 8.1 Ownership and shared Rust boundary

The resolver core belongs in `cem_ml`, below both `cem_ql` and `cem-elements`:

```text
cem_ml scoped module resolver
  ├── CEM template `cem-module-url` resource control
  ├── CEM-QL `module_url(...)`
  └── XPath `cem-ql:module-url(...)`
```

This dependency direction is required because the native XPath evaluator is owned by
`cem_ml`, `cem_ql` already depends on `cem_ml`, and browser/SSR hosts must use the same
portable resolver. The shared operation is conceptually:

```rust
pub trait CemModuleUrlResolver: Send + Sync {
    fn resolve_module_url(
        &self,
        request: &CemModuleUrlResolutionRequest,
    ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError>;
}

pub struct CemModuleUrlResolutionRequest {
    pub purpose: CemModuleUrlResolutionPurpose,
    pub authored_specifier: String,
    pub current_context: CemResolutionContextHandle,
    pub referrer: Option<CemModuleUrlReferrer>,
    pub source_map: SourceMapStack,
}

pub enum CemModuleUrlReferrer {
    Url(String),
    Context(CemResolutionContextHandle),
}

pub enum CemModuleUrlResolutionPurpose {
    TemplateSlice,
    CemQl,
    XPath,
}
```

`CemResolutionContextHandle` selects an immutable, preloaded frame stack and has a
registered parent link. It is not a base-URL string and is not interchangeable with
`QueryContextScope`, a browser `Document`, or a resolver-policy stamp. Surface adapters
translate a typed AST node to `CemModuleUrlReferrer::Context`; the resolver core does
not depend on a particular AST representation. The current handle supplies the
authority boundary, while a permitted descendant handle supplies the selected base and
map frames described in §3-§5.

The operation is pure resolution:

- it does not fetch the resolved target;
- it does not load a module map lazily;
- it does not test whether the target exists;
- it does not execute or import JavaScript;
- it does not consume an external-fetch budget;
- it does poll the caller's normal operation-control boundary before and after the
  call when invoked during query or XPath evaluation.

Hosts load and normalize map resources before evaluation and install the resulting
context handle as a capability. Keeping the call non-I/O makes one deterministic
implementation usable by the current native and WASM evaluators. Future asynchronous
map loading remains a context-construction operation, not an overload of
`module_url()`.

### 8.2 CEM-QL surface

CEM-QL exposes contextual one- and two-argument standard-library functions:

```cemql
module_url("@example/assets/logo.svg")
module_url("./worker.wasm", "@example/workers/worker.js")
module_url("./asset.css", $component_node)
```

Their canonical signatures are:

```text
module_url(specifier: string | anyURI) -> anyURI
module_url(specifier: string | anyURI, referrer: string | anyURI | node) -> anyURI
```

The function has stdlib identity `cem:stdlib/modules#module_url`. It is available as a
built-in unqualified function, like `read()`, and MAY also be reached through an
explicit import alias for `cem:stdlib/modules`. Underscore spelling follows CEM-QL's
Rust-first function naming; `module-url` is not valid CEM-QL function spelling.

Evaluation rules:

1. Evaluate and atomize the specifier. Require exactly one `string` or `anyURI`; an
   empty or multi-item sequence is a type error.
2. Obtain the current resolution handle from a scope-bearing context node, falling back
   to the evaluation host's owning handle for atomic, absent, or unscoped focus.
3. If present, evaluate the referrer without prematurely atomizing nodes. Require
   exactly one string, `anyURI`, or scope-bearing node.
4. Convert a scalar to `CemModuleUrlReferrer::Url`. Convert a node through the host's
   node-to-context association to `CemModuleUrlReferrer::Context`.
5. Invoke the shared resolver with purpose `CemQl` and return exactly one
   `AtomValue::AnyUri` containing the serialized absolute URL.

The second argument is a module referrer, not the plain base argument of a URL join.
It affects relative target normalization and import-map scope selection. A query that
only needs RFC/WHATWG URL joining uses the separate URL operation defined for that
purpose.

The CEM-QL compiler always knows the function and its signature. Availability of a
runtime resolver is a capability check, not name resolution. A missing capability or
failed resolution emits a call-site diagnostic and fails that expression; it does not
return an empty sequence, `null`, or the authored specifier. The initial diagnostic
mapping is:

| Shared failure | CEM-QL diagnostic |
| --- | --- |
| invalid argument/specifier | `cem.ql.module_url_invalid` |
| unmapped bare specifier | `cem.ql.module_url_unresolved` |
| blocked mapping | `cem.ql.module_url_blocked` |
| resource-policy denial | `cem.ql.module_url_policy_denied` |
| no active resolution capability/context | `cem.ql.module_url_unavailable` |
| invalid or unresolved scalar referrer | `cem.ql.module_url_referrer_invalid` / `cem.ql.module_url_referrer_unresolved` |
| node has no live resolution scope | `cem.ql.module_url_referrer_unavailable` |
| node scope is not current-or-descendant | `cem.ql.module_url_referrer_scope_denied` |

The result URI is the query value. Match provenance, resolver identity, and policy
stamps remain in the query report/trace rather than changing the value into a record.

`EvaluationContext` and `StandaloneExpressionContext` therefore gain an optional
runtime module-resolution capability consisting of the shared resolver plus an opaque
context handle. It is not serialized into a compiled query artifact. Artifact identity
records that `module_url` is used and stamps the required resolver capability/profile;
reloading does not capture a live resolver object.

### 8.3 XPath `cem-ql:` extension function

CEM-owned XPath hosts expose the equivalent expanded-name function:

```xpath
cem-ql:module-url("@example/assets/logo.svg")
cem-ql:module-url("./asset.css", $referrer)
```

The function contracts are:

```text
Q{https://cem.dev/ns/query/cem-ql/1}module-url(
  $specifier as xs:string
) as xs:anyURI

Q{https://cem.dev/ns/query/cem-ql/1}module-url(
  $specifier as xs:string,
  $referrer as item()
) as xs:anyURI
```

`xs:anyURI` arguments are accepted through XPath function conversion to string. The
prefix `cem-ql` is a conventional built-in binding for
`https://cem.dev/ns/query/cem-ql/1` in CEM-owned XPath host attachments. Expanded QName
identity is authoritative; another prefix bound to the same namespace invokes the same
function. A foreign namespace with local name `module-url` MUST remain unsupported.

The XPath extension is intentionally not `fn:resolve-uri()`. The standard function
joins a relative reference to a static or explicit base; it neither applies the CEM
module-map stack nor enforces CEM's outer-context precedence. The CEM extension's
optional second argument is a module referrer with the §5 semantics.

Evaluation rules mirror CEM-QL:

1. Evaluate the specifier and apply the declared one-item string conversion.
2. Obtain the current resolution handle from the scope-bearing dynamic context node,
   falling back to the host's owning handle when focus is absent, atomic, or unscoped.
3. If present, evaluate the referrer as exactly one item. Accept `xs:string`,
   `xs:anyURI`, `xs:untypedAtomic`, or a scope-bearing node; reject every other value.
4. Read the resolver and node-context capabilities from `XPathEvaluationRequest` and
   invoke the shared resolver with purpose `XPath`.
5. Return one `XPathAtomicValue` with type name `xs:anyURI` and the resolved URL as its
   lexical value.
6. Preserve the call expression's source map on the result and on any diagnostic.

`XPathEvaluationRequest` therefore gains an optional module-resolution capability.
`CemQlXPathInvocationAdapter`, CEMT, XSLT, standalone transform, native, and WASM paths
forward the handle supplied by their owning CEM context. The XPath static checker
recognizes the expanded QName and arity independently from runtime capability
availability.

Failure is a dynamic XPath error with no result item:

| Shared failure | XPath diagnostic |
| --- | --- |
| invalid argument/specifier | `cem.xpath.module_url_invalid` |
| unmapped bare specifier | `cem.xpath.module_url_unresolved` |
| blocked mapping | `cem.xpath.module_url_blocked` |
| resource-policy denial | `cem.xpath.module_url_policy_denied` |
| no active resolution capability/context | `cem.xpath.module_url_unavailable` |
| invalid or unresolved scalar referrer | `cem.xpath.module_url_referrer_invalid` / `cem.xpath.module_url_referrer_unresolved` |
| node has no live resolution scope | `cem.xpath.module_url_referrer_unavailable` |
| node scope is not current-or-descendant | `cem.xpath.module_url_referrer_scope_denied` |

### 8.4 Surface equivalence

For one active resolution-context handle and one authored specifier, these expressions
must resolve to the same URL or equivalent failure reason:

```cem
{cem-module-url @slice=asset @src="@example/asset"}
```

```cemql
module_url("@example/asset")
```

```xpath
cem-ql:module-url("@example/asset")
```

The adapters differ only in result projection:

- the template instruction stores the URL in a named slice and records resource state;
- CEM-QL returns one `anyURI` item;
- XPath returns one `xs:anyURI` item.

Errors keep surface-appropriate diagnostic prefixes but retain the same shared reason,
matched frame/key, resolver identity, policy stamp, and source-map provenance. No
adapter retries through `import.meta.resolve()`, a package registry, `fn:resolve-uri()`,
or another fallback.

## 9. Host API

The current browser callback shape
`resolveModuleUrl(specifier, baseDocument, resourceBaseUrl)` is not sufficient: it
passes two competing base concepts, exposes a browser `Document` to a supposedly
portable operation, and does not identify the active context stack.

The shared scope-aware boundary is conceptually:

```ts
interface CemModuleUrlResolutionRequest {
  purpose: 'template-slice' | 'cem-ql' | 'xpath';
  authoredSpecifier: string;
  currentContext: CemModuleUrlResolutionContext;
  referrer?:
    | { kind: 'url'; value: string }
    | { kind: 'context'; context: CemModuleUrlResolutionContext };
}

interface CemModuleUrlResolutionContext {
  readonly identity: string;
  readonly baseUrl: string;
  readonly resolverIdentity: string;
  readonly resourcePolicyStamp: string;
}

interface CemModuleUrlResolution {
  authoredSpecifier: string;
  normalizedSpecifier: string;
  resolvedUrl: string;
  contextIdentity: string;
  resolverIdentity: string;
  resourcePolicyStamp: string;
  referrerKind?: 'url' | 'context';
  authoredReferrer?: string;
  resolvedReferrerUrl?: string;
  selectedContextIdentity: string;
  matchedFrameId?: string;
  matchedScopePrefix?: string;
  matchedCollection?: 'imports' | 'resources';
  matchedKey?: string;
  contentTypeHint?: string;
  integrity?: string;
}

interface CemModuleUrlResolutionFailure {
  authoredSpecifier: string;
  normalizedSpecifier?: string;
  contextIdentity: string;
  resolverIdentity: string;
  resourcePolicyStamp: string;
  reason:
    | 'invalid'
    | 'unresolved'
    | 'blocked'
    | 'policy-denied'
    | 'unavailable'
    | 'referrer-invalid'
    | 'referrer-unresolved'
    | 'referrer-unavailable'
    | 'referrer-scope-denied';
  matchedFrameId?: string;
  matchedKey?: string;
}
```

The browser projection exposes immutable context metadata but not the handle or frame
array. The runtime translates that object to its opaque Rust handle before calling the
WASM boundary. Hosts supply root map/base inputs and map-resource loading capabilities;
they do not receive or reconstruct a mutable array of lexical frames for each request.

`CemElementRuntimeOptions.moduleUrlRoot` accepts a frozen base and explicit import-map
input. An explicit map replaces automatic page `<script type="importmap">` capture.
`resolveScopedModuleUrl(request)` is the host override and may return an absolute URL or
the structured resolution record above. The old
`resolveModuleUrl(specifier, Document, resourceBaseUrl, referrer?)` hook is retained only
as a compatibility adapter. When neither callback is supplied, `cem-elements` sends the
captured root plus per-instance template contexts to the Rust resolver through the
`resolveModuleUrl` WASM export; it does not call `import.meta.resolve()`.

Resolution cache keys include at least:

```text
authored specifier
+ active base URL
+ referrer kind and resolved referrer URL
+ current and selected resolution-context identities
+ ordered selected resolution-context frame identities
+ normalized module-map hashes
+ resolver identity
+ resource-policy stamp
```

Caching only `resourceBaseUrl + specifier`, as the current browser runtime does, is
incorrect when two lexical contexts share a base but have different map stacks.

## 10. Resource lifecycle and diagnostics

`cem-module-url` participates in the same scheduled/loaded/failed resource lifecycle as
other template resource controls, even though successful resolution does not fetch the
target.

Required diagnostic categories are:

| Code | Condition |
| --- | --- |
| `cem.module_url.slice_required` | `slice` is missing or empty |
| `cem.module_url.src_required` | interpolated `src` is missing or empty |
| `cem.module_map.prelude_order` | a context map occurs after ordinary content |
| `cem.module_map.prelude_duplicate` | more than one map prelude occurs in a context |
| `cem.module_map.entry_duplicate` | a key is duplicated within or across imports/resources |
| `cem.module_map.entry_invalid` | key, target, prefix, metadata, or JSON shape is invalid |
| `cem.module_url.unresolved` | every frame misses a bare specifier |
| `cem.module_url.blocked` | a matching entry blocks resolution or cannot produce a contained prefix result |
| `cem.module_url.policy_denied` | the final URL violates effective resource policy |
| `cem.module_url.referrer_invalid` | a referrer has invalid type, cardinality, spelling, or URL form |
| `cem.module_url.referrer_unresolved` | a relative or bare scalar referrer cannot be resolved in the current context |
| `cem.module_url.referrer_unavailable` | a node has no live registered resolution-context handle |
| `cem.module_url.referrer_scope_denied` | a node selects an ancestor, sibling, or foreign context |
| `cem.module_url.context_mismatch` | serialized/server and active resolution-context identities disagree |

Failures retain authored input, source map, context identity, matched frame/key when
available, and policy/resolver stamps. They do not publish the original `src`, `null`,
or a partially resolved target into the slice.

## 11. Verification contract

Implementation follows the native tests-first flow: test the pure resolver and context
stack in Rust before browser/WASM integration.

### 11.1 Resolver unit cases

- scheme URLs including forms without `//`, protocol-relative, root-relative, `./`,
  `../`, query-only, and fragment-only inputs;
- active inner base normalization and map-source-relative target normalization;
- exact match before longest prefix inside a collection;
- most-specific applicable URL scope before less-specific scopes and unscoped entries;
- `imports` before `resources` in one specifier map;
- outer prefix match before inner exact match;
- inner fallback only after every outer candidate map misses;
- prefix traversal/backtracking rejection;
- invalid/null outer match preventing inner fallback;
- unresolved bare specifier producing no value;
- allowed-scheme and parent-policy denial after mapping;
- deterministic context and resolver identity hashes.
- absolute scalar referrer used as-is while selecting scoped mappings for the target;
- relative and bare scalar referrers resolved once through the current frame stack;
- scalar referrer retaining the current map frames rather than acquiring child maps;
- current and descendant node contexts accepted with their full frame stacks;
- ancestor, sibling, foreign-tree, destroyed, and unscoped node contexts rejected;
- intermediate referrer and final target policy denials distinguished in provenance;

### 11.2 Parser and schema cases

- valid CEM-ML prelude with exact imports, prefix imports, typed resources, and scopes;
- prelude after content, duplicate prelude, duplicate key, invalid target, and mismatched
  prefix slash diagnostics;
- equivalent CEM-ML and runtime-profile JSON maps normalizing to the same map identity;
- existing module-map v1-v3 fixtures retaining their accepted/rejected deployment
  behavior while contributing only their valid runtime-resolution subset.

### 11.3 Browser and SSR integration cases

- inline page declaration uses frozen `document.baseURI`, including `<base href>`;
- external and redirected template resolves relatives from the final template URL;
- page import map overrides a conflicting template-local exact or prefix entry;
- a template-local map fills a page-map miss;
- normalized absolute/relative URL keys can remap URL-like inputs;
- page `scopes` select mappings using the owning template URL as referrer;
- v2/v3 typed resource resolution preserves content type and integrity metadata;
- maps added after root creation are ignored;
- failed re-resolution removes a stale slice value and retains failed metadata;
- browser and SSR produce identical URLs and identities from equivalent root inputs;
- hydration detects a root base/map/policy identity mismatch rather than reusing a stale
  server value;
- transient `cem-module-url` controls never remain in visible light DOM.

### 11.4 CEM-QL and XPath integration cases

- CEM-QL type checking recognizes `module_url(string) -> anyURI` and
  `module_url(string, string | anyURI | node) -> anyURI`, rejecting other arities,
  types, and cardinalities;
- a scope-bearing CEM-QL context node supplies the current context while atomic or
  absent focus falls back to the owning evaluation context;
- CEM-QL returns `AtomValue::AnyUri` through native and WASM evaluation with identical
  resolution trace identity;
- compiling succeeds without a live resolver, while evaluation fails with
  `cem.ql.module_url_unavailable` when the capability is absent;
- XPath static resolution recognizes
  `Q{https://cem.dev/ns/query/cem-ql/1}module-url#1` and `#2`, including the conventional
  `cem-ql` prefix and an alternate prefix bound to the same namespace;
- XPath `#2` accepts one URI-like atomic or node referrer and preserves typed node
  identity until the host resolves its context handle;
- a same-local-name function in a foreign namespace remains unsupported;
- XPath returns one `xs:anyURI` item with call-site source-map provenance;
- embedded CEM-QL XPath, CEMT, XSLT, standalone/native, and WASM invocation adapters
  forward the owning resolution-context handle rather than constructing a new root;
- invalid, unresolved, blocked, denied, and unavailable failures map to the specified
  surface diagnostic while retaining the shared resolver reason and match provenance;
- the template, CEM-QL, and XPath forms produce byte-identical serialized URLs for
  inline-page, imported-template, outer-override, inner-fallback, prefix, scoped-map,
  and typed-resource cases;
- neither query surface fetches the resolved target or consumes an external-fetch
  budget.

## 12. Implementation status and remaining gaps

Implemented now:

- `cem_ml::module_resolution` owns the immutable parent-linked context tree, outer-first
  map walk, exact/prefix/scoped matching, active-base handling, policy checks, referrer
  preprocessing, descendant authorization, provenance, and surface-neutral errors;
- CEM-QL registers and evaluates both `module_url#1` and `module_url#2`, preserves node
  identity for the second argument, derives current context from query focus, and
  returns `anyURI`;
- native XPath recognizes both expanded-name arities, propagates resolution context on
  native nodes, accepts scalar or node referrers, and returns `xs:anyURI`;
- the common WASM module exposes the same Rust resolver as a JSON context boundary;
- `cem-elements` freezes the page base/import maps per Document, creates parent-linked
  per-instance template contexts, installs compile-time local maps before custom-element
  upgrade/render, supports an explicit root override and typed host callback, lowers
  canonical `cem-module-url` controls out of DOM/worker plans, evaluates clone-safe
  descendant `referrer-selector` controls after patch commit, writes only successful
  absolute URLs, and removes stale values on failure;
- the CEM-QL compiler extracts one static `module-map` prelude into the portable
  component-template artifact, including `imports`, typed `resources`, and `scopes`;
- the canonical schema recognizes `module-map`, its static entries, and
  `cem-module-url@slice`, `@src`, optional scalar `@referrer`, and optional
  `@referrer-selector`. The old `module-url` spelling remains a browser/legacy
  compatibility input, not a second canonical instruction.

Remaining integration work:

- define and ship the standalone runtime-resolution module-map schema/profile and its
  external JSON/content-handoff loaders; the inline CEM-ML prelude is implemented;
- preserve a node-valued whole attribute expression through template rendering so
  `cem-module-url@referrer` can select a typed AST context without the selector bridge;
  browser processing controls intentionally remain clone-safe;
- convert CLI/SSR `rootScope.baseUri` and loaded `rootScope.moduleMap` resources into the
  shared context capability and forward it automatically through query, CEMT, XSLT,
  standalone-transform, and SSR template hosts;
- stamp compiled-query/template artifacts with resolver profile requirements and include
  the effective resolution-context identity in hydration mismatch checks;
- add redirect/final-response URL plumbing, browser/SSR identity parity, and complete
  typed-resource metadata enforcement.

These gaps are staged integration work; they are not alternate accepted resolution
behavior.
