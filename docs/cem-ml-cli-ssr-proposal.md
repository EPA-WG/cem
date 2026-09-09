# CEM-ML CLI Whole-Page SSR Proposal

**Status:** Proposal; the command and page compiler described here are not yet
implemented.

**Applies to:** `cem-ml`, CEMT transform graphs, `cem-elements`, Edge/SSR
hosts, browser hydration, HTML/CEM-ML/Markdown page sources, and repository demo
compilation.

**Related contracts:**

- [`cem-element-lifecycle-principle.md`](./cem-element-lifecycle-principle.md)
- [`cem-element-wasm-proposal.md`](./cem-element-wasm-proposal.md)
- [`cem-elements-edge-ssr-gate.md`](./cem-elements-edge-ssr-gate.md)
- [`../packages/cem-demo-element/README.md`](../packages/cem-demo-element/README.md)
- [`cem-ml-cli-contract.md`](./cem-ml-cli-contract.md)
- [`cem-ml-phase2-run-config-contract.md`](./cem-ml-phase2-run-config-contract.md)
- [`cem-ml-uid-and-scoped-css-design.md`](./cem-ml-uid-and-scoped-css-design.md)

## 1. Proposal summary

Whole-page SSR should be a CEM-ML transform operation. A configured CEMT
template receives a typed page AST and an absolute deployed page URL, invokes a
shared CEM lifecycle intrinsic, and returns an SSR page artifact. That artifact
contains the mutated page AST, canonical data islands, rendered light-DOM
ranges, render identities, diagnostics, and source maps. A later converter and
writer serialize the artifact to deployable HTML.

CEMT is the right declarative orchestration and projection boundary, but it
must not reimplement the `cem-element` lifecycle. Payload capture, declaration
resolution, module URL resolution, data-island transactions, template
compilation, render-plan generation, nested-instance settling, scoped CSS, and
hydration identity must come from the same native core used by browser/WASM and
Edge/SSR processing.

Package-owned custom elements whose behavior is itself a CEM-ML transformation
may publish a tag-specific CEMT SSR participant. `cem-demo-element` is the
immediate example: its JavaScript coordinates source loading and browser DOM,
while its payload-to-source-view/live-demo transformation can be expressed as
CEMT and executed against the same typed ASTs on the server.

The recommended initial public invocation is config-first:

```bash
./dist/target/debug/cem-ml transform --config ssr-pages.cem
```

A future `cem-ml ssr` command may be shorthand that lowers into the same
normalized transform graph. It must not introduce a second SSR engine or
configuration model.

The server **renders** and serializes hydration state. The browser
**hydrates** by admitting the serialized island and adopting an identity-matched
rendered range without an initial rerender.

```text
HTML / CEM-ML page / .cem.md
              |
              v
     namespace-aware DOM AST
              |
              v
 CEMT page + component participants
              |
              v
 shared DCE lifecycle -> islands + render plans -> updated DOM AST
              |
              v
       SSR page artifact
         |           |
         v           v
 deployable HTML   report/source-map/colored source-view sidecars
              |
              v
 browser admission and identity-matched adoption
```

## 2. Goals

The first implementation should:

1. accept an HTML page, a CEM-ML page, or an opted-in `.cem.md` page;
2. normalize every input into one namespace-aware page DOM AST;
3. run the same declaration, instance, data-island, render-plan, URL-resolution,
   and scoped-CSS semantics as the browser runtime;
4. write the canonical island before each instance's derived rendered range;
5. recursively render nested DCE instances until the page reaches a bounded,
   deterministic settled state;
6. serialize functional HTML that the browser can hydrate without rerendering;
7. support one file or directory/glob fan-out with deterministic destinations;
8. keep the deployed page URL independent from the source and output paths;
9. retain source maps and all resolver, policy, artifact, and revision
   identities needed to validate adoption; and
10. optionally project formatted or colorized source views without modifying
    the deployable page.

The first implementation does not:

- execute arbitrary page JavaScript or emulate a browser;
- SSR arbitrary JavaScript-defined custom elements without a registered,
  deterministic CEMT/native participant;
- infer network, storage, time, random, layout, image-load, or user-event
  results;
- make rendered HTML an alternative source of component state;
- use the output filesystem path as a URL base; or
- accept a colorized source-view document as hydratable application HTML.

## 3. Three identities must remain separate

SSR needs three primary identities and must never silently substitute one for
another.

