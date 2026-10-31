# Shopping-cart application deployment profile

Status: native reference fixture, strict artifact admission, shared compiled
registration, declarative static-closure loading and compiled-form recovery in
worker/fallback hosts, plus an isolated compiled-template/JSON WASM prototype,
2026-10-29. A complete shopping-cart browser app, GraphQL transport/validation
and reduced component integration remain pending.
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
| Initial client | Compiled native render/evaluation, JSON decoding/encoding, required action/form behaviors, live DOM updates, app input checks, resources/cancellation and ownership/recovery guards. Compiled inline CSS uses opt-in v2; opt-in v3 closes imported CSS at build time. Source/dynamic-style paths retain CSS parsing. |
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
The ordinary full-runtime path still assembles its general function registry.
The selected-registry and isolated WASM paths below avoid that assembly.
Source-free reload by itself does not establish omitted implementations.

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

The seven app-boundary tests and nine existing artifact reload tests pass. This
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
requires a single named canonical CEM-ML declaration. Generated declarations now
use the opt-in `cem-template-artifact/2` format with compiled inline CSS and a
`templateFormat` metadata requirement. Legacy metadata without that field defaults
to v1; the shared host compares it with the actual admitted binary before defining
a custom element. XPath source companions,
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
static form closure; it does not complete the shopping-cart/GraphQL transport,
dynamically constructed module closure or reduced binary.

## Compiled inline stylesheet admission

`compile_template_artifact_with_compiled_styles` emits an opt-in
`cem-template-artifact/2` / `cem-template-ir-v2` envelope. Native declaration and
static-closure builds select this format; ordinary source/WASM compilation keeps
v1. Both formats remain supported by native and shared worker/fallback readers.
The existing processing transport stays v30, with explicit format agreement
checked against native import results. A v2 pin cannot accept a v1 body or invoke
a source fallback.

Each inline stylesheet carries a separately hashed `CEMCSS1` native capsule:
decoded CSS tree nodes, source maps/ranges, original URI and source hash, native
compiler version, and original CSS for provenance/inspection. Strings and source
maps are interned to reduce repeated data. Admission validates hashes, versions,
indices, graph ownership/reachability/depth, ranges and bounded expansion before
publishing a retained native tree. It does not tokenize, parse or validate CSS
source again. No JavaScript AST or new application behavior participates.

The capsule has a 16 MiB wire/expanded-accounting ceiling, at most 100,000 nodes
and 256 graph levels. The compiled-form fixture checks every binary against the
ordinary 1 MiB response policy. Do not infer allocator/RAM measurements from
these accounting limits or serialized file sizes.

The restored tree goes through the same native resource resolver, import loader,
private/shared/instance scoping, keyframe naming and managed CSS emission as the
source path. Native tests compare exact decoded nodes, URI/fingerprint/ranges,
resource/import requests, effective stylesheet identities, CSS and diagnostics.
A test-only parser counter proves zero CSS source-parser calls during capsule
admission. Invalid compiled data, missing pins and source-only v1 style payloads
inside a v2 envelope are refused.

V1/v2 external stylesheet imports keep their authorized `text/css` transport,
MIME/integrity/policy checks and native parser. Source-driven and dynamic
DOM/instance styles retain their current parser path. The opt-in v3 contract
below closes build-known imports; the full runtime and all current capabilities
remain available. No source parser has been excluded from the deployed WASM.

```bash
cargo test --locked --offline -p cem-ml --lib compiled_css::tests
cargo test --locked --offline -p cem-ql --lib template_artifact::compiled_style_tests
node packages/cem_ql/tests/build-declaration-fixtures.mjs
node packages/cem_ql/tests/declaration-artifact-wasm.mjs
```

## Compiled imported stylesheet closures

Use `compile_declaration_closure ... --compiled-css <root-tag>...` for the
opt-in `cem-template-artifact/3` / `cem-template-ir-v3` deployment contract.
Without that flag, the native closure builder keeps v2. The single-declaration
builder also keeps v2; native callers can explicitly upgrade its result through
`build_declaration_imports` with a caller-owned response loader.

The CLI maps CSS URLs under the explicit source-base URI to the components
input directory. It performs no network fetches, rejects outside-root/query URLs,
checks canonical paths to prevent symlink escapes and bounds each file before
reading it. CEM module-map resource mappings still determine import requests.
Native API callers supply their own authorized loader, including any redirect
handling; an artifact never grants filesystem or network access.

The build resolves and parses imports through `CssImportClosure`, retaining
source order, repeated occurrences, conditional imports, final URL bases and
source identities. Each imported full stylesheet becomes a pinned `CEMCSS2`
capsule with its original MIME profile. The v3 payload carries the closure's
original base and ordered request URLs. V1 scoped-block capsules, v1/v2 template
artifacts and their source import paths stay supported.

Reload validates every capsule without parsing CSS. A retained v3 stylesheet
then replays each dependency through the current native import state machine:
resolution, final-URL authorization, cycles/depth, MIME, source-byte SRI and
response/aggregate byte limits still apply. Scope compilation, resource URL
rewriting, keyframe names, emission, diagnostics, native ownership and managed
style installation all use the existing shared implementation. Runtime policy
is authoritative; stored responses cannot authorize a blocked URL.

