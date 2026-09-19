# XSLT-authored data-table viewer

Status: native JSON projection, reusable query/template recovery, the
separately approved native-owner loader repair and XPath-owned XML node
normalization are implemented and verified. The user also approved the generic
native query-function hook; its runtime integration is implemented and verified
below. The compiled-bundle/browser-host delivery path is approved, with
independently identified XPath programs. The XPath artifact foundation is
implemented; XSLT bundle delivery and bounded root-template runtime lowering
are implemented. Template dispatch, parameters, modes, imports/includes and
strict transform/CLI execution, bounded grouping and stable sorting are now
implemented. Subsequent viewer stages remain open in [todo.md](todo.md).
Explicit host position and sequence size are now supported by the shared XPath
API. No equivalent XSLT stylesheet or browser sample is available yet.
The shared parsing and typed-error extension was approved on 2026-09-18 and is
implemented through CEM import and native buffered recovery. The bounded
contract and verification are recorded under XSLT-VIEW-DATA and in the
[runtime profile](xslt-runtime-lowering.md#standard-parsing-and-recovery).
The shared [native output construction](xslt-runtime-lowering.md#native-output-construction)
extension was approved on 2026-09-18 and is implemented. Native nodes and atomics
remain distinct through calls/recovery; output AVTs, whitespace, expanded names,
attribute ordering and static result-style exports use the shared renderer.
The viewer stylesheet and browser parity demonstration remain open.
The 2026-09-19 native probes identified missing stylesheet access to stable
source-selection keys/line numbers and a CSV header-profile mismatch. The
[viewer selection and CSV proposal](xslt-runtime-lowering.md#viewer-selection-and-csv-parity-gate)
was approved and implemented on 2026-09-19, including matching native CEM-QL
provenance functions. The next [presentation decision](xslt-runtime-lowering.md#namespace-declaration-parity)
concerns namespace declarations: the CEMT reference displays them as attribute
columns, while native XPath correctly excludes them from its attribute axis.

## Scope

Author an XSLT equivalent of
[`data-table-view.cemt`](../packages/cem-elements/demo/data-table-view.cemt),
translate it through the existing native XSLT-to-CEMT route, and demonstrate the
same rendering and interactions in an additional gallery case. Keep the direct
CEMT examples and their imported presentation aspects as the parity reference.

XSLT-specific parsing, validation, lowering and integration are authorized,
plus the explicit `cem-ml` JSON AST projection and reusable query/template
error recovery approved on 2026-09-13, including native module-call integration.
The separate native-owner loader repair was also approved on that date.
The user subsequently directed XML normalization into the existing XPath/XDM
layer, reused by XSLT, without changing core XML/CEM AST or default imports.
The user also approved the reusable native query-function invocation hook on
2026-09-13 and the XSLT-owned compiled bundle with explicit browser/WASM host
loading on 2026-09-14. XPath must retain its own namespace and content types,
as CEM-QL does. The shared XPath host position/size and XSLT dynamic group-context
extensions were approved on 2026-09-18. The shared CEM-QL call-depth accounting
correction was also approved on that date, preserving configured limits and
cumulative function-call accounting. Native standard parsing through CEM import,
typed import errors and standard error identity for XSLT recovery were also
approved on that date. Shared native result construction was also approved,
including native copying, atomic/text normalization, attribute ordering, typed
failures, bounded namespace fixup and portable output. Shared source-selection
keys, original line-number access in both query languages, and explicit CSV
header options were approved on 2026-09-19. Other shared capability expansions
still require approval.
If an essential operation needs a new CEMT/CEM-QL capability, importer behavior
or browser runtime behavior, stop and ask before implementing it. No table-
specific Rust rendering, demo-local JavaScript, browser XSLTProcessor, or opaque
embedding of the original CEMT implementation in an XSLT wrapper.

The standard feature contract is [XSLT 3.0](https://www.w3.org/TR/xslt-30/)
and its applicable XPath function library. Implement available standard features
in the compatibility layer, not as CEM extensions. Explicitly namespaced CEM
extensions are allowed only for capabilities without a standard equivalent,
such as CSV/YAML parsing and browser-specific state/provenance access.

## Bounded feature list

| Feature | Required lowering and evidence |
| --- | --- |
| Runtime execution | Emit executable CEMT instructions; imported data and changed state are evaluated at render time, not conversion time. |
| XPath context | Viewer-required axes, namespace-aware node tests, predicates, variables, position, sequences, comparisons and conditional expressions retain XPath semantics. |
| Maps and arrays | Implement only operations needed for projection; preserve member boundaries, empty members, value types and native node references. Do not silently substitute CEM collection semantics. |
| Templates and modes | Recursive named calls, parameters, match rules and explicit modes lower to existing native dispatch. |
| Imports and overrides | Translate XSLT import precedence, priority and declaration ordering into the existing module/match machinery; verify notes-tree and IP-filter-form overrides. |
| Grouping | `xsl:for-each-group`, `current-group()` and `current-grouping-key()` use existing generic collection operations with stylesheet-owned row detection. |
| Columns and cells | First-seen union headings, attribute/direct-text/child cells and heterogeneous rows are authored in XSLT. |
| Sorting | Stable dynamic multi-key `xsl:sort`; stylesheet-defined validity keys place missing/invalid values last in both directions. Preserve source identities and stable ties. |
| Standard parsing | Standard XML/JSON functions return their specified data models and errors; do not relabel `data:read` as a conforming standard function. |
| Extensions | CSV/YAML import and CEM-only host state/source provenance delegate exclusively to existing native capabilities. |
| Error recovery | Preserve the viewer's parse-error display and recovery using standard XSLT constructs; pause if recoverable evaluation needs shared engine support. |
| Result construction | Literal HTML, AVTs, whitespace, styles and existing declarative event/slice attributes retain output and interaction semantics. |
| Validation | Document supported forms, source ranges and resource limits; unsupported forms produce diagnostics rather than partial output. Recognizing version 3 does not establish execution conformance. |

## Native data-model gate

Check both standard JSON routes before choosing a lowering:

- [`fn:parse-json`](https://www.w3.org/TR/xpath-functions-31/#func-parse-json)
  produces maps/arrays and maps JSON null to an empty sequence **inside its
  array member**, without removing that member.
- [`fn:json-to-xml`](https://www.w3.org/TR/xpath-functions-31/#func-json-to-xml)
  produces a document containing the standard XPath-functions-namespace tree,
  including `map` elements and keyed value elements in source order.

By default the CEM reader intentionally returns its own typed AST projection:
`cem:generic-data` object/property/array/scalar nodes. That is the correct input
for the existing CEMT viewer, not automatically either standard JSON result.
The feasibility check must retain native owners and provenance; converting
this tree to a string and reparsing it is not an acceptable adapter.

### Original gate result and approved resolution

Three original native characterization tests in
[`xslt_data_model_boundary.rs`](../packages/cem_ql/tests/xslt_data_model_boundary.rs)
establish the existing contracts, not standard XSLT conformance:

- CEM's array IR collects flat item streams: empty sequences contribute no
  members and multi-item sequences contribute several. `Item::Array` and
  `QueryItemView::members()` expose item vectors, not sequence-valued XDM
  members. There is currently no surface CEM-QL array constructor either.
- `data:read` retains JSON null slots, object member order and native source
  identity, but exposes `cem:generic-data` object/property nodes rather than
  the standard `map` and keyed-value node structure. The data itself is not
  lost; the missing piece is an appropriate native query view.
- An ordinary query record can keep a native source reference, but assigning
  fields called `kind`, `name` and `namespace` does not construct a native
  node with identity and source maps.

The array representation and evaluator are in
[`eval.rs`](../packages/cem_ql/src/eval.rs); the current native reader/view is
in [`eval/data.rs`](../packages/cem_ql/src/eval/data.rs). Query function dispatch
was fixed in [`eval/pipeline.rs`](../packages/cem_ql/src/eval/pipeline.rs);
at the original gate, the evaluation context had no native-function hook for
generated CEMT. The subsequently approved generic hook is implemented below;
it does not itself change array semantics or implement standard JSON functions.

**Approved resolution:** the user placed projection ownership in `cem-ml`
JSON import. `validation::json_xml::project_json_to_xml` now builds the native
CEM AST directly from `JsonDocumentAst`, including keyed values in the W3C
namespace, source-order duplicate policy, XML-safe escaping and source maps.
It uses iterative traversal with explicit limits. The original JSON AST stays
unchanged and is retained by the query view.

Select it with `data:read(source, "json", "json-to-xml")` or
`cem-data @type=json @projection=json-to-xml`. Omitted projection (or explicit
`cem`) keeps the previous shape and identity. The explicit projection has
separate identity, no partial root on error, and requires JSON input. Existing
typed CEM-tree presentation works directly; no XML serialization is involved.

Native Rust options support escape marking and retain/use-first/reject keys.
The query profile uses untyped nodes, `escape=false`, duplicate retention,
32 KiB input, depth 64 and 4096 values. Schema-typed output, custom fallback
callbacks and liberal JSON parsing are not provided. In particular the existing
JSON parser rejects unpaired-surrogate escapes; this projection does not change
that input contract or claim complete `fn:json-to-xml` conformance.

The subsequently approved standard string-import profile now handles BOMs and
surrogate codepoints for native `fn:json-to-xml`, while leaving the default
reader profile above intact. Its typed errors, options and XSLT catch lowering
are documented in the [runtime profile](xslt-runtime-lowering.md#standard-parsing-and-recovery).

The shared XPath layer owns standard parsing function contracts and error names;
XSLT lowers catch selection and scope through generic recovery. The standard
node-tree route avoids changing CEM array semantics or
adding the full XDM map/array model solely for this viewer.

Alternatives checked:

- Directly renaming `data:read` to a standard JSON function changes its specified
  result semantics and is not acceptable under the standards-first boundary.
- Lowering `parse-json` directly to current CEM arrays drops null array members;
  introducing a private tagged XDM collection model would be another shared
  representation decision, not a simple syntax translation.
- Record-shaped pseudo-nodes do not provide the native projection contract.
  String construction followed by XML/CEM reparsing loses the native handoff.

The approved projection/data-reader selection and subsequent approved error
recovery work change shared code.
Do not advertise an equivalent stylesheet until the remaining lowering and
parity checks pass; pause again if they need another shared capability.

## Error-recovery scope gate (2026-09-13)

Four original native characterization tests in
[`xslt_error_recovery_boundary.rs`](../packages/cem_ql/tests/xslt_error_recovery_boundary.rs)
confirmed the behavior before approval (and remain compatibility checks outside
the new recovery constructs):

- Invalid XML and JSON return a native reader report with an error string and
  no root, but no evaluation error or query diagnostic.
- `report:emit(..., "fatal")` records a diagnostic; evaluation can still return
  subsequent values. It does not raise a catchable dynamic error.
- Empty-sequence coalescing works for absence, but propagates a failed query
  instead of evaluating its fallback.
- An expression failure inside a named template call adds diagnostics but
  retains already-rendered content and renders later siblings. There is no
  existing CEMT instruction for catching that failure and replacing the failed
  region with a recovery result.

These are characterization tests, not a proposed change to ordinary CEM error
behavior. The four probes and nine existing data-model/legacy-XSLT tests pass
together. No production code changed for this gate.

Standard [`xsl:try` / `xsl:catch`](https://www.w3.org/TR/2017/REC-xslt-30-20170608/#try-catch)
handles dynamic errors within the protected evaluation and delivers the selected
catch result on failure. Output recovery defaults to enabled. Checking a reader
report before rendering can handle that particular parse operation, but does
not by itself implement recovery for failures in later expressions or called
templates. Logging a fatal diagnostic is not an equivalent replacement.

The existing [AC-QE-2](cem-ql-ac.md#11-errors-diagnostics-and-reports) already
places query `try { … } catch (code, msg) { … }` in Tier B. At the original
gate this was a planned capability, not executable support or authorization
to implement it in this XSLT-scoped task.

Decision resolved: the user approved reusable recovery. The original options
were:

- **Reusable recovery (recommended):** authorize the planned generic query
  failure/recovery capability and scoped CEMT recovery with buffered output.
  Preserve structured failure information and source maps, propagate failures
  across nested calls, and keep host cancellation/resource-limit enforcement
  intact. Existing reader-report and diagnostic-only behavior must remain
  backward compatible. XSLT error-name mapping, catch selection and standard
  syntax remain in the compatibility layer; no table-specific code belongs in
  this shared work. Detailed native acceptance fixtures come before changes.
- **Parser-only XSLT profile:** investigate a deliberately restricted
  `xsl:try` whose protected expression is a supported parsing call, with a
  catch-all recovery expression. Lower through reader-report checks without
  shared changes, and statically reject broader recovery forms or unsupported
  error-variable access. This could retain the viewer's parse-error display,
  but is not general expression/template/aspect recovery. Record that narrower
  acceptance contract before implementing it; do not silently advertise wider
  `xsl:try` support.

The approved implementation supplies `report:raise(code, message)`, query
`try { ... } catch (code, message) { ... }`, and buffered CEMT `try`/`catch` with
ordered predicates, native source-mapped failure records, output/binding
rollback and nested/imported call propagation. The CLI adapter now invokes
module calls inside rendering instead of expanding them after recovery has
finished. Diagnostic-only emission and existing reader reports remain unchanged;
host cancellation, resource limits and unsupported engine capabilities cannot
be swallowed. See the [public recovery contract](../packages/cem_ql/README.md#error-recovery).

Verification on 2026-09-13: all 293 `cem-ql` tests pass, including eight new
query and ten new template recovery tests. The CLI recovery test passes all
three cases (direct, named and imported calls). The Nx WASM build and three
fresh WASM checks pass, as does Nx lint (with existing Clippy warnings).
The initial broader adapter and CLI XSLT runs exposed the two failures below;
the loader repair now resolves the adapter failure, while the stylesheet-export
failure remains open.

Runtime/template, bounded grouping, stable multi-key sorting and standard
parsing/catch lowering were completed in later milestones. The equivalent
gallery cases remain open. Generic recovery supplies buffering and failure
propagation; shared XPath supplies parsing and structured error names, while
XSLT supplies catch-name matching and standard variable scope.

## Document-loader regression discovered during verification

The adapter's existing
`common_query_runner_executes_registered_cem_ql_runtime_with_native_owners`
test initially failed before query evaluation with `cem.query.input_model_unsupported`,
also in isolation. In `cem_ml/src/real.rs`, the Markdown-only helper called
`loaded.ast_stream.take()` and returned on other variants without restoring
them, discarding the native XML owner. The same bug was present in `HEAD`,
and that file was unchanged by the recovery work.

Fixing the general document loader was outside the approved recovery/projection
changes. The user separately approved preserving non-Markdown AST owners and
adding native XML/JSON/CSV/YAML regression coverage on 2026-09-13. Keep parser
semantics unchanged; do not skip the test or route native ASTs through
serialized data.

The repaired helper borrows the retained AST and replaces it only after a
successful Markdown-to-HTML conversion. Seven new native tests cover
XML/JSON/CSV/YAML/HTML ownership, absent ASTs and failed Markdown conversion;
the existing successful conversion test remains green. Allocation identity,
typed AST/source metadata, bytes, format, adapter selection and diagnostics
are retained. All 204 engine tests and 93 adapter tests pass, including the
previously failing query-runner case. The three CLI recovery cases also pass.
Both explicit JSON-boundary audits, fresh Nx lint (existing warnings), the WASM
build and five WASM data-read checks pass. Those checks cover XML/CSV/YAML/JSON
and the explicit JSON-to-XML projection. Lint and WASM have overlapping Nx
cache outputs; after an initial concurrent build lost a dependency artifact,
sequential uncached verification succeeded without changing task configuration.

The broader CLI XSLT regression run also finds
`graph_config_projects_inline_style_export_to_css_and_links_html` failing:
the rendered page lacks the stylesheet link. Template compilation already
extracts literal styles into stylesheet sidecars (also present in `HEAD`),
while the XSLT render/export path does not consume them. The recovery changes
do not modify style extraction or export policy. This is recorded under
XSLT-VIEW-OUTPUT; the other eight existing CLI XSLT cases and the new three-case
CLI recovery test pass. Do not report the broader suite as fully green.

## XML node-model scope gate (2026-09-13)

Before implementing runtime XPath paths and standard XML parsing, four native
probes in
[`xslt_xml_model_boundary.rs`](../packages/cem_ql/tests/xslt_xml_model_boundary.rs)
checked the source-oriented CEM tree and the existing native XPath owner.
All four probes and 13 existing JSON/error/legacy-XSLT tests pass. These tests
characterize current behavior; they do not endorse it as XDM conformance.
No production code changed for this gate.

[`fn:parse-xml`](https://www.w3.org/TR/xpath-functions-31/#func-parse-xml)
returns an XDM document, not a lexical XML event stream. The
[XDM text-node rules](https://www.w3.org/TR/xpath-datamodel-31/#TextNode)
require adjacent character content to form one nonempty text node. Namespace
declarations are separate from ordinary
[attribute nodes](https://www.w3.org/TR/xpath-datamodel-31/#ElementNode).

| Probe | Current native CEM reader | Required normalized view |
| --- | --- | --- |
| `<r>a<![CDATA[b]]>c</r>` | Three distinct source-mapped nodes: text, CDATA, text. | One text node whose value is `abc`. |
| `<r><![CDATA[]]><x/> <![CDATA[z]]></r>` | Empty CDATA, element, whitespace and CDATA. | Element followed by one text node whose value is ` z`. |
| XML declaration and namespace declarations | XML declaration appears as a PI; `xmlns` entries appear among attributes. | No XML-declaration node; namespace bindings are not ordinary attributes. |

The probe also found that a real `<?keep yes?>` instruction currently receives
the fallback target `xml` in the CEM reader, while its retained value is
`keep yes`. The existing query view exposes expanded local-name/namespace
fields, but not lexical prefix, parent/root or document-order metadata.
Those omissions must be accounted for when selecting the bounded path forms.

Before normalization, reusing `cem-ml`'s `XPathNativeNode` was not sufficient:
executing `/r/text()` against the first probe returned three distinct native
event handles, although they correctly retain the original owner and source
maps. Its namespace/declaration filtering is useful existing machinery, not
proof of normalized text-node behavior.

Filtering CEM children to text/whitespace/CDATA does not fix XPath cardinality,
positions or node identity. Concatenating their strings fixes only their text
value. An ordinary record describing the combined text is not a native node;
serializing and reparsing XML violates the native data handoff boundary.
Consequently this required a native XPath-view correction, not a token rewrite
or table-specific rendering requirement.

### Approved XPath-layer resolution

The user directed this work to XPath/XDM, reused by XSLT. The earlier proposal
to add a generic reader projection was broader than needed and is superseded.
`XPathNativeNode` now presents logical XML nodes over the original immutable
event owner. Adjacent text/CDATA/predefined and numeric reference events use
the first event as their canonical node identity, retain every contributing
source span, and expose a single decoded text value. Empty text runs and
document-level whitespace are omitted. Axes, positions, comparisons, ordering,
string values and atomization consume these logical nodes.

Namespace declarations are excluded from XPath attribute constructors as well
as axes; expanded names preserve namespace shadowing. PI nodes retain their
target names, and PI node tests carry their parsed targets in the XPath AST.
Target matching follows the [XPath PI-test rules](https://www.w3.org/TR/xpath-31/#id-sequencetype-matching).
Literal XML line endings normalize without changing numeric CR references.
The XML event AST/importer, CEM AST and default `data:read` have no changes from
this work; the three default-reader characterization tests remain unchanged.

All 11 tests in
[`xpath_xml_view.rs`](../packages/cem_ml/tests/xpath_xml_view.rs) and 86 existing
XPath unit tests pass. Coverage includes constructor identity, source maps,
namespace/PI metadata, axes, sequence limits, cancellation and unchanged owners.
The 22 XSLT unit tests and 23 reader/XML/JSON/error/legacy-XSLT regression tests
also pass; fresh uncached Nx lint succeeds with existing warnings.
The sequential uncached WASM build and five WASM reader checks pass, including
unchanged XML text/CDATA boundaries and CSV/YAML/JSON/projection shapes.
This remains an untyped bounded view, not schema-aware XML, DTD/entity expansion
or a claim of complete XPath/XDM conformance.

## Runtime query-function integration gate (2026-09-13)

A native test now proves `XsltXPathInvocationAdapter` executes the same
XSLT-owned select AST against changed XML input, returning the normalized nodes
with original ownership and provenance. No expression string or serialized AST
is handed across that invocation boundary.

The original missing connection was generated CEMT at render time:

- `XsltParityTransformTemplateAdapter::compile` still converts source into a
  CEMT artifact and renders through CEM-QL; it does not invoke the typed XPath
  adapter. Normalizing the XPath view does not change that route.
- CEM-QL's `EvaluationContext` had bindings and URL resolution, but no host
  native query-function hook; stdlib dispatch was fixed.
- `TemplateCallHandler` returns a render plan and failure, not a typed query
  result that expressions can bind, filter or sort.
- `data:read` exposes its CEM tree and keeps its source-format XML owner private.
  Renaming it `fn:parse-xml` would still return the wrong node model. Rebuilding
  a source string or inventing records with node-like fields is not a native
  XPath handoff.

**Approved resolution:** the user approved the generic invocation hook, with
XPath/XSLT semantics retained in their own layers. `native:call(identifier,
...arguments)` now dispatches through an explicit native registry supplied in
the CEM-QL/CEMT runtime context. It preserves native item/sequence boundaries
and provenance, carries operation controls, enforces call/result budgets, and
participates in existing error recovery. The registry is not a data binding,
cannot be populated from JSON, and is absent from compiled artifacts. Hosts
must explicitly rebind it after reload. No new core AST/importer behavior or
table-specific code is introduced.

The XSLT-owned invocation adapter now has a controlled entry point that retains
host validation and uses the caller's operation scope. A native integration
fixture in
[`native_xpath_calls.rs`](../packages/cem_ml_transform_cem_ql/tests/native_xpath_calls.rs)
retains one typed XSLT XPath AST in a callback and invokes it through CEMT
against changed XML owners, preserving normalized text identity/source maps.
This is evidence for the runtime connection, **not** a completed XSLT-to-CEMT
compiler. The fixture's native node wrapper is test-only; the production
lowering still owns its argument/result adaptation and standard error mapping.

Verification: all 308 CEM-QL tests pass, including 11 native-hook tests, along
with 93 transform-adapter tests, the native XPath integration fixture, 86 XPath
unit tests and the three-case CLI recovery fixture. Sequential uncached Nx
lint and WASM builds pass with existing lint warnings. Nine WASM smoke checks
verify missing-capability failures, JSON capability isolation, CEMT recovery
boundaries and unchanged XML/CSV/YAML/JSON reader projections.

Runtime lowering, the equivalent `data-table-view.xslt`, and the additional
browser sample remain open. The generic hook decision is resolved; do not ask
for it again, and do not equate these integration fixtures with viewer parity.

## Compiled-program delivery decision (approved 2026-09-14)

The in-memory native hook alone does not define how a CLI-produced template
reaches the browser with its executable XPath programs. Inspection established
the following delivery boundary:

- `XsltParityTransformTemplateAdapter` lives above both `cem_ml` and `cem_ql`.
  At that delivery audit its compiler still used the legacy converter;
  XSLT-VIEW-MATCH has since replaced execution with typed bundles. The original
  CLI-native payload alone was not a browser-loadable artifact.
- The portable CEMT artifact carries CEMT/query IR. It intentionally excludes
  the callback registry and any typed XPath AST retained by a callback.
  `cemt_binary_reload_requires_explicit_capabilities_and_preserves_recovery`
  already verifies that loading such an artifact without explicit capabilities
  cannot execute its calls.
- `cem_ql::api::wasm` retains only `TemplateArtifact`; its JSON render input
  constructs callback-free `TemplateData`. Browser runtime support calls this
  boundary. Neither source loading nor CEMT module-closure loading supplies the
  missing XSLT programs or rebinds their capabilities.

**Approved:** deliver an XSLT-owned compiled bundle
containing the generated CEMT and its compiled XPath programs/imports, with an
explicit browser/WASM host path that validates and retains the bundle and
rebinds only its declared capabilities. Define executable artifact versioning,
hashes, source maps, import ownership and disposal before implementation.
This is a deployment artifact boundary, not serialization/reparsing of runtime
data ASTs or XPath source on every render. Keep presentation in the stylesheet,
default generic CEMT artifacts/callback registries unchanged, and ordinary JSON
bindings unable to install capabilities.

The user additionally requires XPath to own its namespace and content type,
like CEM-QL. Do not relabel compiled XPath as CEM-QL, substitute legacy token
rewriting, bake output from one input, or publish unresolved callback IDs.

### XPath-owned artifact foundation

The existing XPath namespace `https://cem.dev/ns/query/xpath/1` and source type
`application/vnd.cem.xpath` remain unchanged. Its new compiled program type is
`application/vnd.cem.xpath-artifact+cem-bin`, distinct from both CEM-QL artifacts
and XPath result metadata. The XPath package owns the versioned binary codec,
identity/hash validation and bounded typed-program reload. Reload preserves
source ranges, host ownership and static namespaces without reparsing XPath or
serializing runtime data. The source syntax model gains no serde dependency.

Explicit WASM `compileXPathArtifact`, `importXPathArtifact` and
`disposeXPathArtifact` entry points expose this control-plane boundary. Native
XSLT-owned programs can be retained by the WASM host, independently of CEMT
handles. Loading does not install native callbacks; JSON render data cannot
bind them. Expected content/source hashes, host identity, versions, binary
limits, retained-program limits and non-reused handles are checked. See the
[XPath package contract](../packages/cem_ml/schema-packages/xpath/v1/README.md#compiled-programs)
for API details and the explicit-import limitation.

Verification: six native artifact tests, two malformed-codec tests and 13
native/WASM checks pass. The uncached XPath package verification target also
passes schema registration, source/lifecycle compatibility, CLI examples and
README checks. The shared WASM host builds successfully; native CEM-QL lint
passes with warnings.

This completes the independently identified program foundation, not the XSLT
bundle or an executable viewer. The approved next work is bundle composition,
explicit capability binding, typed stylesheet lowering, equivalent stylesheet
and browser parity. Keep those checklist items open; the delivery decision
itself is resolved and does not need to be requested again.

## Bundle invocation focus gate (2026-09-18)

Bundle composition must specify how each compiled XPath slot receives its
dynamic context. The original public `XPathDynamicContext` accepted a context
item, variable bindings and default language, but no context position or size.
`XPathFocus::outer` therefore created singleton focus whenever an item was
present. The XSLT invocation adapter delegated to that same evaluator.

Two native characterization fixtures in
[`xslt_bundle_focus_boundary.rs`](../packages/cem_ml_transform_cem_ql/tests/xslt_bundle_focus_boundary.rs)
originally exercised XSLT-owned expressions and source-free binary reloads
over retained imported CEM nodes. They established:

| Invocation | Observed result |
| --- | --- |
| Host invokes a label expression separately for items A and B | `A:1/1`, `B:1/1`; three-item input likewise reports `1/1` for every call |
| One XPath expression iterates with `/r/item ! ...` | `A:1/2`, `B:2/2`; an inner predicate has its own focus and restores the surrounding focus |

The native owner and stylesheet source provenance survived reload. The missing
contract was outer focus supplied at invocation, not program serialization or
node import. The original probes characterized the limitation and did not
establish XSLT loop conformance.

Initial gate verification: the Nx adapter test target passed all 97 cases,
including the two probes and existing native-hook integration. Adapter lint
passed with existing warnings. That gate changed tests and documentation only.

XPath defines focus as the context item, position and size; XSLT iteration
sets these for the selected sequence. See
[XPath dynamic context](https://www.w3.org/TR/xpath-31/#id-xq-evaluation-context-components)
and [XSLT focus](https://www.w3.org/TR/xslt-30/#focus). The bundle's XSLT host
needs to supply the position and size required by `xsl:for-each` or
`xsl:apply-templates` bodies. Expression-local focus must continue to override
and restore the outer focus correctly.

**Approved and implemented:** the user chose the shared XPath invocation
extension on 2026-09-18. `XPathDynamicContext.context_position` and
`context_size` are optional `u64` fields. Both must be supplied together with
a context item and satisfy `1 <= position <= size`; invalid requests produce
`cem.xpath.focus_invalid` with the owning expression's source location.
Omitting both preserves the existing singleton or absent-focus behavior.
The size is metadata and does not materialize or allocate a sequence.

The acceptance fixtures now pass explicit host positions and sizes to the
same source/reloaded XSLT programs, preserving imported CEM owners and source
diagnostics. XPath's own fixtures cover invalid/partial/absent focus, exact
64-bit coordinates, nested focus restoration, inline-function isolation,
limits, cancellation and unchanged binary bytes across runtime calls. Existing
inline functions still have absent focus; capturing values explicitly is
supported. Progressive streaming and unknown size remain a later phase.

Implementation verification: 86 XPath unit tests, 42 focused native cases and
all 98 adapter tests pass. The shared WASM rebuild, 17 artifact checks and 125
function-companion checks pass; native lint passes with existing warnings.

Focus is runtime invocation state and does not enter compiled-program bytes.
The native hook, focus extension and compiled-bundle delivery are approved;
do not request these decisions again. The bundle implementation below binds
that focus explicitly. Stylesheet loop lowering and the deployable XSLT viewer
remain subsequent work.

### Compiled bundle implementation (2026-09-18)

[`cem_ql::xslt`](../packages/cem_ql/src/xslt.rs) implements the
[versioned bundle contract](xslt-bundle.md): generated CEMT plus independently
hashed XPath programs, an ordered import/include closure, original stylesheet
ownership, scoped source-map validation, and explicit focus/variable arguments.
Loading uses the member codecs directly and creates a local capability
registry. It never parses source or fetches imports. Bundle and member hashes,
runtime versions, ownership and bounds must pass before a handle is retained.

The shared WASM module exposes explicit import/render/dispose operations.
Rendering accepts declared scalar parameters and retained CEM document handles.
The same program consumes XML, JSON, YAML and CSV after shared import; no
runtime format branches or JavaScript document records are involved. Stale
handles and ordinary-data attempts to install callbacks are rejected.

Verification: nine native bundle cases and 42 native/WASM checks pass through
`cem_ml_schema_package_xslt_v1:verify:compiled-bundles` and the native unit
tests. The contract covers all three focus modes, namespace-qualified
variables, original diagnostics, retention, shared cancellation and budgets.

This completes delivery infrastructure using a manually composed generated
CEMT fixture. XSLT-VIEW-LOWER subsequently preserves stylesheet
instructions as reusable runtime CEMT and connects their typed XPath slots to
the bundle. Import precedence, grouping and sorting were subsequently
implemented; viewer output and the gallery case remain tracked separately.

The [bounded runtime compiler](xslt-runtime-lowering.md) now compiles recursive
named/matched templates, explicit parameters, modes, exact priorities and
import/include precedence into that bundle. It preserves native focus,
lexical scope, original XPath source owners and atomic failure output. The
transform/CLI adapter executes this namespace-correct XSLT 3.0 profile; legacy
execution and implicit parameter/version/namespace shortcuts are retired.
Standalone legacy conversion commands remain separate. Native-produced
closures use the same explicit WASM loading lifecycle without fetching source.
The grouping slice now lowers non-composite `group-by` through typed native
XPath queries, including multiple keys, numeric promotion, first-seen headings
and preserved native members. The approved shared XPath group context supplies
the standard group functions across template calls and clears inside functions.
The native fixtures run over all four import formats and portable bundles.
Stable multi-key sorting now uses compiler-owned typed XPath over retained
native values, including common numeric promotion, dynamic controls and group
sort context. Its WASM gate exposed cumulative depth charging in the shared
evaluator; the approved correction now measures active nesting while preserving
the total-call budget. Standard data parsing now delegates to shared CEM import;
ordered QName catches use structured errors and preserve native nodes, focus
and lexical scope. The next task is XSLT-VIEW-OUTPUT; the complete viewer
profile remains pending. See the
[bounded runtime profile](xslt-runtime-lowering.md#standard-parsing-and-recovery).

## Acceptance and implementation order

1. Record the checklist and run minimal native data-model probes. If blocked,
   document the smallest missing reusable capability and request approval.
2. Add failing native fixtures for each compatibility feature, then implement
   and verify it without modifying unrelated language/runtime contracts.
3. Author `data-table-view.xslt` and a companion imported-aspect stylesheet.
   Compare semantic DOM, values and ordering against direct CEMT rendering
   through the real CLI conversion path; do not compare incidental patch IDs.
4. Add separate base-viewer and aspect gallery cases. Exercise all four formats
   in native parity tests, including namespaces, quoted CSV, nested collections,
   later columns, empty/missing/null values, invalid input and resource limits.
5. Verify source edits/reset, text/numeric sorting, selection, aspects, form
   drafts, focus preservation and independent instances in browser tests. Run
   standalone/source-loaded checks, lint/typecheck and compact layout checks.

A fixed-input HTML match does not establish reusable or interactive parity.
Do not publish a nonworking stylesheet or mark downstream checklist items
complete while an essential capability awaits approval.