| Identity | Example | Purpose |
| --- | --- | --- |
| source URI | `file:///work/cem/packages/cem-elements/demo/hex-grid.html` | Reads bytes, resolves source-owned build references, and anchors diagnostics/source maps. |
| page URL | `https://docs.example.test/examples/hex-grid/` | Models the browser document URL, seeds `document.baseURI`, resolves page-relative runtime URLs, and participates in hydration/cache identity. |
| output destination | `/work/cem/dist/examples/hex-grid/index.html` | Receives bytes atomically; it has no URL-resolution authority. |

The page URL may match the deployed filename:

```text
page URL: https://docs.example.test/demo/hex-grid.html
output:   dist/demo/hex-grid.html
```

It may also deliberately differ:

```text
page URL: https://docs.example.test/guide/
output:   dist/guide/index.html
```

This distinction is required before lifecycle execution because a relative URL
inside an inline declaration must already resolve against the public page
context while the render plan is being built.

`pageUrl` must be an absolute URL with an allowed scheme and no fragment. Its
query is retained. A trailing slash remains significant: `.../guide/` and
`.../guide` have different relative-URL behavior and must not be normalized
into one another.

### 3.1 Derived URL identities

The page compiler derives and records additional identities:

- `documentBaseUrl`: the page URL after applying the first valid HTML
  `<base href>` under normal browser rules;
- `declarationUrl`: the resolved URL of an external declaration resource;
- `templateUrl`: the source URL of the effective declaration template;
- `moduleMapUrl`: the URL against which a module map's relative targets resolve;
- `resourceUrl`: the resolved identity used by HTTP, template, image, CSS, and
  other resource state; and
- `outputUri`: the normalized writable URI for the output destination.

Inline page DCE declarations inherit `documentBaseUrl`. An external declaration
or template uses its own resolved resource URL. Nested scopes push their own
base URL and module map. `cem-module-url`, CEM-QL URL functions, template loads,
and resource elements consume the same scope stack.

The first `<base>` element changes browser URL resolution, not the page's
canonical identity. Therefore both `pageUrl` and `documentBaseUrl` belong in the
SSR artifact and its cache/adoption identity.

### 3.2 Public URL to local-byte mapping

A page may resolve `./card.cem` to
`https://docs.example.test/guide/card.cem` while the build reads
`/work/cem/src/guide/card.cem`. Resolver read maps provide that explicit bridge:

```text
https://docs.example.test/=/work/cem/src/
```

The resolved public URL remains the declaration/resource identity. The local
path is only the authorized byte provider. This preserves browser and SSR URL
behavior without requiring the output tree to exist before rendering.

## 4. Source forms

File extensions are convenience hints. The normalized input spec's content type
and schema remain authoritative.

All three source forms converge on the existing binary-first DOM projection:

```text
content type: application/vnd.cem.dom+cem-bin
schema:       https://cem.dev/ns/projection/dom/1
```

The compiler retains the native AST/evaluator view across this handoff. It does
not serialize an intermediate JSON tree or generated HTML and parse it again.

### 4.1 HTML

HTML input uses `text/html` and the HTML document schema. The HTML parser
produces the source page AST directly. It retains inert templates, HTML recovery
semantics, namespaces, raw-text/RCData boundaries, import maps, `<base>`, and
source ranges.

The SSR compiler preserves page scripts as page content but does not execute
them. In particular, the client module which installs `cem-elements` remains in
the result for hydration.

### 4.2 CEM-ML

A `.cem` page should use a dedicated page-document schema, proposed as:

```text
content type: application/vnd.cem.page+cem
schema:       https://cem.dev/ns/page/1
```

The schema describes a document whose output nodes have explicit HTML, SVG, or
MathML namespace identity and may contain `cem-element` declarations and DCE
instances. A registered converter lowers it directly into the same DOM
projection used by HTML input.

A CEM-native template module is a different source kind. If a `.cem` file is a
template module, an earlier transform stage must render its selected entrypoint
to the page-document AST before the SSR stage. Generic `application/cem`
without a page or template schema is too ambiguous and must diagnose.

### 4.3 `.cem.md`

`.cem.md` should be an explicit CEM-enabled Markdown profile, not an extension
that silently changes generic Markdown behavior. The proposed identity is:

```text
content type: text/markdown; charset=utf-8; variant=CEM
schema:       https://cem.dev/ns/page/cem-markdown/1
```

Normal Markdown blocks first lower through the existing typed Markdown AST.
The profile may additionally admit explicitly labelled CEM-ML page fragments.
Those fragments are parsed as CEM-ML and joined into the page DOM AST with
source-map frames for both the Markdown fence and embedded source.