Changed bases, remapped import URLs, missing/unused responses, corrupt capsules
and policy failures refuse the load. A v3 payload cannot silently downgrade to
v2 or fetch/parse source CSS as a fallback. Original CSS bytes remain solely for
provenance, inspection and integrity verification. Dependency fingerprints and
current resolution context continue to identify emitted styles.

Closures retain the existing 64-sheet (including the root), 16-import-depth,
16 MiB per-response and 64 MiB aggregate source-byte limits. Decoded capsule
expansion accounting is additionally capped at 64 MiB per closure; this is not
an allocator/RAM measurement. The shared loader's 1 MiB response ceiling still
applies to every generated form `.bin` asset.

The compiled-form fixture imports a stylesheet that imports another twice,
under conditional rules. Shared browser checks prove the properties are active
without fetching CSS. Recovery replaces real worker/fallback generations twice
while retaining installed style nodes, edited controls, focus and selection.
Entered passwords/OTP still block checkpoint capture and ticket issuance;
explicit clear and empty authored/reset defaults are required. Checkpoint sizes
and native handoff deadlines remain 4 MiB and 5 seconds.

```bash
cargo test --locked --offline -p cem-ml --lib compiled_css
cargo test --locked --offline -p cem-ql --lib template_artifact::compiled_style_tests
node packages/cem_ql/tests/build-declaration-fixtures.mjs
node packages/cem_ql/tests/declaration-artifact-wasm.mjs
```

The full WASM still includes native CSS parsing for source/dynamic consumers.
This gate establishes compiled import admission, not a smaller core binary or
a completed shopping-cart application.

## Native compiled requirements inventory

The native declaration-closure CLI now also emits `requirements.json` and
`requirements.hash`. The report is `cem-application-requirements/1` build-control
metadata. Its exact byte pin is generated natively; a deployment must obtain
trusted pins independently of downloaded files. The report does not grant native
functions, resource access or runtime permissions.

Requirement inspection uses admitted compiled artifacts and native IR, without
scanning/reparsing query source, evaluating queries or parsing application data.
Legacy v1 artifact admission retains its existing inline CSS importer; v2/v3
admit compiled inline CSS instead. The public Rust
`compiled_declaration_requirements` entrypoint validates metadata and template
pins/format agreement before inspection. The closure builder records explicit
entry/projected-content roots and the complete static component graph alongside
each declaration's exact metadata/source/artifact hashes, host bindings and shared
capability.

The report conservatively collects resolved standard-library module/name/arity
and dispatch kind from every IR node, including unused function bodies, lambdas,
attribute expressions, conditional branches and loops. Pipeline calls account
for their implicit stream argument; receiver operations and field reads are
listed separately. Catalogue membership is not proof that every dispatch path
is implemented. Unknown functions, unsupported pipeline adapters and unbound or
dynamic calls/callbacks remain explicit gaps.

Literal `data:read`/`data:parse` format/schema requests are recorded without
reading input values or evaluating options. These are requested profiles, not
validated parser/schema package selections. Dynamic format/schema choices and
additional import options require further contracts. Host/native dispatch,
template/state/module/source providers, projected/deferred payloads, shared
behavior bindings and HTML resource/style construction also remain open where
the compiled IR cannot establish their closure. Query text, arbitrary string
literals and runtime values are absent from the report.

CSS requirements use the same retained trees and native import state machine as
runtime style admission. V3 closures are traversed without loading source;
every stylesheet occurrence retains its original source URI/key, final URL base
and ordered import/URL requirements. Repeated sheets remain distinct. V1/v2
source imports are reported as unresolved; inspection does not fetch them.
Resource URL metadata is a requirement, not asset contents or a source grant.

Use `--require-static-requirements` to reject any unresolved requirements before
publishing closure assets. Omit it to preserve existing full-runtime builds and
produce a report showing the remaining gaps. Both modes emit the same native
template binaries and declaration metadata; the sidecars add build evidence.

```bash
dist/target/debug/examples/compile_declaration_closure \
  ./components https://app.example/components/ ./build/components prod \
  --compiled-css --require-static-requirements cem-static-app
```

The report explicitly has `nativeLinkClosureVerified: false`. Even a static
inventory does not prove transitive native implementation reachability, native
code omission, a selected registry or a smaller WASM. The next step maps these
requirements to implementation dependencies and explicit provider/source
contracts before assembling a reduced target.

Bounds remain 64 declarations, with at most 100,000 inspected template/IR items
and storage edges per declaration, 64 template nesting levels and a 1 MiB report
sidecar. IR storage cycles, dangling references and inconsistent tables fail
before publication. This bounds inspection; it is not a native RAM measurement.
The implementation is excluded from WASM with `cfg(not(target_arch = "wasm32"))`.
No Studio/application UI behavior, runtime admission or credential policy changes.

```bash
cargo test --locked --offline -p cem-ql --lib application_requirements
cargo test --locked --offline -p cem-ql --test declaration_closure --test declaration_artifact
node packages/cem_ql/tests/build-declaration-fixtures.mjs
```

