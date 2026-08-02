# Temporary Decision: Native AST Transform Boundary

Status: Option C selected in the active todo; exact artifact API remains
proposed, temporary, and non-normative until accepted

Scope: CEM-ML transform inputs, graph artifacts, adapter compile/render
boundaries, XPath evaluation, and explicit JSON AST exports

## Decision Question

Should `serde_json::Value` remain the common transform data carrier, or must
transform tiers pass native AST/event/item-stream artifacts without an implicit
JSON projection?

Recommended decision: remove JSON values from the transform data plane. Native
AST artifacts are the only primary in-process representation. JSON is permitted
only when an input is being parsed as JSON or an explicitly selected output has
a registered JSON or `+json` content type.

## Why JSON Appeared

Commit `94fd20b` introduced the transform adapter execution API before an
executor existed. `TransformTemplateCompiledArtifact`,
`TransformTemplateDataArtifact`, and `TransformTemplateOutputArtifact` used
`serde_json::Value` as an opaque, cloneable, serializable placeholder. The
initial runtime-adapter test passed a small JSON object because no typed data
artifact contract had been defined yet.

The stated reason for the adapter boundary was dependency inversion: `cem_ql`
already depended on `cem_ml`, so `cem_ml` could not call the CEM-QL renderer
directly. The historical documentation described an opaque plugin artifact; it
did not declare JSON to be the semantic transform representation.

The placeholder later became an internal conversion boundary:

- `load_transform_data_artifact` lifecycle-loads the source, then ignores its
  typed `LoadedInputAstStream` as the data body and calls
  `projection::dom_json`.
- `TransformTemplateDataArtifact.value` carries the resulting JSON value
  between graph and adapter tiers.
- `template_data_from_artifacts` and `expression_policy_bindings` convert that
  JSON value into CEM-QL item streams.
- graph joins synthesize JSON objects containing nested artifact values.
- `TransformTemplateOutputArtifact.value` uses the same field for native CEM
  trees, text, JSON, and other result categories.

This sequence is an AST-to-JSON-to-engine conversion even when neither source
nor target content is JSON. It loses representation identity at the type level
and permits accidental JSON use to spread through adapters. It should be
treated as architectural debt rather than precedent.

Relevant implementation locations:

- `packages/cem_ml/src/real.rs`: `load_transform_data_artifact` and graph joins
- `packages/cem_ml/src/transform_template.rs`: transform artifact contracts
- `packages/cem_ml_transform_cem_ql/src/lib.rs`:
  `template_data_from_artifacts`, `expression_policy_bindings`, and
  `value_to_stream`
- `packages/cem_ml/src/lifecycle.rs`: `LoadedInputAstStream`

## XPath Reference Assessment