Unlabelled fences remain code. Raw embedded HTML remains subject to the existing
trust policy. Ordinary `.md` with CommonMark or GFM identity does not execute
CEM-ML. The compound suffix is only the default identity hint for local CLI
use; config should still state the content type/schema in reproducible builds.

Front matter is not required for the first slice. Page URL and deployment route
belong in transform configuration so the same source can be published at more
than one route without rewriting source bytes.

## 5. Normalized SSR artifact

The lifecycle stage should produce a typed binary-first artifact rather than an
HTML string:

```text
content type: application/vnd.cem.ssr-page+cem-bin
schema:       https://cem.dev/ns/projection/ssr-page/1
```

Conceptually:

```text
CemSsrPageArtifact
├── route
│   ├── sourceUri
│   ├── pageUrl
│   ├── documentBaseUrl
│   └── outputUri?
├── documentAst
├── declarations[]
├── instances[]
│   ├── canonical data-island AST
│   ├── compiled template identity
│   ├── render-plan identity and rendered range
│   ├── data/resource revision
│   └── hydration/adoption identity
├── managedStyles[]
├── sourceMaps[]
├── diagnostics[]
└── executionIdentity
```

The artifact is the handoff between lifecycle rendering and output
presentation. HTML serialization, an inspect view, a CEM report, and a source
view are projections of it. No projection becomes a competing lifecycle
authority.

`documentAst` should reuse the existing CEM DOM projection and its CEMT-primary
HTML converter. The SSR envelope adds route, lifecycle, adoption, and execution
identity; it does not define a competing DOM representation. The proposed
`ssr-page-to-html-cemt` converter selects that contained DOM projection and
delegates serialization to `cem-dom-projection-to-html-cemt`.

`outputUri` is useful provenance but is excluded from the render-plan identity
unless the author explicitly derives `pageUrl` from it. Moving identical output
bytes to another local directory must not change component behavior.

## 6. Shared lifecycle execution

The common native core should expose a page operation similar to:

```text
renderPageInitial(
    pageAst,
    pageUrl,
    rootScope,
    resolverSnapshot,
    capabilitySnapshot,
    sourceMapMode,
    settlePolicy
) -> CemSsrPageArtifact
```

The browser/WASM runtime and native CLI use the same functions for the steps
below. Only their final DOM appliers differ.

### 6.1 Admission and discovery

1. Validate the page AST and route identities.
2. Compute `documentBaseUrl` and parse page import maps as data.
3. Combine the page map with the configured global/root module map under the
   existing winner rules. The global policy may override page mappings; nested
   declaration scopes then push local maps.
4. Discover all static `cem-element` declarations in active page content.
5. Discover registered non-DCE SSR participants by exact tag and adapter
   identity.
6. Resolve and compile external declarations and participant templates through the same scoped resolver
   used by the browser.
7. Reject duplicate, incompatible, unresolved, or policy-denied declarations
   before mutating the output AST.

An inert template remains inert unless its owning, known component lifecycle
or registered SSR participant explicitly consumes it. The compiler must not
search arbitrary template contents for declarations or instances.

### 6.2 Instance initialization

For every active produced DCE instance without a marked island:

1. capture author payload using the normative first-connection rules;
2. create the direct `template[data-cem-island="instance"]`;
3. create exactly one namespace-aware `cem-island:context-root` at the current
   lifecycle version;
4. initialize hydration, attributes, dataset, payload, slices, resources, form,
   validation, and event sections in canonical order;
5. derive a sanitized processing snapshot from that island;
6. generate the render plan from the effective compiled declaration; and
7. apply the plan to the page AST outside the island.

The AST applier must implement the same ownership and namespace rules as the
browser DOM applier. It must preserve nested initialized DCE bodies, render-node
identities, source-map fidelity, raw-text boundaries, void-element rules, form
control semantics, and declaration-owned managed styles.

### 6.3 Bounded settling

Rendered output may introduce nested DCE instances or declarative resources.
The server therefore processes deterministic lifecycle work to a bounded fixed
point:

```text
discover -> initialize -> render -> apply to AST -> discover nested work
```

The settle policy records maximum declaration depth, instance count, render
passes, resource count, total bytes, and wall/CPU budgets. Repeating the same
unresolved work without a state revision is a cycle and fails. Partial or
budget-exhausted pages are not committed unless an explicit static-fallback
policy permits them and marks them non-adoptable.