## Native selected-registry prototype

`--selected-registry` adds `selected-registry.json`, its exact native content pin
in `selected-registry.hash`, and generated `selected_registry.rs`. Selection
requires a complete static inventory before any assets are written. Open
behavior/provider/source contracts, unreviewed functions, unsupported dispatch
adapters and catalogue-kind mismatches refuse generation. Existing builds without
this option keep the full-runtime path.

The initial reviewed set has 15 pure functions: sequence `first`, `last`, `count`,
`take`, `drop`, `map`, `flat_map`, `where`, `any`, `all`, `peek`; string `length`,
`lower`, `upper`, `contains`. Entries use exact module/name/arity identities.
The recipe binds `requirements.hash` and records the transitive execution-unit
closure and its source paths. Callback recursion closes through an explicit
fixed point: selected dispatch, evaluator/control, retained values/provenance,
field navigation, local lambdas, boolean conversion and the relevant sequence or
text helpers. These are reviewed execution contracts, not complete Rust symbol
reachability or native byte-removal evidence.

Generated Rust constructs an immutable `SelectedFunctionRegistry` from closed enum
identifiers. It contains no template/query text and grants no native callbacks,
resource access or application permissions. Native callers can pass it through
`TemplateData.selected_registry`, or use
`Evaluator::evaluate_with_selected_registry` with their existing operation
control/scope. Standard calls use a selected dispatcher and the existing
value/callback helpers; the full function catalogue is never assembled there.
The ordinary path remains the default. Native code using complete `TemplateData`
struct literals must initialize `selected_registry: None`; `Default` already does.

Unselected calls refuse before evaluating arguments. Native/URL special routes
and nested callback calls cannot fall back to the general dispatcher. Supported
sequence pipelines check the implicit receiver arity. Receiver methods remain
unsupported, including preexisting chain receivers; ordinary field navigation
and local lambdas retain their shared native semantics. This is an execution
selection for the prototype, not a sandbox for arbitrary host-provided values.

Selected template renders deliberately remain ineligible for native read receipts
and pure-render cache lookup/publication. Existing full-registry receipts cannot
match selected data. Registry identity and admission must join those reuse
contracts before enabling reuse; an output cache must not hide a refusal.
Cancellation and operation budgets still pass through the ordinary evaluator.

The existing cart entry template renders all six original/updated/removed/empty/
partial-error/failed native JSON fixtures with identical HTML and zero full
function-catalogue assemblies. This proves entry-template evaluation; it does
not close shared component behavior/provider contracts or implement storefront
networking/actions. CLI fixtures also compile and execute generated Rust and
refuse unreviewed/open requirements before publication.

The full/default runtime retains its general evaluator and optional language/tooling
implementations. The selected dispatcher is also available in the isolated WASM
prototype below. Recipe `nativeLinkClosureVerified` remains false: selected
execution and a smaller linked artifact do not close every component dependency.
Credential recovery refusal and the existing 4 MiB/5-second budgets are unchanged.

```bash
cargo test --locked --offline -p cem-ql --lib selected_registry
node packages/cem_ql/tests/build-declaration-fixtures.mjs
# A closed native prototype project:
dist/target/debug/examples/compile_declaration_closure \
  ./components https://app.example/components/ ./build/components prod \
  --compiled-css --selected-registry cem-static-app
```

## Isolated compiled-component WASM prototype

`component-runtime` is an opt-in Cargo feature propagated to `cem-ml`. The build
script's `--component-runtime` mode uses a separate target directory and writes
`packages/cem_ql/dist/wasm-component`; ordinary builds and their outputs stay on
the complete ABI. This prototype exports `renderCompiledApplication`, passive
`CompiledDeclarationOwner` methods, a counter-only inventory and wasm-bindgen
support, rather than either crate's general WASM tooling APIs.
The general standard-library/native/URL fallback and receiver-chain routes are
compiled out of this target's evaluator; legacy v1 CSS source decoding is also
excluded from its reload branch. This is a narrower execution boundary, not a
claim that every source parser or schema is absent from the linked result.

The function selection is trusted **build input**, never request metadata. Set
`CEM_APPLICATION_REGISTRY_RS` to the absolute path of native-generated
`selected_registry.rs`. Cargo watches both that variable and the selected file.
Without the variable, the generated registry is empty and function calls refuse;
it does not substitute the full registry. Arbitrary Rust build input has the
normal Rust build trust boundary; deployment artifacts cannot replace it.

The call accepts bounded native v2/v3 template bytes, independently pinned
hashes/versions/bindings/source-map mode, one original JSON response and its
absolute source URI. Shared `cem-ml::import::import_json_bytes` uses the ordinary
native JSON parser, limits, CEM-tree projection and retained owner, without the
all-format loader dispatcher. A parity fixture checks source identities, trees,
owners and malformed-input diagnostics against ordinary JSON import. No
JavaScript object or serialized replacement AST is involved.