The Xee repository was verified from its official GitHub source at commit
[`200b1e3356ea9d6dd2901d67bd941b779df7e5b7`](https://github.com/Paligo/xee/tree/200b1e3356ea9d6dd2901d67bd941b779df7e5b7)
on 2026-08-01. Its root and component crates use the MIT license. The source may
be studied, adapted, and modified, but copies or substantial portions require
the upstream copyright and permission notice.

Assessment: Xee provides enough architectural and algorithmic reference to
start a CEM-owned XPath implementation, but it is not sufficient as the sole
specification or as a reusable CEM execution boundary.

- Its documented pipeline covers lexer, syntax AST, specialized IR, bytecode,
  interpreter, item sequences, maps/arrays/functions, and an extensive standard
  function library. Its vendored QT3 integration is useful as a test-harness
  design reference.
- Upstream describes XPath support as almost complete, reports missing Functions
  and Operators implementations, lacks deep XML Schema integration, and does
  not yet expose general extension-function bindings. CEM cannot infer missing
  behavior from that code.
- Xee owns XML nodes in a Xot arena and its runtime uses `Rc<RefCell<_>>`
  document state. Adopting those types would replace, rather than preserve, CEM
  XML AST identity and would not satisfy the native/WASM transform artifact
  contract.
- Xee's dynamic-context defaults can read local time and can initialize process
  environment variables. Its file loader reads paths directly, while `doc()`
  consults an evaluator-owned document store. Those facilities must not cross
  into the CEM security model.

Therefore the W3C XPath 3.1, XDM 3.1, Functions and Operators 3.1, serialization,
and error specifications are normative. Applicable QT3 tests are conformance
inputs under their W3C test-suite license and require a separate provenance and
license review before vendoring. Xee is a non-normative implementation
reference pinned by commit. Every adapted algorithm must record the source file
and commit; copied substantial portions additionally require MIT attribution.

No Xee compiler, interpreter, high-level XPath, Xot, or loader crate may become
a runtime dependency. The currently pinned `xee-xpath-lexer` and
`xee-xpath-ast` crates are transitional foundation dependencies and must be
replaced by CEM-owned token and syntax AST types before executable XPath is
registered. During replacement they may serve only as a parity oracle in tests.

### Accepted XPath Conformance Scope

Xee is enough to begin implementation, but it cannot establish that CEM has a
complete XPath 3.1 implementation. Its own conformance statement excludes
unsupported features and lists missing standard functions, deep schema
integration, and extension-function support. The CEM work therefore needs an
explicit initial conformance profile and a machine-readable gap matrix against
the normative specifications and applicable QT3 tests.

Full XPath 3.1 is accepted as the destination, delivered in vertical
conformance slices. The first executable slice covers native XML context
navigation, predicates, variables, deterministic core functions, and typed
sequence results. Unsupported standard behavior produces a stable typed
capability or static/dynamic error and remains visible in the gap matrix; it
must never silently inherit Xee's omissions. Per-file provenance is required
for algorithms adapted from the pinned Xee source.

## Representation Rules

1. Parsing produces a native, source-mapped AST or event stream.
2. That same AST artifact is passed between lifecycle, graph, transform, query,
   and evaluator tiers. Tiers may borrow it, wrap it in `Arc`, or create a lazy
   typed view; they must not project it through JSON merely to cross an API.
3. An adapter may walk the source AST to build evaluator-local indexes or node
   handles. That is evaluator preparation, not a replacement transport format.
   Source/node identity and source maps must remain linked to the original AST.
4. Serialization is an explicit conversion or writer operation. It is never an
   implicit transport step between in-process tiers.
5. A content type identifies content, while a representation kind identifies
   the in-process carrier. Relabeling an AST with a JSON content type does not
   serialize it and must be rejected.
6. Encoded text or bytes may leave an encoder and enter a writer. To become a
   transform input again, they must pass through an explicit lifecycle decoder
   or parser edge and produce a new typed AST artifact.
7. Control-plane configuration and diagnostic/report projection are separate
   from the data plane. JSON may be accepted at CLI/config ingress, but it must
   be normalized into typed options or parameter values before execution.

## JSON-Specific Rules

JSON source is not an exception to the native-AST rule:

- `application/json` input is represented internally by `JsonDocumentAst` and
  `JsonValueAst`, preserving duplicate object members, lexical numbers, source
  ranges, and parse facts.
- JSON Schema input is represented by `JsonSchemaDocumentAst`.
- Cross-format JSON/YAML/CSV data uses `GenericDataDocumentAst` and
  `GenericDataValueAst` where a common data model is required.
- `serde_json::Value` is not the internal representation of any of those ASTs.

JSON serialization is allowed only at an explicitly identified JSON boundary,
for example:

- `application/json`
- a registered `application/*+json` content type
- `application/vnd.cem.dom+json`
- `application/vnd.cem.events+json`
- `application/vnd.cem.xpath-result+json`

The typed `XPathResultArtifact` remains the evaluator and transform-stage
result. Its `Serialize` implementation supports the explicit
`application/vnd.cem.xpath-result+json` export; it does not make JSON the
internal XPath result carrier.

## Proposed Data-Plane Contract

Replace the generic `value: Value` fields with an explicit native/encoded
artifact model. The exact names can change, but the separation is required.

```rust
pub struct TransformDataArtifact {
    pub artifact_id: String,
    pub uri: Option<String>,
    pub identity: Option<FormatIdentity>,
    pub body: TransformArtifactBody,
}

pub enum TransformArtifactBody {
    Lifecycle(Arc<LoadedInputAstStream>),
    CemDocument(Arc<CemDocument>),
    GenericData(Arc<GenericDataDocumentAst>),
    CemTree(Arc<CemTreeAstStream>),
    XPathResult(Arc<XPathResultArtifact>),
    Collection(Arc<TransformArtifactCollection>),
    Extension(Arc<dyn TransformNativeArtifact>),
    Encoded(Arc<TransformEncodedArtifact>),
}

pub trait TransformNativeArtifact: Any + Send + Sync {
    fn representation_id(&self) -> &'static str;
    fn source_map(&self) -> Option<&SourceMapStack>;
    fn as_any(&self) -> &dyn Any;
}

pub struct TransformEncodedArtifact {
    pub identity: FormatIdentity,
    pub encoding: TransformEncoding,
    pub bytes: Arc<[u8]>,
}

pub enum TransformEncoding {
    Text,
    Json,
    Binary,
}
```

The hybrid enum is recommended:

- explicit variants make core AST handling exhaustive and reviewable;
- `Extension` preserves the plugin boundary without a crate dependency cycle;
- `Encoded` makes terminal serialization visible instead of disguising it as a
  generic value;
- `Collection` lets graph joins retain references to typed artifacts rather
  than copying their content into a JSON object.

`TransformTemplateOutputArtifact` should carry the same body model or a stricter
output equivalent. A transform may return another AST artifact for downstream
processing or an encoded artifact for an explicit target/writer. It cannot
return an untyped JSON value.

## Adapter Contract

Each executable adapter declares accepted input representation IDs and produced
representation IDs. Selection requires both format identity and representation
compatibility.

Examples:

- XPath accepts an `XmlDocumentAst`, XML-family typed AST, or an explicitly
  supported owner-node/subtree view. The CEM-owned evaluator traverses those
  nodes through stable native handles and retains original CEM node identities
  and source maps.
- CEM-QL walks native CEM, lifecycle, generic-data, or collection artifacts and
  exposes lazy `ItemStream` views. It does not call `value_to_stream` on a JSON
  projection of non-JSON content.
- CEMT consumes the typed CEM tree/event artifact required by the selected
  function stage.
- an explicit DOM JSON exporter consumes a native document AST and produces an
  encoded JSON artifact with `application/vnd.cem.dom+json` identity.

Evaluator-local structures do not replace the artifact body. An XPath adapter
may build indexes or compiled instructions over stable CEM node handles, but it
must not construct a replacement XML tree, reparse source text, or expose Xee
AST, Xot, program, or value types across any tier.

## Enforcement

The primary protection must come from Rust types, not conventions:

1. Remove `serde_json::Value` from `TransformTemplateDataArtifact` and
   `TransformTemplateOutputArtifact`.
2. Do not derive `Serialize` or `Deserialize` for internal native artifact
   containers. Provide separate report/projection DTOs.
3. Make JSON encoders return `TransformEncodedArtifact`, never a generic value.
4. Require `TransformEncoding::Json` to have a registered JSON or `+json`
   content type. Reject identity/encoding mismatches at construction.
5. Require encoded artifacts to pass through an explicit parser edge before an
   adapter can consume them as AST input.
6. Keep projection helpers in export/writer modules and do not expose them as
   generic transform-data constructors.
7. Normalize CLI/config JSON parameter values into a typed transform parameter
   model before compile/render.

Add structural and behavioral verification:

- a source audit fails if `load_transform_data_artifact` calls `dom_json`,
  `to_cemt_subject`, `serde_json::to_value`, or an equivalent projection;
- an adapter audit fails if a generic `serde_json::Value` is used as its primary
  semantic input;
- `Arc::ptr_eq` tests prove the lifecycle AST body is retained across load,
  graph routing, and adapter dispatch;
- duplicate JSON object members survive internal transform routing;
- XML comments, processing instructions, namespace identity, node identity,
  lexical ranges, and source-map stacks survive routing;
- graph collect/join artifacts preserve typed child bodies and order without
  synthesizing JSON;
- JSON output tests require an explicit JSON target identity;
- non-JSON transforms fail if any implicit JSON projection is attempted;
- native and WASM executions produce equivalent explicit output artifacts.

A narrow source audit is useful as a regression tripwire, but it is secondary
to removing the generic value fields. As long as the data-plane API contains a
`Value`, accidental JSON transport remains easy and cannot be reliably ruled
out by tests.

## Migration Order

1. Add red tests that demonstrate current loss or bridging: AST identity across
   transform load, duplicate JSON members, XML node/source identity, and typed
   graph collection children.
2. Introduce `TransformArtifactBody`, `TransformNativeArtifact`, typed
   collections, and encoded artifact constructors without changing adapters.
3. Change `load_transform_data_artifact` to retain `LoadedInputAstStream` or the
   native CEM document rather than calling `projection::dom_json`.
4. Change graph routing and joins to pass or reference typed artifacts.
5. Migrate the CEM-QL/CEMT/XSLT adapters to typed or lazy AST views.
6. Remove `value_to_stream` as the generic transform ingress path. Keep explicit
   JSON-AST-to-query conversion only when the input identity itself is JSON and
   the adapter intentionally consumes `JsonDocumentAst` or
   `GenericDataDocumentAst`.
7. Change transform outputs to typed AST or encoded artifacts and enforce
   identity/encoding agreement.
8. Replace the transitional Xee lexer/parser dependencies with CEM-owned XPath
   token and syntax AST types, using pinned Xee source only as a parity oracle
   and non-normative implementation reference.
9. Only then implement the CEM-owned XPath compiler/evaluator and register
   `TransformTemplateKind::XPathExpression`.
10. Add explicit JSON AST/result exporters as conversion graph edges.

This order prevents XPath from depending on a boundary already known to violate
the native-AST rule.

## Options

### Option A: Keep `serde_json::Value`

Rejected. It preserves plugin convenience but cannot assure that non-JSON ASTs
avoid internal JSON projection. Content identity, AST identity, duplicate
members, lexical representation, and source maps remain convention-dependent.

### Option B: Native AST With JSON Fallback

Not recommended. A fallback becomes the easiest adapter path and recreates the
current bridge. It also makes native-AST support optional rather than an
invariant.

### Option C: Strict Native AST Data Plane With Explicit Encoded Edges

Recommended. It gives compile-time separation between native ASTs and encoded
content, supports external adapters through a native trait object, and permits
JSON exactly where content identity explicitly requires it.

## Decision To Record

- [x] Accept Option C as the transform-boundary invariant and track its
      migration in `docs/todo.md` immediately after XPath.
- [ ] Accept the hybrid core enum plus `TransformNativeArtifact` extension
      model.
- [x] Require removal of the existing implicit DOM JSON bridge before XPath
      transform registration.
- [x] Treat JSON/report DTOs as export/control-plane structures only, never as
      transform data-plane bodies.
- [x] Accept Xee commit `200b1e3356ea9d6dd2901d67bd941b779df7e5b7` as a
      non-normative, MIT-licensed implementation reference only; do not add Xee
      evaluator/Xot/runtime dependencies, and replace existing Xee syntax
      dependencies before XPath execution.
- [x] Accept full XPath 3.1 as the destination with staged conformance slices,
      a specification/QT3 gap matrix, stable unsupported-feature diagnostics,
      and per-file provenance for adapted Xee algorithms.

Option C is now promoted into `docs/todo.md`. The remaining decisions are the
exact native artifact API and its extension boundary. Once those are accepted,
promote the stable invariant into the CEM-ML package README and replace this
temporary note with a permanent ADR plus implementation records.