### 6.4 Host capabilities

SSR has no implicit browser globals. Capabilities are explicit:

| Capability | SSR rule |
| --- | --- |
| URL/location | Read-only values derive from `pageUrl`; history and navigation do not run. |
| HTTP | Disabled by default; enabled only through a bounded resolver/cache policy whose response identity enters the artifact hash. |
| local/session storage | Requires a configured snapshot/store adapter; otherwise resource state remains unavailable or deferred according to declaration policy. |
| timers, randomness, current time | Forbidden unless supplied as deterministic input. |
| DOM events | Bindings and semantic event state serialize; listeners attach only during browser hydration. |
| image/font/media load | The server does not infer load success from a URL. Initial loading/fallback state remains until the browser or an explicit media metadata provider advances it. |
| layout/computed style | Unavailable. Rendering must not depend on layout measurements. |
| arbitrary scripts/custom elements | Scripts are preserved but not executed. A custom element runs only through an exact registered CEMT/native SSR participant. |

This matters for the hex-grid image links: the server can resolve `href` and
`src`, render the image and fallback nodes, and serialize the initial
`imageState`. It cannot claim that the image loaded. The browser's real `load`
or `error` event advances that state after hydration.

## 7. CEMT boundary

### 7.1 What CEMT should do

The SSR CEMT module should:

- declare the public `render-page` entrypoint and typed parameters;
- receive the page AST as `$input`;
- provide page URL, source-map mode, settle policy, and named capability/resource
  snapshots to the lifecycle intrinsic;
- return the typed SSR page artifact;
- allow an application-owned wrapper module to add build-time page structure
  before or after the lifecycle stage; and
- let the transform graph branch to HTML, inspection, report, source-map, and
  source-view exports.

Illustrative CEMT shape:

```cem
@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@ns ssr = "https://cem.dev/ns/ssr/cem/1"
@default transform

{module @version="1.0.0" |
  {param @name=input @type=object @required=true}
  {template @name=render-page @visibility=public |
    {param @name=pageUrl @type=string @required=true}
    {param @name=sourceMapMode @type=string @default=prod}
    {param @name=settlePolicy @type=string @default=static}
    {body |
      {$ ssr:render-page($input, {
        pageUrl: $pageUrl,
        sourceMapMode: $sourceMapMode,
        settlePolicy: $settlePolicy
      }) }
    }
  }
}
```

`ssr:render-page` above is a proposed typed intrinsic. It is not an existing
CEM-QL function. The transform adapter must admit it only in the SSR page
runtime phase and supply immutable resolver/capability inputs. General CEMT
evaluation remains pure and cannot gain ambient filesystem, network, DOM, or
clock access.

### 7.2 What CEMT should not do

A CEMT template must not manually:

- search element names and imitate custom-element upgrade order;
- construct island XML by string concatenation;
- duplicate CEM-QL render, module-map, scoped CSS, or resource-state rules;
- serialize and reparse JSON between lifecycle stages;
- execute page module scripts; or
- decide that output paths are base URLs.

Those approaches would create a second lifecycle whose output could resemble
the browser in simple fixtures while diverging under reconnection, nested DCEs,
resources, scope overrides, and failure recovery.

### 7.3 Component SSR participants

A component implemented with browser JavaScript may still be SSR-capable when
its visible behavior is a deterministic transformation over declared payload
and resources. It publishes a participant descriptor containing:

- exact custom-element tag;
- participant contract and implementation version;
- CEMT module URI, content type, schema, and entrypoint;
- accepted payload/slot/resource identities;
- output fragment identity;
- permitted deterministic capabilities;
- hydration/adoption identity; and
- a browser implementation compatibility range.

The registry selects participants by exact declared identity. It never guesses
from a tag name, imports browser JavaScript, or scans a class implementation.
Participant CEMT receives the admitted payload AST, named slot projections,
resolved resources, page/scope context, and formatter/colorizer services. It
returns an ordinary page-fragment AST plus participant state. The page compiler
inserts that fragment, records its adoption identity, and continues the normal
settling loop. Any active `cem-element` declarations and produced DCE instances
introduced by the fragment are then handled by the generic shared lifecycle.

A participant which needs client hydration must serialize its state through the
same DOM-native island topology and context-root version as a DCE; it must not
invent a sibling JSON script or recover state from rendered output. Generalizing
the normative island owner from “produced DCE instance” to “registered CEM
transformation host” requires an explicit lifecycle-contract amendment before
the participant ships. The host still owns exactly one island, placed before
its participant-rendered range.