Template/JSON inputs are each limited to 1 MiB, pin metadata to 16 KiB, and HTML
results to 1 MiB. Native admission/memory accounting remains active in WASM;
rendering uses a five-second operation policy. The one-shot entry owns its
inputs for the call; retained declaration owners are explicit below. Native
pointers and executable values never cross instances. Failed admission/rendering returns
a bounded error without echoing application content; there is no source fallback.
Selected data remains excluded from read-receipt/pure-render cache reuse.

The native fixture generator compiles the existing cart entry and writes reference
HTML for six changing JSON responses, plus valid denied-call, plain, v3 and
legacy-refusal cases. Its three-function selection is a **fixture**, not a closed
recipe for the cart's complete component behavior graph. The WASM driver checks
identical HTML on two independently instantiated heaps, refusal and subsequent
valid calls, absent full API exports, actual linked size reduction and code/data
section sizes. Measurements are written to
`dist/reports/components-wasm/selected-runtime.json`; Binaryen input/output pins
are in `dist/reports/wasm-optimization/cem-ql-component.json`.

Matched release builds use the same Rust toolchain, wasm-bindgen 0.2.122 and
Binaryen 132.0.0 `-Oz`. The full build retains its default debug/tooling surface;
the isolated build uses `--no-default-features --features component-runtime`.

| WASM measure | Full bytes | Prototype bytes |
| --- | ---: | ---: |
| Raw | 24,202,382 | 11,474,094 |
| Gzip level 9 | 7,372,989 | 3,243,949 |
| Brotli quality 9 | 5,422,451 | 2,430,810 |
| Code section payload | 21,443,655 | 9,176,474 |
| Data section payload | 2,729,774 | 2,284,396 |

The prototype is 10.94 MiB raw, a 52.6% whole-file reduction.
It has 10 imports/21 exports, compared with 28/171 for the full runtime.
The public surface is the one-shot renderer, passive declaration-owner methods
and a counter-only inventory; remaining exports are wasm-bindgen support. This proves a smaller linked surface
and reduced code/data sections, without attributing removed bytes to particular
languages, schemas or functions. Complete application link closure remains false.

Prototype SHA-256: `b9ae8e07460c1d04143355f30909d7e83bbf3439b727fe450185fef8da92937b`.

Full SHA-256: `d57c36d274e564eae7b826bc7b0fb69286ed15ffce9b08d168722aa31837ede3`.

```bash
cargo run --locked --offline -p cem-ql --example component_runtime_fixture -- \
  dist/artifacts/cem_ql/component-runtime
CEM_APPLICATION_REGISTRY_RS="$PWD/dist/artifacts/cem_ql/component-runtime/selected_registry.rs" \
  node tools/scripts/build-cem-ql-wasm.mjs --component-runtime
node packages/cem_ql/tests/component-runtime-wasm.mjs
# Native default-deny fixtures deliberately use no selected build recipe:
env -u CEM_APPLICATION_REGISTRY_RS cargo test --locked --offline -p cem-ql \
  --lib --features component-runtime application_runtime
```

## Retained passive declarations and stylesheet leases

The selected ABI now admits `CompiledDeclarationOwner` from original compiled
bytes, declaration metadata and an independently supplied metadata hash. Native
admission checks both metadata/template identities, the actual v2/v3 envelope
format, module-map equality and authored stylesheet scope. Declarations requiring
a widget capability refuse admission until their owned input/provider/recovery
contracts exist. Source-only artifacts and source fallback remain unavailable.

Each owner reuses `RetainedTemplate`, the native CEM JSON importer and the selected
renderer. `renderJson` accepts original JSON bytes, their absolute source URI and
a pinned host-binding name. It returns HTML and keeps data in native CEM trees.
It does not construct a DOM data island or register a browser custom element.

`renderInputs` accepts one multi-root native CEMV artifact and a routing array.
JavaScript transports the original bytes and JSON control metadata containing
only `target`, `name`, `index` and an optional slice `attribute`. Native code uses
its existing CEMV decoder, `TemplateData::bind_native_attribute` and
`bind_native_slice` APIs. Attribute roots must match their requested names;
authoritative typed value sequences are used instead of compatibility strings.
A slice can keep a native root/reference intact or unwrap the values of a named
native attribute. Typed values, reference identity and original source maps stay
native. This is not a JSON AST or document-shape adapter.

This entry bounds the input artifact to 1 MiB, routing to 16 KiB/64 bindings,
values/references to 16,384 and depth to 64, with the existing five-second native
operation control. Encoded bytes and accounted native graph storage share the
1 MiB decoder budget. Input names use ASCII letters/digits, underscore and hyphen,
starting with a letter or underscore. Runtime roots (`datadom`, `attributes`,
`slices`, `island`, `instanceID`, `suggestions`, `input`, `position`), duplicate
aliases across either channel, invalid indices and unknown control fields refuse.
Failures return a fixed diagnostic without embedding input content. Every call
uses a fresh native input frame and releases it after rendering. Explicit
attribute/slice envelopes make channel contents independent of routing order;
the declaration owner stores no live inputs or checkpoints. A failed call leaves no stale inputs
for the next call.

