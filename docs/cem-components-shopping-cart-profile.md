# Shopping-cart application deployment profile

Status: native reference fixture, strict artifact admission, shared compiled
registration, declarative static-closure loading and compiled-form recovery in
worker/fallback hosts, 2026-10-29. A complete shopping-cart browser app, GraphQL transport/validation
and reduced WASM remain pending.
This is the first concrete application case for the
[component WASM profile](cem-components-wasm-profile.md#application-build-and-deployment-boundary).

The app shows cart products, quantities, server totals and service errors. Its
build validates and compiles the authored UI and fixed GraphQL operations. Its
deployed client needs reactive rendering and JSON communication; unrelated
content formats, schemas and authoring tools can be loaded if requested later.

## Reference inputs and required behavior

The [fixture README](../packages/cem_ql/fixtures/component-application/shopping-cart/README.md)
links every input. The [reference manifest](../packages/cem_ql/fixtures/component-application/shopping-cart/profile.json)
declares one host binding, five shared components, JSON runtime data, optional
formats and fixed operation inputs. It is a test-consumed reference contract,
not the final production module manifest or an installed build target.

| Stage | Requirements |
| --- | --- |
| App build | CEM-ML/CEM-QL and declaration parsing, native template/expression compilation, style compilation/validation, GraphQL schema/operation and static-data validation, exact dependency/ABI/source hash records, generated variable/response guards. |
| Initial client | Compiled native render/evaluation, JSON decoding/encoding, required action/form behaviors, live DOM updates, app input checks, resources/cancellation and ownership/recovery guards. CSS parsing remains required by the current artifact wire format. |
| Optional demand | Additional data formats, dynamically admitted source templates, editor highlighting, Parse/Inspect, language converters and general schema tooling. |

Predefined `Cart`, `SetCartLineQuantity` and `RemoveCartLine` operations use the
same response shape through mutation aliases. Their JSON envelope carries data
and optional errors; a partial result remains visible alongside its error. The
protocol basis is [GraphQL over HTTP](https://graphql.org/learn/serving-over-http/).
The partial-error case nulls the nullable shipping-estimate field while retaining
the cart's non-null lines and totals.
The server returns integer minor-unit amounts and formatted totals. Stock,
prices, quantity limits and authorization are server decisions; generated client
checks support interaction without replacing those decisions.

## Native boundary proof

`template_artifact::application_profile_tests` compiles the cart at build time
and reloads its binary IR with the build's source identity and binding policy.
The reload/render path receives no original view source. Compiler profiling has
a positive build-stage control and asserts no template/query compilation during
runtime reload and repeated rendering. Both dev and prod source-map modes are
covered. JSON API documents are imported natively and retain their source owner;
they do not cross a JSON AST or JavaScript object reconstruction boundary.
The evaluator still assembles its general function registry during this path;
that runtime dependency is distinct from source compilation and must also be
partitioned before the minimal binary can exclude unused implementations.

The response sequence checks initial quantities, changed quantities and an added
line, line removal, empty cart, partial data/errors and failed requests. Product
text remains escaped. The fixture compares component references in compiled IR
with its declared roster, builds/reloads those canonical shared declarations,
and checks retained native CSS. Corrupt artifacts, different source hashes,
incompatible bindings/compiler versions and malformed JSON are refused.

Reproduce from the repository root with the Rust toolchain on `PATH`:

```bash
cargo test --locked --offline -p cem-ql --lib template_artifact::application_profile_tests -- --nocapture
cargo test --locked --offline -p cem-ql --test template_artifact_reload
```

The six app-boundary tests and nine existing artifact reload tests pass. This
native proof does not establish browser recovery, code omission or size savings;
those are delivery gates below. Full-runtime defaults remain in place. The
separate admission path below adds opt-in processing support and preserves
existing source/cache behavior.

## Strict source-free processing input

The native `importPinnedTemplateArtifact(bytes, contextJson)` endpoint and shared
`retainPinnedCemMlTemplateArtifact` adapter now admit binary IR without template
source. The independent context requires complete CEM content/source hashes,
both compiler versions, host bindings and source-map mode. Hashes use the existing
`cem-bin/1+blake3` identity, distinct from deployment-file SHA-256/SRI. Native
format, integrity, policy, memory/admission and build-error checks precede retaining
the template. A manifest describes requirements; it supplies no source grants or
native owners.

`CemProcessingCompileInput.source` accepts `pinned-template-artifact-v1` with the
existing binary transfer fields. The transfer's format/policy/source-map identity
must match the host request. Source/cache fallback, source-module overrides and
XSLT/CSS compilation options cannot be mixed into this strict input. The shared
processing protocol is v30. Shared compiled registration below uses this transport;
the declarative loader below now acquires build-generated component closures.

The host validates bounds before copying buffers and owns deployment inputs
across asynchronous dispatch. Cache identity includes the pinned artifact and
source identities, and retained identity checks compare the immutable bytes.
Only successful compile inputs become fallback/preflight replay inputs; a failed
replacement preserves the last accepted input. New processing owners import the
bytes again into their own native owners. Source references keep resolver/link
identity but cause no source download on this path.

Browser fixtures exercise worker/fallback admission, explicit pin refusal/retry,
changing data, literal text, and preservation of edits/focus/selection during
rerender. Native WASM fixtures reimport the same artifact into separate heaps,
refuse invalid builds/pins, dispose native owners and fence retired endpoints.
Shared unit fixtures cover mutation, superseded/disposed work and worker loss
after a rejected replacement. They preserve the old optional-cache source path.

```bash
node packages/cem_ql/tests/pinned-template-artifact-wasm.mjs
yarn vitest --config packages/cem-elements/vitest.unit.config.mts --run
yarn vitest --config packages/cem-elements/vitest.config.mts --project=storybook --run src/lib/pinned-template-artifact.stories.ts
```

Build the normal `cem_ql:build:wasm` and `cem-elements:build` targets before using
these checks against packaged assets. No separate minimal target is introduced.

## Build-generated declarations

The native `declaration_artifact::build_declaration_artifact` builder emits a
`cem-declaration-metadata/1` JSON sidecar, its independent CEM hash and binary
native template IR. It preserves the declared tag and optional build alias,
version, intrinsic capability, shared style scope, source URI/hash, resolved XML
bases, link policy, module maps, bindings, compiler versions and source-map mode.
Root capability precedence matches existing runtime registration. Attribute and
slice defaults stay in native IR; JavaScript does not reconstruct a template AST.

`CemElementRuntime.registerCompiledDeclaration(element, assets)` accepts
`{ metadataJson, metadataHash, templateBytes }`. The element has a matching `tag`
and no `src` or template children. Call it before connecting the element; it
marks registration pending synchronously. The host must obtain the expected
metadata hash from the admitted build manifest, independently of downloaded
assets. A colocated `.hash` file is useful for tests, not a trust anchor.

Native `admitDeclarationMetadata` validates exact metadata bytes and the compiler
contract. The host owns a copy of inputs before awaiting admission, validates
binary pins before defining a custom element, then reuses normal collision,
capability, form-association, style ownership and remount handling. Header
capability/scope/version/link-policy overrides and disposed scopes are refused.
Worker and fallback rendering import the pinned IR; they never request original
declaration source. Recovery replay uses the existing successful-input transport;
the complete compiled-form acceptance below now verifies the loaded application
across physical generation replacement.

Build one declaration, supplying its original deployment URI:

```bash
cargo build --locked --offline -p cem-ql --example compile_declaration_artifact
dist/target/debug/examples/compile_declaration_artifact \
  packages/cem-components/src/components/cem-text-field/cem-text-field.xhtml \
  dist/artifacts/cart/text-field \
  https://app.example/components/cem-text-field/cem-text-field.xhtml prod
```

The output is `text-field.bin`, `text-field.json` and `text-field.hash`. An
optional fifth argument sets a fixed build alias. Current support deliberately
requires a single named canonical CEM-ML declaration. XPath source companions,
external CEMT imports, legacy resource routing and unsupported declaration
attributes are refused rather than silently omitted. Native CSS parsing is still
required. Normal inline/`src` registration continues to use source.

Reproduce canonical form/action/card browser checks and separate-heap admission:

```bash
node packages/cem_ql/tests/build-declaration-fixtures.mjs
node packages/cem_ql/tests/declaration-artifact-wasm.mjs
cargo test --locked --offline -p cem-ql --test declaration_artifact
yarn vitest --config packages/cem-elements/vitest.config.mts --project=storybook --run src/lib/compiled-declaration.stories.ts --no-file-parallelism
```

Build the normal WASM and cem-elements assets first. Generated fixture files stay
under ignored `dist/`. Browser checks cover intrinsic defaults, FormData, actions,
scoped CSS, edits/focus/selection, remount, mutation, pin/capability/conflict refusal
and disposal during admission, with declaration downloads forbidden.

## Declarative asset loading and static closure

The ordinary shared runtime also accepts generated declarations directly:

```html
<cem-element tag="cem-text-field"
  compiled-src="./cem-text-field.json"
  compiled-template-src="./cem-text-field.bin"
  compiled-hash="cem-bin/1+blake3:BUILD_METADATA_HASH"></cem-element>
```

Use the complete 64-digit metadata hash emitted by the native builder. Both
asset attributes accept explicit relative/absolute URLs or import-map specifiers,
using the existing shared module resolver. The declaration must be named and
cannot combine these attributes with `src` or a source template. The normal
runtime installation is the only JavaScript setup; application UI stays
canonical CEM-ML. This does not add application-local loader code.

The shared byte reader omits credentials and refuses redirects. Same-origin
HTTP(S) is the default; the trusted host's existing HTTP resource policy may
allow cross-origin acquisition or lower limits. Responses require
`application/json` or `application/octet-stream`, respectively. Metadata pins
are checked before requesting the executable binary; binary pins are checked
before custom-element definition. Acquisition has a 15-second maximum deadline,
64 concurrent declarations, the host response ceiling (1 MiB by default), and
absolute metadata/binary ceilings of 1/64 MiB. Cancellation drains even an
uncooperative fetch. Scope disposal, changed URL attributes/resource context,
missing files, bad pins and mixed source inputs refuse registration. Explicit
registration or reconnect can retry failures. Admitted declarations reuse the
existing remount path without fetching assets again.

The build-only native `declaration_closure::build_declaration_closure` walks
compiled render IR, including conditional/loop/choice branches, and recursively
loads custom tags. It accepts entry and explicit projected-content roots,
deduplicates cycles and refuses missing/mismatched dependencies, duplicate roots,
inline runtime declarations and dynamic result-element construction. Bounds are
64 declarations, 16 MiB total source and 64 MiB emitted metadata/template bytes.
This is a **static declaration closure**: arbitrary computed query results or
host-projected components require explicit roots. It is not a general proof of
the complete runtime function, resource or language closure.

For a directory of canonical `<tag>/<tag>.xhtml` declarations:

```bash
cargo build --locked --offline -p cem-ql --example compile_declaration_closure
dist/target/debug/examples/compile_declaration_closure \
  packages/cem-components/src/components https://app.example/components/ \
  dist/artifacts/cart-components prod cem-card cem-text-field cem-action
```

All declarations compile and validate before output is written. The example
emits `<tag>.json`, `<tag>.bin`, `dependencies.json` and `declarations.html`.
Include the generated markup with asset URLs relative to the containing document's
resource base, or adjust the deployment prefix when assembling the page. Each
declaration admits independently; successful browser tag definitions remain if
a different dependency fails. This example is not a final deployment packager.

The [compiled form fixture](../packages/cem_ql/fixtures/component-application/compiled-form/README.md)
generates six declarations from an entry plus five canonical shared components.
Worker/fallback browser checks exercise real metadata/binary downloads, import
maps, bad-pin and HTTP refusal/retry, changed URLs, cancellation on scope disposal,
FormData, CSS, edits, focus/selection and remount. They forbid XHTML downloads.

```bash
node packages/cem_ql/tests/build-declaration-fixtures.mjs
cargo test --locked --offline -p cem-ql --test declaration_closure
yarn vitest --config packages/cem-elements/vitest.config.mts --project=storybook --run src/lib/compiled-declaration-loader.stories.ts --no-file-parallelism
```

## Compiled-application recovery acceptance

The [compiled recovery stories](../packages/cem-elements/src/lib/compiled-declaration-recovery.stories.ts)
load the generated six-component form in isolated browser realms and exercise the
existing shared checkpoint protocol. Each successful worker/fallback case completes
two real close/drain, native retirement, fresh allocation, native IR reimport,
all-owner binding/activation and fresh publication cycles. All six instantiated
owners participate. Both committed application and generation checkpoint bytes
stay within the existing 4 MiB budget; native handoff deadlines remain 5 seconds.

The original input node, FormData value, focus, selection direction/range and
installed style elements survive. Attribute changes and later accepted edits
render and publish in the fresh generation, while recovery itself does not
manufacture accepted input events. Original XHTML loading is disabled, component
metadata/binaries are not fetched again, and observed worker compilations use
only pinned native IR. Full WASM acquisition for fresh heaps remains allowed.

Password and one-time-code controls refuse capture and recovery-ticket issuance
while values are entered, including property-only edits without input events.
After explicitly clearing credentials, controls with empty authored/reset defaults
can be captured again. A previously committed safe checkpoint may remain during
refusal; the tests verify that credential-bearing data cannot replace those bytes.
They do not erase live credentials or broaden the privacy policy.

Four additional worker/fallback cases change either the input property or the
build metadata pin after retirement. The handoff refuses stale authority and
preserves newer values, selection/focus and the exact last-good checkpoint bytes.
This is acceptance of the existing shared adapters; no app-local behavior or new
runtime exception is introduced.

```bash
node packages/cem_ql/tests/build-declaration-fixtures.mjs
yarn vitest --config packages/cem-elements/vitest.config.mts --project=storybook --run src/lib/compiled-declaration-recovery.stories.ts --no-file-parallelism
```

Build the normal `cem-elements:build` artifacts first. This proves the reviewed
static form closure; it does not complete the shopping-cart transport, dynamically
constructed module closure, compiled CSS format or reduced binary.

## Remaining delivery boundaries

- Extend app-build function/resource closure generation and optional module
  acquisition beyond the verified static form. Ordinary `cem-element src`
  loading continues to support source.
- Stylesheet v1 reload imports CSS source. Prove compiled style/scoping admission
  with original identity and URL bases before removing its parser dependency.
- Generate/validate the app manifest and API contracts during the app build.
  The illustrative GraphQL files currently have no schema/operation validator in
  the fixture runner, and a manifest alone proves no native code omission.
- Bind real quantity/remove actions through shared declarative resources and
  GraphQL transport. The native render fixture sends no requests and has no
  interactive action bindings. Register a normal Studio example project with
  working build/preview only when those behaviors exist.
- Partition native dependencies/exports/registries and measure the separate
  compiled-UI + JSON distribution. Verify optional module acquisition, offline
  behavior and worker/fallback recovery with the same app state and semantics.

Existing full builds/defaults stay intact. Entered passwords/OTPs continue to
block recovery until cleared; no credential is stored in an app manifest or
public cache. The 4 MiB checkpoint limit and current deadlines are unchanged.