This is appropriate for `cem-demo-element`: its CEMT participant can select the
`source`, `demo`, `text`, `legend`, `description`, and `status` regions, load an
explicit external source through the resolver, dispatch formatting/coloring by
AST content type, emit the visible code and description structure, and
materialize the live demo fragment. Inline source keeps the page scope; loaded
source establishes its resolved source URL as the fragment's resource base.
The inner CEM declarations and instances are not interpreted by the participant;
after materialization they are discovered and rendered by the normal DCE
lifecycle.

Browser `cem-demo-element` must consume the same participant contract or prove
equivalent output and adoption identity. Its JavaScript may coordinate lazy
loading, viewport behavior, DOM attachment, and WASM calls, but it must not own
a divergent payload transformation. Hydration adopts identity-matched
participant output without rebuilding it.

## 8. Transform configuration

The examples in this section are proposed target configuration. They will not
run until the SSR page adapter, artifact identity, and any noted graph bindings
are implemented.

### 8.1 One HTML page

```cem
@doc cem-ml 1

{run |
  {import
      @id=page
      @src="packages/cem-elements/demo/hex-grid.html"
      @content-type="text/html"
      @schema="https://cem.dev/ns/data/html/1" |
    {convert
        @id=page-ast
        @content-type="application/vnd.cem.dom+cem-bin"
        @schema="https://cem.dev/ns/projection/dom/1"
        @converter="html-to-cem-dom-projection-rust" |
      {transform
          @id=ssr
          @src="builtin:cem-elements/ssr-page.cemt"
          @template-content-type="application/vnd.cem.transform+cem"
          @template-schema="https://cem.dev/ns/transform/cem/1"
          @entrypoint=render-page |
        {param
            @name=pageUrl
            @value="http://localhost:8080/packages/cem-elements/demo/hex-grid.html"}
        {param @name=sourceMapMode @value=dev}
        {param @name=settlePolicy @value=static}
        {convert
            @id=html
            @content-type="text/html"
            @schema="https://cem.dev/ns/data/html/1"
            @converter="ssr-page-to-html-cemt" |
          {export
              @out="packages/cem-elements/dist-ssr/demo/hex-grid.html"
              @content-type="text/html"
              @schema="https://cem.dev/ns/data/html/1"}
        }
      }
    }
  }
}
```

Command:

```bash
./dist/target/debug/cem-ml transform --config ssr-hex-grid.cem
```

The page URL controls runtime URL resolution. The export path controls only the
filesystem write.

### 8.2 Directory fan-out with matching routes

Existing graph imports and `{path}`/`{file}` bindings can express a directory
tree without making the renderer accept a directory as a document:

```cem
{run |
  {import @id=pages @src="packages/cem-elements/demo/*.html"
      @content-type="text/html"
      @schema="https://cem.dev/ns/data/html/1" |
    {convert @id=page-ast
        @content-type="application/vnd.cem.dom+cem-bin"
        @schema="https://cem.dev/ns/projection/dom/1"
        @converter="html-to-cem-dom-projection-rust" |
      {transform @id=ssr
          @src="builtin:cem-elements/ssr-page.cemt"
          @template-content-type="application/vnd.cem.transform+cem"
          @template-schema="https://cem.dev/ns/transform/cem/1"
          @entrypoint=render-page |
        {param
            @name=pageUrl
            @value="https://docs.example.test/demo/{file}"}
        {convert @id=html
            @content-type="text/html"
            @schema="https://cem.dev/ns/data/html/1"
            @converter="ssr-page-to-html-cemt" |
          {export @out="dist/demo/{file}"
              @content-type="text/html"
              @schema="https://cem.dev/ns/data/html/1"}
        }
      }
    }
  }
}
```

The CLI lowers the glob to a sorted set of exact source URI, page URL, and
output destination tuples before lifecycle work begins. Duplicate page URLs or
output destinations fail preflight. Outputs commit atomically only after all
required pages succeed.

### 8.3 `.cem.md` to clean route and `index.html`

For a source named `guide.cem.md`, the logical page name should be `guide`, not
`guide.cem`. Add a normalized `{page}` binding which strips one registered page
suffix (`.html`, `.cem`, or `.cem.md`) without applying arbitrary repeated
extension removal.