`renderInputs(bytes, bindings, instanceId)` optionally supplies host-owned
`instanceID` through a separate native `renderInstanceInputs` entry. Existing
two-argument calls are unchanged. Identity must be a nonempty string of at most
1,024 UTF-8 bytes without whitespace/control characters and must have a pinned
host binding. The shared adapter rejects unpaired UTF-16 surrogates before
UTF-8 transport, so encoding cannot silently change the supplied identity.
Unicode and punctuation remain values and are escaped by the native HTML
projector. The identity is bound only in the temporary render frame;
a later call cannot inherit it. It is not an ownership, placement or recovery
grant, and it is never automatically reflected to the DOM. Attribute/slice
routing cannot replace runtime roots. Owned declaration `renderJson` calls now
also refuse `instanceID`, `island` and `suggestions` as JSON binding targets;
`datadom` keeps its existing native JSON behavior.

Compiled receiver `{attribute ...}` declarations use the existing native renderer
for required values, defaults, scalar conversion and facets. For example,
`@type=integer @required=true @minInclusive=1 @maxInclusive=5` converts an
incoming native string `003` to integer `3` in both the attribute alias and
`datadom.attributes`, and rejects missing, invalid or out-of-range values. A
looser producer value cannot override the pinned receiver declaration. Invalid
defaults and unresolved named types refuse with a fixed diagnostic, without
copying input content into errors. These contracts are already in pinned native
template IR; no runtime schema text or JavaScript conversion path is added.

The native-only `application_declaration::build` wrapper now packages reviewed
named scalar receiver contracts with a v2/v3 declaration. Its `CEMAPPD1` binary
contains the unchanged pinned template bytes and native compiled facet records.
The independently pinned `cem-application-declaration/1` control sidecar retains
the original declaration JSON/hash, the whole binary hash and each type's schema
URI, physical source URI, source-byte hash and compiled-contract hash. The trusted
app build supplies already validated native scalar models; this wrapper does not
compile arbitrary executable datatype definitions or prove issuer authority.

The opt-in generation's existing `open(bytes, metadataJson, metadataHash)` accepts
this envelope. Admission verifies all identities and requires exact agreement
between named `attribute @type` uses found in native IR, metadata requirements and
provided contracts. Missing, changed, duplicate, unused or unresolved types,
dynamic type selection, trailing/unknown binary fields and stale pins refuse
before a declaration owner is published. Named aliases cannot shadow shipped
types or the reserved `schema:` namespace; names/URIs grant no scope access.
Full-runtime declaration admission refuses the application envelope; it cannot silently discard required types.
The regular v1 metadata and v2/v3 template inputs retain their existing behavior.
`metadata()` returns the normalized original declaration; native owner identity
uses the outer application pin, including its type-policy identity.

Each fresh JSON, binary-input or island frame receives the owner's pinned named
contracts through the shared native `value_types` binding. Required/default,
scalar conversion and receiver-local restrictions use the existing renderer;
a local facet cannot weaken its named base. The fixture's `cart-count` base range
2–5 intersects a receiver maximum of 4; `003` becomes integer `3`, while 1 and 5
refuse. String pattern, minimum length and whitespace collapse are also covered.
Types are released with the declaration; no schema text, catalog lookup, host
callback, JavaScript type conversion or source fallback is involved at runtime.

This first capsule admits bounded native scalar facet models over string,
integer, decimal/number and boolean bases, with enum, numeric/length/digit,
pattern and whitespace restrictions. It keeps original model source maps and
explicit schema/source identities. Schema and physical source identifiers are
bounded hierarchical URIs in this first capsule. There are at most 64 types, 16 restrictions
per type, 64 KiB accounted per contract and 1 MiB aggregate accounted contracts;
the binary and metadata each retain their 1 MiB limits. Native IR inspection is
bounded to 100,000 nodes and 64 levels. Executable datatype/tokenizer callbacks,
node schemas, unresolved native type slots and other model policies remain
refused until reviewed portable contracts exist. Patterns use the existing shared
runtime facet validator; this gate does not establish precompiled regex/executable
IR or per-parser code omission. No checkpoint authority is added.
The [portable native schema contract design](cem-native-schema-capsules.md)
defines separate inert records, explicit fresh-runtime linking and atomic
publication for future executable/node contracts. Those compiled-component adapters are not yet
implemented; v1 refuses them even with internally consistent integrity pins.
Native Rust now has a separate `CEMSC1` literal structural schema exporter and
verifier. It preserves a source arena and passive namespaces, pins local
attribute/child dependencies, and reloads inert shared validation models without
source parsing. The inert native dependency envelope described below can now
carry this capsule. WASM transport and fresh-host compiled-component runtime
admission remain pending. The
[implemented subset and limits](cem-native-schema-capsules.md#implemented-native-structural-subset)
describe its scalar fields, child-count rules, provenance and refusal policies.
A native `FreshStructuralHost` can now link that verified structural model to an
original literal schema node receiver and validate authorized targets through
shared native rules. Live context/invocation receipts refuse stale hosts,
relabelled captures, changed receiver metadata and retired contracts. This does
not admit template named-node bindings or alter the deployed WASM ABI.
Native `CEMQV1` programs can now restore a bounded compiled query
subset through the shared validation registry; original checked declarations
can travel in a `CEMQD1` native source capsule and bind to an explicitly registered
fresh native host scope. Native `CEMNR1` now carries exact validator/tokenizer
requirements and binds actual explicitly registered implementations to the same
fresh host. It preserves the shared compiler's checked tokenizer replacement
and unavailable outcomes; callbacks, preparation tokens and grants stay local.
Direct typed preparation/conversion/equality/list-serializer adapters now feed
the ordinary native compiler with weak fresh-host guards. Constant/facet/shared
list-item and canonical native source-result bindings now retain those guards
through preparation, result adaptation and retained attribute consumers.
The separate native-only
[CEMAPPD2 dependency envelope](cem-native-schema-capsules.md#bounded-native-application-dependency-envelope)
now packages compiled declarations and structural/query/native requirements in
content-addressed native blobs. Independently trusted pins, exact declared graph
closure, recomputed IR requirements and registry recipe, bounded canonical
verification and complete-source sharing are implemented. Structural receiver
requirements are inert metadata, not live receiver bindings. Explicit contract
edges do not prove complete semantic/native code dependencies. Native named-node
assembly now checks original receiver occurrences/local cardinality and retains
explicitly linked query/native guards, with checked native slot replacement.
Scalar/executable named receiver composition, physical worker/fallback transport
and component admission/publication remain pending. Scalar CEMAPPD1 and the current WASM ABI remain unchanged and
refuse this new native envelope.
The native-only
[datatype source contract](cem-native-schema-capsules.md#native-datatype-source-closure-and-compilation)
now preserves exact original local scalar/list source dependencies and compiles
explicit registrations through the shared native compiler on a checked fresh
host. Only exported roots validate; copied sources, stale/foreign contexts and
missing implementations refuse. This does not prove complete behavior/native
code closure or enable scalar/executable named receiver admission. Existing
application formats and the WASM ABI remain unchanged.
Native root preparation now freezes input and issues sealed immutable evidence,
with fresh validation on consumption, cumulative budgets and exact live issuer
checks. Shared tokenizer retirement guards protect root and attribute receipts.
The [handoff contract](cem-native-schema-capsules.md#guarded-datatype-preparation-handoff)
keeps complete behavior/imported closure and receiver/runtime admission pending.
Native CEMDP1 now records original dependency fields and exact DT1/QD1/NR1
inputs, including passive references. Its [inert dependency assertions](cem-native-schema-capsules.md#inert-datatype-dependency-assertions)
retain original lexical scopes but do not prove target selection or grant access.
The [fresh-host selection adapter](cem-native-schema-capsules.md#fresh-host-datatype-dependency-selection)
now checks actual contexts, original names, explicit exports and directed grants.
Its weakly guarded inspection results grant no compilation/publication permission.

The [native linking contract](cem-native-schema-capsules.md#native-fresh-host-linking-and-node-receivers)
defines its required source registration, explicit grants, bounds and lifetime.
The [native schema/type fixture](../packages/cem_ql/fixtures/component-application/named_receiver_types.rs)
compiles original schema facets and emits independent full-renderer reference
outputs for the browser/WASM transport tests.

Complete DOM data-island construction, multiple independent input artifacts,
executable/node named-schema contracts, widget/provider adapters and worker/recovery
integration remain delivery gates for using the complete component library on
this profile.

The passive native island gate adds `renderIsland(bytes, instanceId)` and
`projectIsland(bytes, instanceId)`. Both consume one native CEMV v3 context root
(or a document containing that root), check the nine ordered expanded-name parts
against the canonical lifecycle topology, and require lifecycle `0.2.0`.
Hydration contains exactly the matching version, host `instanceId` and
`outputTarget=light-dom` string fields. Namespace prefixes are not identity;
expanded names are. A native drift fixture checks the schema's declared part
order/namespaces and the shared runtime version.

The island gate now binds qualified attributes, dataset and slices through
native semantic views over the original CEMV nodes. Literal attribute entries
require unique canonical lowercase host names and string values. The dataset
must exactly equal the DOMStringMap projection of the original `data-*`
attributes, including hyphen conversion, punctuation and the empty key. Its
values share the corresponding attribute value identity/source. Receiver
conversion updates semantic aliases and `datadom.attributes`; dataset strings
remain the original lexical values (`003` stays `003` while an integer receiver
sees `3`). Attributes and top-level slices share at most 64 aliases; duplicate,
reserved runtime-root and cross-channel collisions refuse. `dataset` is also
reserved at this island boundary. Ordinary attribute aliases use the existing
bounded ASCII naming policy; canonical `data-*` attributes additionally admit
HTML-safe suffix punctuation and Unicode needed by DOMStringMap.

Qualified `slice` entries declare `string`, `boolean`, `number`, `null`, `record`
or `array` explicitly. Strings preserve whitespace, booleans require `true` or
`false`, and numbers use shared finite-decimal validation without JavaScript
number coercion or precision loss. Null has no content. Records require unique,
nonempty child names of at most 256 UTF-8 bytes; arrays require contiguous ordered
indices starting at zero. Nested values retain their original container/value source keys and maps through native
record/array/scalar views. Keys such as `kind`, `constructor` and `__proto__` are
ordinary record members, never inferred document/node shapes. Host strings use
the original `value` attribute as their source. The shared CEMV writer now also
preserves atomic provenance on re-export. CEMV still exports only native nodes
and atomic values: select record fields or array members for export; whole
semantic records/arrays refuse rather than inventing a JSON document.

The [native domain fixture](../packages/cem_ql/fixtures/component-application/island-domains.xhtml)
and native unit tests cover source identity, exact scalar types, nesting, alias
bounds, malformed input and refusal. The native fixture generator independently
supplies typed control data to the full renderer for reference HTML; two-heap
WASM/browser checks transport CEMV without decoding/reconstructing the graph.

Resources, forms, validation and events must remain empty until their reviewed
adapters exist. Payload identity must be `text/html` with
`https://cem.dev/ns/data/html/1`, containing canonical lowercase HTML names in the
HTML namespace. Native CEMV references may select further references or these
reviewed HTML/text/whitespace/CDATA/comment nodes. Document, attribute and atomic
targets, foreign namespaces, nodes without required source metadata, alternate
payload schemas and CEM-ML source handoffs refuse.
The root, reference occurrences and every rendered target/descendant require
source maps, nonempty source keys and absolute hierarchical source URIs. These
checks retain supplied native source metadata; they are not issuer authentication or complete HTML schema validation. The
producer still owns source import and app-build/domain validation. XML and
CEM-ML parsing for fixtures/builds stays outside this runtime entry.

`island` points to the original qualified context root. `datadom.payload` is the
native payload section, retaining original reference occurrences, target edges
and source parents. `datadom.slots` indexes direct rendered roots: direct
references are followed in target order and each terminal root supplies its own
`slot` name, including the empty default name and ordinary keys such as
`__proto__`. Multi-target references can supply several slots; repeated targets
remain repeated entries sharing the original native node identity. An empty
reference contributes no slot entry. A reference inside an HTML element stays
inside that element and does not become a root slot entry. Grouping appends native
values without cloning source nodes or constructing JavaScript data records.

Admission checks the complete rendered reference/child closure before publishing
any groups, including references nested inside HTML. Every referenced password,
file or credential-autocomplete control is subject to the same refusal as direct
payload controls. The shared CEMV decoder checks the entire retained graph's
backlinks, dangling targets, cycles, depth and expanded work; the island traversal
also polls operation control and bounds visits. Source-parent edges stay
provenance/context edges, not rendered content or a schema/recovery grant. Native
fixtures exercise repeated, nested, multi-target and empty references and compare
HTML and opaque CEMV against an independent full-native renderer on separate heaps.
`renderIsland` returns native HTML. `projectIsland` exports the native render
result with the existing CEMV writer, preserving reference/source edges and
supporting empty output. Referenced source context can include unrendered
siblings/ancestors needed for source-parent access. The projection is derived
output, never replacement island state: feeding it back into either island entry
refuses. JavaScript transports bytes and never decodes the value graph.

Input/expanded-value and output budgets remain 1 MiB with 16,384 values/references,
depth 64 and five-second native operation control. Each call releases its input
frame; owners retain no island or projection handles. Password/file controls and
password/one-time-code autocomplete metadata refuse even when empty because this
entry has no live property/capture proof. The same refusal applies to host
`type=password|file` and credential autocomplete attributes. This does not change
the full runtime's entered-credential recovery policy. It is not a checkpoint
permit, live-control registration, DOM installation or worker/recovery adapter.
Those gates, remaining island domains and app-specific named schema contracts
remain open.

Styles reuse the existing native resolver, compiled-import policy replay,
`RetainedStylesheet` identity/scoping emission and last-consumer invalidation.
The declaration metadata fixes the base URL, declaration identity and private or
shared scope; callers supply only consumer/occurrence and current resolver
context. V2 source imports refuse instead of returning a fetch request; v3
responses must be build-closed and pass current URL/MIME/integrity/byte policy.
Imported responses retain the existing full-stylesheet profile restriction.
`text/css; mode=stylesheet` now overrides automatic scoped-block inference for
`:host`, `@scope` and `scope` hints; `entry=stylesheet` also works when `mode` is
absent. The owned-import fixture uses an explicitly declared stylesheet response
containing `:host`. Source and compiled loading retain the same original MIME,
URI, bytes, source keys and native scoped emission. Unqualified/scoped inputs are
still refused as response capsules when they produce a scoped-block root; no
capsule, current-resource or credential checks are relaxed.

Every attempted native style load gets a private consumer identity. Failure
removes its loader bookkeeping without touching a previous valid lease; success
publishes the replacement before releasing the old native consumer. Owners are
bounded to 64 per heap, 64 styles per declaration and 128 consumer/occurrence
leases per owner. Stylesheet context controls are bounded to 64 KiB and emitted
CSS to 1 MiB. Disposing/freeing an owner invalidates its output leases. The
counter-only inventory counts native declaration allocations until `free`, even
if `dispose` already made them inactive, so an external root cannot disappear
from retirement accounting.

The shared `@epa-wg/cem-elements/component-generation` entry has no runtime import
of the full native facade. It consumes the separately generated
`createCemComponentWasmBindings` factory and an immutable compiled
`WebAssembly.Module`. Initialization awaits asynchronous native instantiation,
which works with Chromium's main-thread synchronous-instantiation size limit.
Each factory invocation owns separate glue caches, class finalizers and memory.
Raw native objects stay inside the adapter; callers receive guarded declaration
operations and metadata copies.

`installStylesheets` commits native CSS through an existing
`CemStylesheetConsumerLease` from `DeclarationStyleOwnership`. Connected-scope
checks, staged replacement, context markers, shared membership, style-node reuse
and disconnect cleanup remain owned by that service. It does not parse CSS or
add application UI behavior. A failed/stale/reentrant commit releases only its
own attempt. Active calls block disposal, closing prevents new owners, and
cleanup is idempotent. Generation disposal refuses foreign native allocations
and clears its binding caches only after its own roots/leases reach zero.

The browser fixture uses the real profile, native-built private/shared v2/v3
styles and the existing DOM ownership service. It verifies style application,
stable prior nodes on import-policy refusal, disconnect cleanup and exactly one
profile WASM fetch with no full-runtime/source downloads. The Node acceptance
also checks two independent heaps, native JSON/attribute/slice renders, original
native HTML parity, bad metadata, disposed-unfreed foreign roots and reentrant
cleanup. The native-input browser fixture verifies escaped text, changing inputs,
post-refusal recovery and one profile WASM fetch without source/full downloads.
Receiver fixtures additionally compare native reference HTML across changing
identities, required/bound failures and independent heaps, and verify DOM identity
escaping without materializing markup from the supplied identity. The factory gate pins the reviewed generated
binding shape and refuses extra exports/imports/finalizers.

```ts
import { createCemComponentGeneration } from '@epa-wg/cem-elements/component-generation';
import { createCemComponentWasmBindings } from './selected-runtime/cem_ql_factory.js';

// Package the separate profile assets explicitly; this does not select a Studio backend.
const module = await WebAssembly.compile(
  await (await fetch('./selected-runtime/cem_ql_bg.wasm')).arrayBuffer(),
);
const generation = await createCemComponentGeneration(module, createCemComponentWasmBindings());
const declaration = generation.open(templateBytes, metadataJson, metadataHash);
// declaration.installStylesheets(consumer, context, existingStylesheetLease);
// declaration.renderJson(originalJsonBytes, responseUri, admittedBinding);
// declaration.renderIsland(admittedNativeIslandBytes, 'cart-item-1');
// const nativeProjectionBytes = declaration.projectIsland(admittedNativeIslandBytes, 'cart-item-1');
// declaration.renderInputs(nativeValueBytes, [
//   { target: 'attribute', name: 'label', index: 0 },
//   { target: 'slice', name: 'amount', index: 1 },
// ], 'cart-item-1');
declaration.dispose();
generation.dispose();
```

After the build/fixture commands above and the ordinary `cem-elements:build`:

```bash
node --test tools/scripts/cem-components-wasm-factory.test.mjs
node packages/cem_ql/tests/component-owner-wasm.mjs
yarn vitest --config packages/cem-elements/vitest.config.mts --project=storybook \
  --run src/lib/component-generation.stories.ts --no-file-parallelism
```

This remains an experimental backend, with complete live-component registration,
worker transport, widget/provider/source ownership, native input/type contracts,
checkpoint/journal handoff and demand modules still open. Standalone owner
cleanup is not a recovery permit or checkpoint endpoint. Studio continues using
the full runtime. Entered-credential refusal and the existing checkpoint/handoff
budgets remain unchanged; these passive owners do not capture widget state.

## Remaining delivery boundaries

- Extend app-build function/resource closure generation and optional module
  acquisition beyond the verified static form. Ordinary `cem-element src`
  loading continues to support source.
- Extend compiled inline CSS admission to imported stylesheet closures before
  removing the CSS source parser. Keep authorized source imports, source-driven
  templates and dynamic styles available through the full/on-demand runtime.
- Generate/validate the app manifest and API contracts during the app build.
  The illustrative GraphQL files currently have no schema/operation validator in
  the fixture runner, and a manifest alone proves no native code omission.
- Bind real quantity/remove actions through shared declarative resources and
  GraphQL transport. The native render fixture sends no requests and has no
  interactive action bindings. Register a normal Studio example project with
  working build/preview only when those behaviors exist.
- Extend the isolated prototype to the complete compiled-UI + JSON distribution
  with verified native dependency/ABI closure. Verify optional module acquisition, offline
  behavior and worker/fallback recovery with the same app state and semantics.

Existing full builds/defaults stay intact. Entered passwords/OTPs continue to
block recovery until cleared; no credential is stored in an app manifest or
public cache. The 4 MiB checkpoint limit and current deadlines are unchanged.