```cem
{run |
  {import @id=pages @src="content/*.cem.md"
      @content-type="text/markdown; charset=utf-8; variant=CEM"
      @schema="https://cem.dev/ns/page/cem-markdown/1" |
    {convert @id=html-source
        @content-type="text/html"
        @schema="https://cem.dev/ns/data/html/1"
        @converter="cem-markdown-to-html-rust" |
      {convert @id=page-ast
          @content-type="application/vnd.cem.dom+cem-bin"
          @schema="https://cem.dev/ns/projection/dom/1"
          @converter="html-to-cem-dom-projection-rust" |
        {transform @id=ssr
            @src="builtin:cem-elements/ssr-page.cemt"
            @template-content-type="application/vnd.cem.transform+cem"
            @template-schema="https://cem.dev/ns/transform/cem/1"
            @entrypoint=render-page |
          {param
              @name=pageUrl
              @value="https://docs.example.test/{page}/"}
          {convert @id=html
              @content-type="text/html"
              @schema="https://cem.dev/ns/data/html/1"
              @converter="ssr-page-to-html-cemt" |
            {export @out="dist/{page}/index.html"
                @content-type="text/html"
                @schema="https://cem.dev/ns/data/html/1"}
          }
        }
      }
    }
  }
}
```

Here `pageUrl` and `out` intentionally do not match as strings, yet relative
links resolve exactly as they would at the deployed clean URL.

### 8.4 CEM-ML page input

A CEM-ML page uses its explicit page schema and a proposed typed converter to
the shared DOM projection. It is not parsed as an HTML string:

```cem
{run |
  {import @id=page @src="pages/account.cem"
      @content-type="application/vnd.cem.page+cem"
      @schema="https://cem.dev/ns/page/1" |
    {convert @id=page-ast
        @content-type="application/vnd.cem.dom+cem-bin"
        @schema="https://cem.dev/ns/projection/dom/1"
        @converter="cem-page-to-dom-projection-cemt" |
      {transform @id=ssr
          @src="builtin:cem-elements/ssr-page.cemt"
          @template-content-type="application/vnd.cem.transform+cem"
          @template-schema="https://cem.dev/ns/transform/cem/1"
          @entrypoint=render-page |
        {param
            @name=pageUrl
            @value="https://app.example.test/account/"}
        {convert @id=html
            @content-type="text/html"
            @schema="https://cem.dev/ns/data/html/1"
            @converter="ssr-page-to-html-cemt" |
          {export @out="dist/account/index.html"
              @content-type="text/html"
              @schema="https://cem.dev/ns/data/html/1"}
        }
      }
    }
  }
}
```

If `account.cem` instead declares a CEM-native template module, its selected
page-producing entrypoint must run in a preceding, separately typed transform
stage. Schema identity—not the `.cem` extension—selects the meaning.

### 8.5 Direct-command shorthand

After the graph contract is stable, a convenience command may be added:

```bash
cem-ml ssr packages/cem-elements/demo/hex-grid.html \
  --page-url http://localhost:8080/packages/cem-elements/demo/hex-grid.html \
  --out packages/cem-elements/dist-ssr/demo/hex-grid.html
```

For multiple pages:

```bash
cem-ml ssr 'content/*.cem.md' \
  --content-type 'text/markdown; charset=utf-8; variant=CEM' \
  --page-url-pattern 'https://docs.example.test/{page}/' \
  --out-dir dist \
  --output-pattern '{page}/index.html'
```

`--out-dir` is a routing convenience. Normalization expands it with
`--output-pattern` into exact per-page export destinations. The engine never
writes an undifferentiated directory artifact. For a single input,
`--page-url` and `--out` are required unless config supplies them. For multiple
inputs, explicit patterns are required; no route is guessed from the current
working directory.

## 9. HTML serialization, formatting, and coloring

The primary page export is:

```text
SSR page AST -> HTML formatter -> uncolored HTML writer -> deployable bytes
```

The formatter must be DOM-safe. It may choose canonical quoting, attribute
order, doctype spelling, and line endings, but it must not insert text-node
whitespace where the page AST has none, modify raw-text/RCData data, reorder
meaningful nodes, or cross data-island/render-range boundaries. A `pretty`
profile may add whitespace only at AST locations explicitly marked layout-safe.

Coloring is a presentation projection:

```text
SSR page AST -> formatter -> semantic syntax token stream -> colorizer
             -> terminal preview or source-view HTML
```

An HTML colorizer adds markup representing source tokens. Applying it to the
deployable artifact would change the application DOM and violate hydration
parity. Therefore:

- deployable `.html` defaults to `colorProfile=none`;
- terminal color may be used when writing a preview to a terminal;
- an HTML-colored view must be a separately named sidecar such as
  `hex-grid.source.html`; and
- the colored view is never marked or accepted as a hydration artifact.

Nested source scopes in preserved declarations still use the shared AST syntax
stream: HTML yields to typed CEM-ML template content, CEM-ML yields to scoped
CSS, and each parent formatter/colorizer resumes at its AST-owned boundary.

## 10. Hydration and DOM parity

SSR output places the canonical marked data island before the instance's
rendered range. On client connection, `cem-elements`:

1. detects the direct marked island before payload discovery;
2. validates island schema/version and all namespace-aware domain parts;
3. validates page/base URL, resolver/module-map, declaration artifact, scope
   policy, source fidelity, data revision, render plan, and boundary identity;
4. mirrors serialized render-node identities into browser-owned properties;
5. adopts the existing rendered range without initial DOM mutation; and
6. attaches permitted event/resource capabilities and begins later invalidation.

Parity means the same normalized page AST and render-plan identities, not merely
similar HTML screenshots. Browser serialization may normalize quoting or
attribute order, so tests compare namespace-aware DOM structure, attributes,
text/comments, islands, range markers, managed styles, and render IDs. The
deployable server bytes must nevertheless remain deterministic for a fixed
input and policy identity.

If adoption identity fails, the lifecycle follows the existing fail-closed
contract. It must not reinterpret rendered siblings as payload. Where the
island itself remains valid, an explicitly permitted render-on-load fallback
may regenerate the rendered range from the island; otherwise the SSR result
remains frozen static content with diagnostics.

## 11. Concrete `hex-grid.html` path

`packages/cem-elements/demo/hex-grid.html` is not only a page containing active
page-level DCE instances. Its runnable sample source is placed inside inert
templates owned by `cem-demo-element`. Both `cem-element` and
`cem-demo-element` ultimately transform typed payload through CEM-ML
mechanisms, but they have different ownership boundaries.

The page compiler must not descend into an arbitrary inert template, discover
`cem-element` declarations there, and activate them. A browser would not do so
without the owner component's behavior, and the resulting payload capture and
URL context would be wrong.

Full SSR parity uses the package-owned `cem-demo-element` CEMT participant:

1. the page compiler admits the `cem-demo-element` host, moves its inert payload
   into the participant's canonical data island, and preserves its source URL
   context;
2. the participant applies the same slot/source selection and CEM-ML
   formatter/colorizer services as the browser component;
3. it emits the visible source view and activates the selected live-demo
   fragment in the page AST;
4. the generic settling loop discovers the now-active `cem-element`
   declarations and their produced hex-grid instances;
5. the shared DCE lifecycle creates their islands and rendered ranges; and
6. the browser adopts both participant and DCE output when their identities
   match.

If that participant is unavailable, the page may be explicitly emitted as a
partial, client-only artifact, but it cannot claim full browser DOM parity. The
CLI must not execute `cem-demo-element`'s browser JavaScript or add a
fixture-specific parser exception.

## 12. Cache and reproducibility identity

The SSR artifact cache key includes at least:

- every source byte hash and normalized source identity;
- page URL and derived document base URL;
- all declaration/template/module/resource content hashes;
- effective root, page, and nested module-map frames;
- resolver and resource-policy stamps;
- deterministic capability snapshots used during settling;
- lifecycle, island schema, render-plan IR, CEM-ML, and CEM-QL versions;
- template entrypoint and typed params;
- source-map mode and settle policy; and
- DOM-safe formatter profile for the final byte artifact.

The physical output destination, current working directory, wall clock, process
ID, thread scheduling, and machine-local cache location are excluded. A local
resolver target path is excluded when it is merely the byte provider for a
stable public URL; the bytes and resolver policy identity remain included.

## 13. Diagnostics

The proposal reserves these diagnostic families; final codes should live in
the owning schema packages:

| Condition | Proposed code |
| --- | --- |
| missing/relative/fragmented page URL | `cem.ssr.page_url_invalid` |
| duplicate normalized page URL | `cem.ssr.page_url_duplicate` |
| duplicate output destination | existing graph duplicate-destination diagnostic |
| ambiguous generic CEM-ML page input | `cem.ssr.page_schema_required` |
| unsupported `.cem.md` profile/fence | `cem.ssr.cem_markdown_unsupported` |
| declaration resolution or compatibility failure | existing `cem-element` declaration diagnostics |
| unsupported custom-element SSR participant | `cem.ssr.participant_unavailable` |
| browser-only capability required during initial render | `cem.ssr.capability_unavailable` |
| settle cycle/budget exhaustion | `cem.ssr.settle_cycle` / `cem.ssr.settle_budget` |
| formatter would change semantic DOM | `cem.ssr.formatter_not_dom_safe` |
| colorizer requested for deployable page | `cem.ssr.deploy_colorizer_forbidden` |
| hydration identity cannot be produced | `cem.ssr.hydration_identity_incomplete` |

Every diagnostic includes the source URI, page URL, output destination when
known, instance/declaration identity when applicable, active content type and
schema, scope stack, resolver/policy stamps, and a source-map frame.

## 14. Implementation sequence

### Phase A: route and artifact contracts

- Add `SsrPageRoute` with distinct source URI, page URL, document base URL, and
  output URI.
- Add normalized `{page}` binding and direct-command lowering rules.
- Add binary-first SSR page artifact and schema identities.
- Extend reports and cache identities before adding output writes.

### Phase B: shared lifecycle core

- Move declaration discovery, instance admission, island transaction, template
  compilation, render-plan generation, module URL resolution, and scoped style
  planning behind host-neutral native interfaces.
- Make browser/WASM and existing Edge/SSR hosts consume those interfaces.
- Keep browser-only DOM reconciliation and capabilities in the browser host.

### Phase C: DOM-free page applier

- Apply render plans and island transactions directly to the namespace-aware
  page AST.
- Add bounded nested-instance settling and deterministic resource snapshots.
- Replace the Node evidence serializer with a production HTML converter; do not
  promote fixture-only serialization as the canonical writer.

### Phase D: CEMT and transform graph

- Add the SSR runtime phase and typed `ssr:render-page` intrinsic.
- Ship the built-in public CEMT entrypoint through a schema package.
- Allow graph branching from the SSR artifact to HTML, report, inspect,
  source-map, and colored source-view outputs.
- Add the versioned component-participant registry and the first
  `cem-demo-element` CEMT participant.
- Add atomic multi-page output staging.

### Phase E: source adapters

- HTML first.
- Dedicated CEM-ML page-document schema second.
- Explicit `variant=CEM` Markdown adapter and `.cem.md` inference third.

### Phase F: hydration and repository gate

- Add native and Node artifact tests.
- Hydrate SSR output in Chromium and assert zero initial render mutations.
- Compare SSR-first and client-first normalized DOM and render identities.
- Run the same page/legend inventory and observable demo outcomes against both
  source-loaded and SSR-built pages.
- Prove the `cem-demo-element` CEMT participant and browser coordinator adopt
  the same source-view and live-demo fragment without an initial rebuild.

## 15. Acceptance matrix

The implementation is complete only when fixtures cover:

- HTML, CEM-ML page, and `.cem.md` ingress converging on the same page AST;
- one-file output and directory/glob fan-out;
- matching public/output paths and clean URL -> `index.html` mismatch;
- `<base href>`, relative URL, absolute URL, and bare module specifier cases;
- global/page import maps, nested module maps, and inner-scope override/winner
  behavior;
- inline and external declarations with their correct referrer/base URL;
- naked, wrapped, nested, and node-referrer `cem-module-url` cases;
- canonical island topology and ordering before rendered ranges;
- declaration and instance scoped CSS with deterministic managed-style output;
- nested DCE settling, CEMT participant output, cycle limits, and unsupported
  custom-element boundaries;
- deferred image loading, configured HTTP snapshots, and unavailable storage;
- valid hydration adoption with no initial DOM mutation;
- identity mismatch and fail-closed hydration behavior;
- compact and DOM-safe pretty HTML serialization;
- separate terminal/HTML-colored source projections which cannot be hydrated;
- stable bytes/cache keys across repeated runs and different output roots; and
- atomic failure without partially updated output directories.

## 16. Recommendation

Proceed with config-driven SSR as a new executable transform runtime phase,
using a built-in CEMT entrypoint over a shared native lifecycle intrinsic. Do
not implement SSR as a CEMT-only tree rewrite, a headless-browser wrapper, or a
special `hex-grid.html` loader.

The first vertical slice should use a small HTML page with one inline
declaration and one instance, prove island plus rendered-range hydration with no
client rerender, and only then add external resources, nested scopes, directory
fan-out, CEM-ML page input, `.cem.md`, and the `cem-demo-element` participant.
