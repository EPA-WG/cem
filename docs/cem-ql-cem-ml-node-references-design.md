# CEM-QL / CEM-ML AST node references design

Status: accepted design, promoted on 2026-10-03 and revised by user adoption on 2026-10-04. Adopted rules are normative; deferred details are recorded in roadmap and todo action items. Implementation is separate and is not claimed by this document.
Date: 2026-10-04.


## Adoption summary

The following rules govern implementation of the accepted portion:

- Reference is an AST node type designating node(s) through CEM-QL; targets are not copied or inserted.
- `#` has prefix-unary precedence 9, with existing tighter postfix/type/dot operations and right-to-left prefix nesting.
- `{#...}` and XML `<cem:expr>#...</cem:expr>` retain semantically equivalent typed reference nodes. No separate `cem:reference` vocabulary is introduced.
- The reference model uses existing contextual scope/binding associations and source-position semantics. Context closure resolves AST-specific context relationships; loading does not automatically execute reference selection queries.
- Any AST node kind may be a target. Self-reference and cycles are permitted as graph edges, without implicit recursive evaluation.
- Existing AST identity and graph serialization distinguish reference occurrences and preserve ordered target edges. No global ID service or persistent identity across reparses is required.
- `#` accepts node-valued results and performs no implicit string lookup or expression execution. Referencing an attribute node is distinct from interpreting its scalar value.
- Consumer-specific interpretation and AST-change evaluation/update mechanics remain outside scope.
- Scope properties provide implicit references: syntax implies the scope root and matching property, with inherited defaults and enclosed child overrides. The exact child override syntax is deferred.
- The runtime supplies the evaluation context and its available root at the applicable document lifecycle stage. No authored root ID or mandatory stored context ID is required.
- Evaluation results need not be persisted on the authored reference. A transformation CLI or `cem-element` can supply `datadom` when execution reaches the consumer stage.
- Reference-chain resolution occurs during consumer evaluation. The suggested default is deep resolution bounded by effective scope policy, a traversal limit, and cycle detection; unresolved links follow mandatory, warning, or ignore rules from the applicable scope schema.
- IDs are scoped within a CEM assembly. URL fragment access to externally exposed document parts belongs to the resource/consumer boundary, not CEM-QL `#`.

This design does not prescribe a uniform storage layout, public type spelling,
target-access API, or new comparison operator. Concrete query and transport
details are actionable follow-ups. The former reference-preservation alternative
is superseded by the distinction between retaining reference edges and consumer
resolution of those edges.

## Purpose

Introduce a reference as a CEM AST node type. A reference node designates another AST node or an ordered sequence of AST nodes through a CEM-QL expression. It preserves the relationship without copying the referenced nodes into the reference's location.

The reference is a general document-model concept. It is not limited to templates, elements, attributes, `datadom`, or a particular query root. A reference's targets and available context follow the ordinary CEM-QL evaluation contract.

This design covers reference syntax, the reference AST node, implicit scope
references and defaults, target selection through CEM-QL, identity, lexical
binding resolution, and shared scope-schema rules for reference-chain
evaluation. The runtime supplies context and execution time. Consumer-specific
interpretation, including element-to-ID extraction, remains outside CEM-ML.
AST loading retains selecting expressions without automatically evaluating them.
Re-evaluation and retained result lifetimes belong to the consuming runtime.

## Settled direction

- `#` is the CEM-QL reference marker.
- A reference is represented as an AST node, not merely a specially formatted string or a browser element handle.
- The target is another node or nodes selected through CEM-QL.
- The attribute expression spelling discussed so far remains valid as a surface example:

```cem
@commandfor={#datadom.attributes.commandfor}
```

This example does not make `datadom` the universal reference scope, specify HTML command semantics, or prescribe how the attribute's value becomes a query target. Those details belong to the applicable expression/context or consumer contracts.

## Relationship to existing concepts

[AC-F-5](cem-ml-ac.md) requires references without cloning referenced content. The existing parser document contains an ID table and unresolved slots; those mechanisms are context, not the definition of this reference node. This design does not infer singleton ID lookup from them.

The [syntax reference](cem-ml-syntax.md) assigns attribute expression spans and explicit `$` expression nodes to CEM-QL. The `#` marker belongs to expression semantics; CEM-ML must preserve its result as a reference AST node wherever the language permits that node.

The [reference vocabulary design](cem-ml-reference-vocabulary-design.md) describes schema-owned validation constraints and normalization. It is separate from this general AST reference type. Its registry and host-language constraints neither limit references to those domains nor define the target selection semantics of `#`.

## Scope boundary

Included:

- A distinct reference AST node kind.
- A CEM-QL expression that selects or denotes the target node(s).
- The expression's source provenance and lexical/scope semantics, with runtime-supplied evaluation context.
- Context-specific binding resolution and retention of forward selection expressions during parsing/finalization; no implicit query execution.
- Target identity, ordered multi-target results, and reference diagnostics.
- Implicit scope defaults, enclosed child overrides, and explicit scope crossings.
- Shared reference-chain evaluation principles and scope-schema treatment of unresolved links.

Excluded:

- How references are used by CEMT, cem-element, validators, exporters, applications, or any other consumer.
- Template import, external-document import, and import-related CEM-ML ID semantics.
- A new singleton ID lookup mechanism or an implicit ID interpretation of `#`.
- Browser DOM lookup, live DOM handles, projected IDs, native attribute wiring, focus, visibility, and component endpoints.
- AST mutation tracking, reactive query evaluation, subscriptions, and updating references after their initial resolution.

A consumer may impose its own scope or interpretation. That must not be promoted into a restriction on the general AST reference node.

## Expression and surface semantics

Conceptually:

```text
#(CEM-QL expression)
    -> retained reference AST node
    -> target node sequence when explicitly evaluated with runtime context
```

`#expression` marks the expression's result as a reference relationship rather than requesting that the selected nodes be copied or emitted in place.

Examples, subject to final grammar:

```text
#element
#datadom.attributes.commandfor
#(expression-selecting-several-nodes)
```

`#element` uses the meaning of `element` under CEM-QL name/binding resolution. It does not introduce a separate rule that searches for `id="element"`. Paths, variables, navigation, selection, and supplied contexts retain their ordinary CEM-QL meanings.

The intended reading of `#datadom.attributes.commandfor` is `#(datadom.attributes.commandfor)`. The adopted precedence is specified below.


The standalone CEM-ML reference surface and its XML-convention parity form are adopted below. The attribute expression is not the only location where references can occur.

### Standalone reference surface (adopted)

```cem
{section | {#nodes}}
```

`{#nodes}` is a CEM-QL reference-expression entry point, not a structural node named `#nodes`. The leading `#` remains the reference operator within the expression. The adopted precedence rules apply to the complete expression inside this entry point.

The expression constructs one reference AST node targeting the ordered node sequence selected by `nodes`. The targets are not inserted, copied, or converted to text at that content position. Parsing preserves the reference expression and resolves its contextual bindings at context closure. Target selection is evaluated only when explicitly requested.

The XML-convention equivalent is:

```xml
<section>
    <cem:expr>#nodes</cem:expr>
</section>
```

Semantic parity is required, not assumed from the existing implementation. Both surfaces must preserve the same reference AST kind and produce equivalent typed targets, ordering, multiplicity, and outcomes when explicitly evaluated with equivalent contexts. Each surface retains its own source provenance. XML ingestion must not stringify the reference result or copy its target subtree.

The structural/expression entry points are therefore:

- `{name ...}`: a structural node.
- `{$ ...}`: a general CEM-QL expression.
- `{#...}`: a CEM-QL reference expression, retaining its leading reference operator.

A separate `{cem:reference @select=nodes}` vocabulary is not introduced. It adds no required semantics to the adopted surface. Its potential introduction for a future concrete metadata requirement would be a separate decision; it is not needed for this design.

## Adopted precedence and node-valued operands for `#`

The precedence rules below were adopted on 2026-10-03. The 2026-10-04 revision
distinguishes reference construction from consumer resolution of reference
chains. Public type and target-access spelling remain deferred. Precedence
extends the CEM-QL [Pratt precedence table](cem-ql-stack-design-impl.md): unary 9,
type forms 10, dot/pipeline 11, and calls/indexing 12. Design adoption does not
assert implementation coverage.

### Precedence (adopted)

Treat `#` as a prefix unary operator at the existing unary level (9). Calls, indexing, member access, dot/pipeline steps, and the existing type-postfix forms bind more tightly. Arithmetic, set operations, comparisons, booleans, and coalescing bind less tightly. Repeated prefix operators associate from right to left.

| Expression | Adopted grouping | Consequence |
|---|---|---|
| `#datadom.attributes.commandfor` | `#(datadom.attributes.commandfor)` | The full member expression is the operand |
| `#nodes[0]` | `#(nodes[0])` | Select one node before constructing the reference |
| `#nodes.filter(predicate)` | `#(nodes.filter(predicate))` | The pipeline selects targets before reference construction |
| `#a / b` | `(#a) / b` | Slash retains its existing lower precedence; it does not traverse the referenced targets implicitly |
| `#(a / b)` | Explicit grouped selection | Evaluate the slash expression first, then reference its node result |
| `#a \| b` | `(#a) \| b` | Set union is outside reference construction; its ordinary typed rules still apply |
| `#(a \| b)` | Explicit grouped selection | Construct a reference to the union's node result |
| `#a ?? b` | `(#a) ?? b` | Coalescing applies to the reference result, not its target count |
| `#(a ?? b)` | Explicit grouped selection | Select a fallback operand before reference construction |
| `##a` | `#(#a)` | The operand can itself be a reference node; grouping does not resolve the chain |

Parentheses expose the boundary between selecting nodes and constructing a reference. Syntax must not change precedence based on inferred operand types. In particular, `#` should not silently swallow a lower-precedence traversal or union merely because its result might contain nodes.

To operate on the reference value itself, use a grouped result, such as `(#nodes).operation(...)`; whether that operation exists is determined by ordinary method/type rules. Ungrouped `#nodes.operation(...)` applies the operation to `nodes` first.

### Query type and AST node kind

A reference is an AST node kind. CEM-QL also needs to distinguish its typed reference value from an ordinary selected target node. Conceptual notation `Ref<T>` means a reference node whose target sequence contains nodes of type `T`; this notation does not prescribe the public type syntax or Rust representation.

Node-valued operand rules:

| Evaluated operand | Result |
|---|---|
| One AST node of type `T` | A reference relationship to that node |
| A sequence of AST nodes | A reference relationship to that ordered target sequence |
| Empty node sequence | One resolved reference node with zero targets |
| An existing reference node | Eligible as a target; consumer evaluation governs whether its chain is followed |
| Scalar, string, record, or collection with non-node items | Type error unless an independently declared expression conversion first supplies nodes |
| A sequence containing ordinary and reference nodes | Retain the selected node order and multiplicity; construction does not flatten reference targets |

An operand statically known to be incompatible is rejected by type checking. Dynamically typed operands receive the same check when initially resolved. There is no conversion of strings to ID lookups, target text, or browser handles.

A reference is not a subtype of its target type: `Ref<Element>` is not `Element`. It can be classified as an AST reference node for generic node-model operations, but this does not grant element attributes, child axes, or target traversal. Target access requires an explicit operation under the query contract; spelling that operation is not settled here.

### Existing reference as operand

Reference nodes may designate other reference nodes. CEM-ML retains those
relationships without choosing an implicit flattening or preservation strategy
for chain resolution. Constructing an edge and following it are different
operations. Each authored occurrence retains its source provenance; targets
retain their ownership and structural position. The consumer applies the
[reference-chain resolution policy](#reference-chains-follow-scope-schema-policy)
when evaluation happens, without rewriting the authored graph.

### Empty results, truth, and equality

A zero-target reference is still a reference node; it is not automatically null, an empty query value, or false. `#a ?? b` therefore must not become a hidden fallback for zero targets. Query truth conversion must not follow targets implicitly.

Construction does not define a new equality operator. Reference-node identity and resolved-target-sequence equality are different comparisons. Ordinary identity can compare the reference nodes; any comparison of their targets must be explicit and must preserve order and multiplicity. The public comparison surface is deferred in [roadmap.md](../roadmap.md#deferred-cem-reference-query-and-transport-contracts).

## Reference AST node

The retained reference and its runtime evaluation have distinct responsibilities:

| Field | Meaning |
|---|---|
| Node kind | Reference, distinct from element, text, and ordinary scalar nodes |
| Reference expression | The parsed CEM-QL expression designating target node(s) |
| Lexical/scope information | Document semantics needed to interpret syntax at its source position |
| Evaluation context | Supplied and kept by the runtime, with its context root directly available; not a required stored ID on the reference |
| Target identities | Ordered identities available at consumption; retaining them on the authored reference is optional |
| Resolution state | Runtime evaluation outcome; not a required mutable field on each authored reference |
| Provenance | Authored syntax, source range, and relevant expression diagnostics |

This is a semantic outline, not an implementation struct. A runtime may retain
evaluation results and annotations where its lifecycle requires them, or
evaluate and consume the nodes in one execution without publishing a target
list back onto the source AST.

A reference node has its own AST identity and source location. Its targets retain their existing identity, ownership, and structural position. They do not become children of the reference node merely because it references them.

All AST node kinds are eligible targets, including document, element, attribute, text, and reference nodes. Consumer restrictions do not narrow the general reference node type.

## Existing representation and the remaining AST question

This section records inspected implementation surfaces and recommendations for discussion. It does not adopt a storage layout or authorize implementation.

### Existing layers

| Layer | Existing representation | What it does not currently establish |
|---|---|---|
| Parser document | `CemDocument` owns an arena of `CemAstNode`, addressed by `AstNodeId`; the initial reference variant retains expression source, containing node, and optional targets | The containing-node handle is not a complete runtime evaluation context; optional target storage is not a required source contract |
| Parser reference slot | `NameSlot` contains an owner scope, target name, optional resolved node ID, and source map | A general CEM-QL expression, multiple targets, and the full reference-resolution contract |
| CEM-QL syntax | `Expression` represents names, paths, pipelines, operators, and other expressions with byte ranges | Retention of that expression on a parser reference node |
| CEM-QL context | `QueryContextScope` and query node views carry evaluation/access context; views expose representation and node identity | A portable capture of every evaluation binding on a reference |
| Native reference value | `CemReference<T>` retains an ordered target sequence; `ReferenceView` exposes it as a query node with `kind`, `targets`, and runtime identity | Original selecting expression, evaluation-context capture, or pending-resolution status |
| Constructed native content | `CemValueNode::Reference` contains a reference value and source map | A parse-time expression or pending state |
| Portable value graph | `CemValueRecord` supports `kind="reference"`, ordered `targets` indices, source information, and provenance | A serialized selecting query/context or an explicit parse-resolution state |

Relevant source files: [parser AST](../packages/cem_ml/src/parser.rs), [parser document](../packages/cem_ml/src/parser/document.rs), [native values](../packages/cem_ml/src/value.rs), [portable value graph](../packages/cem_ml/src/value/artifact.rs), [CEM-QL expressions](../packages/cem_ql/src/parser.rs), and [query reference view](../packages/cem_ql/src/eval/values.rs).

These layers are implementation context, not a mandatory common storage
structure or evidence that every adopted lifecycle and schema rule is
implemented. Remaining representation work is recorded in
[todo.md](todo.md#ast-node-reference-implementation).

### Context-specific references and common resolution (adopted)

Context-specific constructs already express reference relationships. The adopted general reference model uses similar resolution semantics rather than inventing a second independent context registry or lookup lifecycle.

Inspected examples:

| Construct | Existing representation | Relevant resolution behavior |
|---|---|---|
| Schema selected for a scope | `SchemaSource::Select`, `InlineRef`, and `SchemaScopeFrame` | Retains designation plus owning scope; inline declarations resolve through inherited/local scope bindings |
| Namespace-qualified name | `ResolvedQName::binding_id`, `NamespaceBinding`, and `NsContext` | Retains the binding effective at the source position; later rebinding does not retroactively change earlier resolved names |
| CEM-QL variable/function/type/template binding | `BindingEntry`, `BindingId`, and `BindingSet` | A scoped name is linked to a declaration identity; name spelling and declaration identity are distinct |
| Embedded expression | `EmbeddedExpressionSlot` | Retains host location, expression location, expected type, and declared evaluation phase |
| Forward name slot | `NameSlot` and the slot-resolution contract | Retains designation, owner scope, target identity when available, and original source provenance |

Relevant sources: [schema scoping](../packages/cem_ml/src/schema/scoping.rs), [namespace context](../packages/cem_ml/src/schema/namespace.rs), [query bindings](../packages/cem_ql/src/resolve.rs), and [embedded expressions](../packages/cem_ql/src/embedded.rs).

These are contextual references in the semantic model. Specialized frames,
bindings, and slots can retain them. Some declaration identities refer to
context records rather than parser element nodes; implementations must not
equate all numeric handles with `AstNodeId`. Coherence requires common scope,
override, and consumption boundaries, not uniform storage or identical syntax.

The common pattern is:

```text
reference occurrence + designation/expression
    -> context/binding association effective at that occurrence
    -> initial resolution against the available declarations/nodes
    -> typed target identity or a source-addressable resolution outcome
```

For a general `#` reference, CEM-QL supplies the designation/selection. For a namespace or schema-context reference, the existing construct supplies its own designation and target-kind constraints. Sharing the resolution model does not require giving each construct the same surface syntax or accepting every target kind.

Consequences for this design:

- Retain the applicable lexical and scope semantics. Runtime context is directly supplied; do not require authored root IDs, mandatory stored context IDs, or copied evaluator environments.
- Preserve the context effective at the occurrence's source position. Deferring initial target resolution until parse finalization must not silently use bindings that were introduced or rebound later at the same source scope.
- Reuse the existing scope chain and shadowing rules where the applicable context defines them. This does not introduce a template-only scope or singleton ID lookup.
- Distinguish binding resolution from query evaluation: fixing which declaration a name denotes does not necessarily mean that a forward target node sequence is already available.
- Use a shared initial-resolution outcome model, while keeping per-construct target kinds, cardinality, dependency readiness, and diagnostic policy explicit.
- A general expression selecting several nodes is a valid multi-target relationship; a namespace binding or active schema selection can retain its own single-target constraint.
- Finalize pending relationships at their declared dependency/context completion boundary. Do not force all contextual references to wait until document end when parsing needs them earlier.
- Preserve the established no-retroactive-rebinding principle for resolved contextual bindings. AST mutation and reference updates after finalization remain outside scope.

Parser-side lexical capture and explicit consumer scope handoff preserve these
associations alongside the specialized records. Native reference consumption by
the schema/namespace scope properties themselves still requires per-property
target-kind, cardinality and readiness contracts, tracked in
[todo.md](todo.md#ast-node-reference-implementation). No replacement of every
frame or binding table is mandated.

### Scope-property consumption audit (2026-10-06)

Existing namespace records retain URI bindings, while `SchemaSource::Select`
retains an expression. Lexical capture and caller-completed scope handoff retain
these associations; native schema/namespace property selection is still unimplemented.
The deferred enclosed child override syntax remains separate.

The [stack-design source-attribute and identifier-resolution tables](cem-ml-stack-design.md#131-document-side-schema-scoping)
now distinguish lexical lookup from consumer cardinality. Adopted on 2026-10-06:
resolve names using the captured lexical scope, consume reference chains under
existing bounds, then require exactly one completed schema target. Innermost
lexical name shadowing remains applicable; it does not rank an arbitrary query
sequence. Zero/multiple completed targets and pending resolution cannot establish
an effective schema scope. Native selector evaluation is not implemented yet.

Target shape was adopted on 2026-10-06: accept a schema-language declaration
with expanded name `{https://cem.dev/ns/schema/1}schema` directly, or a core
`cem:schema` wrapper with a nonempty literal core `cem:name` and exactly one
direct schema-language declaration child. Prefix spelling does not establish
target kind. A nested declaration inside an unrelated subtree does not qualify.
Wrapper bodies are not interpreted implicitly as schema fragments.

`schema::scope_references::admit_schema_scope_target` implements this admission
boundary over `SchemaDeclarationNode` handles. It returns both the selected node
and exact declaration with their original arena owner. An explicit lookup supplies
owner-checked expanded names from source-position metadata; lexical CEM prefixes
are not namespace URIs. Missing metadata returns `NameNotReady`. Admission does
not resolve reference chains, compile the declaration, establish a scope, authorize
crossings or declare dependencies ready. Descendant reference nodes remain authored.

Adopted on 2026-10-06: an explicit pending or invalid schema override keeps its
governed region incomplete until a usable schema is available. Validation cannot
fall back to the inherited schema for that region. This is distinct from keeping
the last complete package active during package replacement. Namespace target
contracts, consumer name handoff and lifecycle scope activation remain subsequent
consumer work.

`LexicallyScopedDocument::expanded_name(owner, node)` now supplies immutable
namespace metadata to the admission lookup. CEM name events use the namespace
context effective at that authored position and associate the scalar result with
original builder IDs, including attributes completed later. Unbound prefixes have
no resolved name; later declarations cannot fill an earlier occurrence. Folded
reference nodes retain their expression snapshots without becoming named elements.
XML capture retains its shared importer's resolved element and attribute names,
including XML's distinct default-namespace rules. Allocation identity gates lookup;
neither path rewrites the original AST or establishes schema readiness.
`CemQlSchemaDeclarationHost::attach_captured_names` now installs scalar metadata
only for an already-registered original owner. Repeat attachment is idempotent;
foreign owners are rejected. Captured lexical-scope handoff also installs names
once preflight succeeds. This does not rewrite the registered tree's query view.

`schema::scope_references::compile_schema_scope_target` now compiles the exact
admitted declaration without rediscovering a schema elsewhere in its arena.
Collection reference evaluation, authored element bases and native attribute-type
readiness tracking all use that declaration ID. Original target handles, lexical
aliases and occurrence source maps remain attached to dependency outcomes; other
schema roots cannot supply declarations or reference sites implicitly. The legacy
whole-document entry point retains its first-schema behavior. Compilation returns
an inspectable model; lifecycle activation must still require complete dependencies
and no hard compilation diagnostics. It neither activates a region nor supplies an
inherited fallback.
The decision, implementation tasks and verification scenarios are recorded in
[todo.md](todo.md#ast-node-reference-implementation).

`CemQlSchemaDeclarationHost::prepare_schema_scope` now provides an explicit
consumer preparation stage for one retained native reference. It resolves the
selection under the existing request/destination bounds and directed grants,
requires one completed target, admits it using captured names, then compiles its
exact declaration. An unavailable selected-declaration context remains pending
even when the requesting context completed selection. Selection and compilation
retain their existing bounded operation contracts; this adds no aggregate budget
or implicit context inheritance.

The result retains selection issues, original target handles and any candidate
model for inspection. Its `is_ready()` requires complete successful selection,
valid singleton admission, available declaration context, complete model dependencies
and no hard compilation diagnostics. Pending native datatype dependencies and
invalid regex facets therefore cannot establish readiness. Repeating preparation
uses current inputs with unchanged captured names; it does not save targets on
source references. Recognizing scope-property syntax, assigning governed regions,
installing effective policies remain separate lifecycle wiring tasks in `todo.md`.
The explicit region validator below blocks inherited validation for caller-declared
incomplete regions.

`validate_structural_input_roots_references` and the CEM-QL host's
`validate_input_roots` now validate an explicit forest of original structural
roots under a supplied consuming model. The whole-document entry point delegates
to the same walk. Selected roots retain original node owners, authored trivia,
reference outcomes, bounds and source provenance; unrelated source siblings are
neither validated nor evaluated. Invalid kinds, missing handles and duplicate root
requests reject before input evaluation. An empty valid forest is complete only
when the supplied model is ready; unavailable regions cannot be represented by
substituting empty roots.

The child boundary ownership rule was adopted on 2026-10-06: the enclosing
schema retains the host's attributes and direct child relationship/sequence
contracts; the child schema validates descendants. Nested overrides restore the
preceding consuming model on exit, including for following siblings. The enclosing
schema's wildcard permission does not bypass the child model's declaration checks.

`validate_structural_input_regions_references` accepts explicit original element
hosts and optional child models. An unavailable or invalid model blocks its body,
keeps host attribute checks active and leaves overall validation incomplete.
Deferred child-sequence/count checks cannot treat the blocked body as an empty
sequence. Empty enclosing models still visit explicit boundaries. Invalid or
duplicate host descriptors reject before evaluation.

Each retained placement carries its consuming model, including placements reached
through references. A selected subtree's child boundaries and native attribute
chains share that selection's existing traversal and active reference identities;
a boundary does not reset work or destination limits. Source arenas, authored
references and lexical associations remain unchanged.

The CEM-QL host's `validate_input_regions` accepts `SchemaInputRegion` descriptors
with preparation snapshots. Only `is_ready()` preparations supply child models.
Entered blocked regions retain selection/compilation diagnostics, and invalid
singleton/admission outcomes have host-attributed override diagnostics. Unrelated
regions outside the requested forest do not supply diagnostics. Callers repeat
preparation when lifecycle inputs change. This API does not recognize authored
scope-property syntax, install runtime contexts/policies or run child-specific
behavior hooks; those lifecycle integration tasks remain in `todo.md`.

`decode_schema_host_control` now recognizes the established `schema-src` and
`schema-select` host attributes using captured expanded names: canonical
unqualified names and core namespace aliases are accepted; foreign namespace
lookalikes remain ordinary data. It retains the original host, attribute and
native payload handles, checks source/selector exclusivity and value shape,
and distinguishes literal URIs, literal selector expressions and native selector
slots. Decoding performs no query execution or scope installation.

`CemQlSchemaDeclarationHost::prepare_schema_host_control` feeds literal selectors
into the same bounded selection/admission/compilation stage as native references
and native expression slots. A literal selector is a consumer-owned implicit
reference occurrence on its original attribute handle, never a fabricated AST
reference. Returned reference chains retain request/destination budgets and
directed grants. Selection must still complete with one valid original schema
target, an available declaration context and a ready model. Each invocation uses
caller-supplied current scope contexts; captured names and authored values stay
unchanged. URI controls retain their authored URI with no selection preparation:
loading belongs to the external resource lifecycle, so they are not ready here.

Literal selector query errors retain the original attribute source and query
frames. CEM literal attributes currently supply a name source map rather than a
decoded-value character map, so these errors anchor to that real handle without
claiming precise positions inside the value. Ordinary attributes do not become
implicit references outside this explicit consumer call.

Full governed-host validation still needs a decision about the control attributes
themselves: whether a shared schema-control contract validates them separately
from ordinary enclosing-schema host attributes, or each application schema must
declare and type them. The shared-contract option is recommended and tracked in
`todo.md`. Wrapping/sibling/prelude controls, runtime policy installation and
per-region behavior dispatch also remain open. No deferred enclosed override
syntax is selected by this host-attribute preparation stage.

### Implicit scope references and defaults

Scope properties provide an indirect form of reference. Syntax and its
applicable schema imply the scope root and matching property. Authors can
establish a default once for the governed region without marking the root
again or repeating an explicit reference at every use site.

Child scopes inherit the effective property under its existing scope rules.
A child override uses an enclosed scope reference whose effect is bounded by
that child. Leaving the child restores the enclosing effective relationship.
Source-position and shadowing rules remain in force. Explicit scope crossings
require a declared relationship; a document-wide ID scan cannot replace it.

Lexical binding snapshots and reference relationship boundaries are distinct.
A namespace/schema binding change preserves the caller-defined relationship
boundary unless an explicit boundary is declared. It may establish a different
compilation environment and effective schema policy without introducing a new
crossing grant requirement. Policies and traversal budgets still constrain the
consumed reference path; lexical changes cannot reset request accounting.

The exact enclosed child override syntax is deferred to
[roadmap.md](../roadmap.md#deferred-cem-reference-syntax-decision). The accepted
default and override semantics do not depend on choosing a new delimiter,
root marker, or context identifier. Existing schema and namespace forms remain
in use until that syntax decision is specified.

### Four different pieces of information

1. **Query expression:** what selects the targets. Preserve the parsed expression or a stable expression-artifact link and its source map; keeping only its rendered string loses type/source information. This is the expression describing construction, not a stored procedure that will be automatically rerun.
2. **Evaluation context:** the bindings/current item/document access supplied by the runtime at its chosen lifecycle stage. The context root is already available; an authored ID or mandatory stored context ID is unnecessary. Document lexical bindings retain source-position meaning, while runtime inputs such as `datadom` arrive for the particular execution. A saved containing-node handle alone is not a complete evaluator environment.
3. **Target identities:** the resulting ordered node identities, distinct from the expression and from the reference node's own identity. Use owning-document/graph context with node handles. Native portable graph indices and parser arena indices are different representation-local handles; neither may be treated as a universal singleton ID.
4. **Resolution state:** distinguish a retained unevaluated expression from outcomes of an explicitly requested evaluation: pending, resolved, unresolved, or invalid. An empty target sequence cannot distinguish those conditions.

### Conceptual source and evaluation forms

```text
ReferenceNode
    own node identity
    expression artifact + source provenance
    applicable lexical/scope semantics

Runtime evaluation with supplied context
    Pending(requested evaluation)
    Resolved(ordered target identities)
    Unresolved(reason + scope-schema policy)
    Invalid(reason + diagnostics)
```

This is conceptual notation, not Rust API syntax or required source-node
fields. An unevaluated source expression is distinct from a requested pending
evaluation and from a resolved empty target sequence.

Expression and provenance can remain on the source reference or its artifact.
A retained evaluation result should have one authoritative target sequence for
that execution. Different executions may have different contexts and results;
they need not mutate one shared target list on the authored occurrence.

### Identity and persistence limits

`CemReference<T>` currently uses shared container identity (`Arc`) for runtime reference identity. Cloning retains that identity without copying targets. That address-derived identity is not a portable identifier across serialization or processes. The portable graph carries its own reference record and graph-local target edges.

A serialized resolved graph needs node distinction, target edges, and
provenance. A source reference can instead be exported for later evaluation
with runtime-supplied context. Transport of pending runtime outcomes is a
separate explicit contract, not a requirement to serialize a live context.
Graph and native output artifact compatibility is deferred in
[roadmap.md](../roadmap.md#deferred-cem-reference-query-and-transport-contracts).

These representation choices concern construction and initial resolution only. Keeping expression provenance must not imply AST-change subscriptions, automatic re-evaluation, or any consumer-specific use of the reference.

## Target selection and context

CEM-QL supplies selection and context. The reference node does not introduce an independent template registry, a new DOM scope, a special ancestor-search rule, or automatic document-global lookup.

A supplied context can contain nodes or references made available by the normal query contract. Whether a query may traverse the current document, a nested structure, or an explicitly supplied document is governed by that contract, not by a reference-specific template restriction.

Construction and selection obey these rules:

- A node result can establish a target identity.
- A node sequence can establish an ordered target sequence.
- Non-node results must have an explicitly defined reference interpretation or produce an invalid-target diagnostic; strings must not silently become ID searches.
- A selected reference node is an eligible target. Following its chain belongs to explicit consumer evaluation and does not reinterpret its textual spelling.
- Multiple selected nodes are a legitimate result, not inherently ambiguity. No first-match selection is implicit in `#`.

If the query contract reports missing bindings or an ambiguous expression, the reference preserves those diagnostics. This design does not impose single-target cardinality on all references.

### Runtime context and scoped IDs

The runtime supplies and keeps the evaluation context, including its available
root. Evaluating a reference does not require assigning an ID to that root or
looking it up by ID. An author may define an ID when their document contract
needs one. Internal node handles distinguish graph nodes without becoming
authored IDs or public context addresses.

A CEM assembly can contain data of different kinds from different vendors.
IDs are meaningful inside their owning scopes. Equal ID strings in different
scopes do not establish the same target. Cross-scope access must be explicitly
defined through the applicable relationships and supplied context.

### URL fragments and external public parts

The `#id` convention belongs to URL fragment access to external documents that
publicly expose parts through IDs. The resource resolver or applicable
consumer resolves the URL and its public parts contract, then supplies retained
CEM nodes to evaluation. CEM-QL `#expression` does not handle URLs or ID lookup.

That external contract treats the document as flattened for access to its
exposed parts. It does not flatten the scopes of a general CEM assembly or
expose every vendor's internal IDs. Fragment-only values are URL references
only at a declared URL boundary that determines the addressed document.

## Identity and graph structure

Resolved targets are identified by their owning AST document and node identity, not only by lexical names, import IDs, or reference expression text. A bare arena index is meaningful only within its owning document.

Two reference nodes can designate the same target sequence while retaining different source locations and expressions. Equality of the reference nodes themselves must be distinguished from equality of their resolved target sequences.

The structural AST remains a tree or arena as defined by the existing model; references add non-owning graph edges. Following an edge is not structural child traversal. Retaining the graph does not recursively dereference its edges, insert target subtrees, or expand cycles.

Self-reference, cycles, and references to reference nodes are permitted as
non-owning graph relationships. Retaining them does not trigger evaluation or
expansion. A consumer requesting chain resolution follows the scope-schema
policy below, including a traversal limit and cycle detection. Circular
evaluation dependencies remain distinct from a retained cyclic graph.

Any portable representation of resolved references must retain adequate document/node identity. It must not serialize a live browser pointer or duplicate the target subtree. Detailed transport contracts are deferred in [roadmap.md](../roadmap.md#deferred-cem-reference-query-and-transport-contracts).

## Parsing and initial resolution

Reference expressions and nodes are retained during parsing. Closure of their owning semantic context is the definite boundary for context-specific binding resolution. Loading retains the expression without automatically executing target selection; explicit evaluation is separate.

| State | Meaning |
|---|---|
| `unevaluated` | The selecting expression is retained; no evaluation has been requested |
| `pending` | An explicitly requested evaluation awaits its declared inputs |
| `resolved` | Initial CEM-QL evaluation completed with a valid target node sequence |
| `unresolved` | An explicitly requested evaluation could not resolve its declared dependencies |
| `invalid` | Expression, context, or result violates the reference construction contract |

A resolved empty sequence is distinct from an unresolved reference. Zero selected nodes are not automatically an error. Whether a particular relationship requires one or more targets belongs to its consumer, outside this design.

A retained forward expression must not be mistaken for a failed lookup against a partial AST. The parser does not automatically run it after every node or at context closure. Its evaluator supplies the appropriate context when explicitly requesting evaluation.

If required context is not available, preserve the expression and its unresolved/deferred dependency. Do not fabricate a target or substitute an ID lookup. The applicable construct defines whether that retained state is permitted at parse finalization.

Runtime scheduling, re-evaluation, result retention, and invalidation remain
consumer responsibilities. The shared chain resolution and diagnostic policy
applies when that consumer requests evaluation.

## AST loading, context closure, and explicit evaluation (adopted)

Loading/parsing an AST does not by itself execute embedded reference selection expressions. Preserve the expression node, its provenance, and its applicable contextual associations.

Context closure remains the definite boundary for AST-specific context/binding resolution. It establishes the context associations needed to interpret the document. It is not a universal instruction to evaluate every embedded query and populate target sequences.

Distinguish:

1. Parsing and lexical resolution: retain syntax, source-position bindings, and scope relationships needed to interpret the document.
2. Runtime evaluation: the consumer supplies context at the applicable lifecycle stage, evaluates the selecting expression, and uses the resulting nodes.

The evaluation request can come from a CLI, transformation, or other evaluator. Its expression can already reside in the AST; expression origin and execution trigger are separate. A caller may explicitly request evaluation during loading, but loading alone does not imply it.

Forward AST target nodes need not exist when the reference expression is first parsed. The AST preserves the expression without prematurely executing it against partial input. An explicit evaluator decides when its required context is ready and which completed AST/context to supply. This design does not add an automatic scheduler or re-evaluation protocol.

Context/binding associations must retain the established source-position and shadowing semantics. Deferring evaluation does not make illegal forward lexical declarations valid or silently substitute later bindings. Context closure must not discard the associations needed to interpret the retained expression when the active parser frame is popped.

At parse finalization, an unevaluated expression is a valid retained expression, not automatically a failed target lookup. Parsing errors, illegal bindings, and required contextual-resolution failures remain diagnostics under the applicable construct. A successful evaluation returning zero nodes is different from not having requested evaluation.

For a template, a transformation CLI or `cem-element` supplies `datadom` during
transformation execution. Two instances can share the authored template while
each execution receives its own context. The result need not remain attached
to the source reference before consumption or be written back afterward.
Caching, subscriptions, retained results, and updates are optional runtime
lifecycle choices, not requirements of the authored reference.

A reference may target any AST node kind. Self-reference and cyclic target edges are allowed as graph structure. They do not require the parser or loader to follow those edges. A cycle of evaluation dependencies that prevents a requested evaluation from completing is a different issue from a valid cyclic target graph.

### Reference chains follow scope schema policy

Reference nodes may designate other reference nodes. CEM-ML retains the edges;
the consumer resolves their chain during evaluation. Constructing an edge and
following it are separate operations. Resolution does not implicitly flatten,
replace, or rewrite the authored graph. Parsing and structural inspection do
not automatically follow reference chains.

When a consumer requests resolution, the suggested default is deep resolution
within effective scope policy. The runtime supplies context and execution
time and gives the resulting nodes to the consuming operation.

Lexical scope depth and reference-chain depth are separate limits. Scope rules
define permitted crossings and applicable policies. Resolution also requires
a reference traversal depth or work limit from effective scope policy, plus
cycle detection: a long chain or cycle can stay inside one lexical scope.
A cycle or exhausted limit produces an incomplete unresolved outcome under
that link's schema disposition; even neutral or ignore stops the affected
traversal. Standard bounds are 128 reference expansions per path and 100000
visited nodes per request, with positive schema overrides.

Request-wide bounds remain active across crossings. Each destination scope
additionally constrains its subtree; its depth allowance starts at entry,
while work accounting is cumulative across repeated entries in the request.
Ancestor scope constraints also remain active. Request work exhaustion stops
the request; scope work exhaustion prunes that subtree while preserving
available enclosing siblings. Runtime scope keys are supplied by the consumer
and require no authored context/root IDs.

The applicable scope schema determines unresolved-link treatment:

| Schema requirement | Evaluation treatment |
| --- | --- |
| No effective rule, or explicit neutral disposition | Retain a neutral unresolved outcome for the consumer; emit no diagnostic and do not imply successful resolution. |
| Mandatory resolution | Report failure through the scope's existing diagnostic and error policy. |
| Warning | Report the unresolved link and retain its unresolved outcome for the consumer. |
| Ignore | Tolerate the unresolved link without a diagnostic; do not fabricate a target or remove the authored edge. |

Apply policy to each link under its effective scope schema. A resolved empty
selection is distinct from an unresolved link; schema cardinality determines
whether an empty result is acceptable. Consumer-specific use of the resolved
nodes, including element-to-ID extraction, remains outside CEM-ML.

The neutral fallback was adopted on 2026-10-04. An omitted child rule inherits
the enclosing disposition and its diagnostic definition. A child may explicitly
restore neutral behavior. Invalid expressions and scalar operands remain
invalid; pending evaluation and successful empty selection are separate states.
Disposition does not bypass cycle detection or traversal bounds.

The implementation declares `reference-unresolved-disposition` in the scope
schema with `@value` set to `neutral`, `mandatory`, `warning`, or `ignore`.
Mandatory and warning rules may name a diagnostic declared in that schema;
omitting `@diagnostic` uses the generic schema's corresponding diagnostic.
Mandatory diagnostic severity must be error or fatal, and warning severity
must be warning. Neutral and ignore rules do not name diagnostics. Duplicate
disposition/depth/work declarations in one schema scope are rejected; inherited
rules can be overridden in a child scope. This uses existing schema forms and
does not choose the deferred enclosed child scope-reference spelling.

## Identity and graph preservation (adopted)

Reuse existing AST node identity and graph serialization to distinguish reference occurrences and preserve target edges. This minimal identity direction is adopted; no new wire format, equality operator, global identity service, or persistent identity across independent reparses is adopted.

Reference-node identity distinguishes two reference occurrences even if their selecting expressions or selected target sequences are identical. The existing AST node-identity machinery can provide this; no separate global reference naming service is implied.

Target identity distinguishes the selected node itself from another node with equal content. Two identical elements remain different nodes. Within one AST arena, a node handle is sufficient. Where evaluation explicitly uses more than one document, pair the handle with its owning document/graph context to avoid collisions.

Target-sequence equality, if needed, compares target node identities position by position, preserving order and multiplicity. `[A, B]` differs from `[B, A]` and `[A, B, A]`. This does not make two distinct reference AST nodes identical. It also does not require a new public equality operator in this design.

Portability is conditional. If only source expressions are serialized, their future targets are obtained by an explicit evaluation and no resolved target identity transport is required. If a finalized/resolved graph is exported, its reference edges must reconnect to the correct exported nodes when loaded. Graph-local indices and normal AST graph remapping can meet that requirement; globally persistent UUIDs, URLs, source hashes, and identity across independent reparses are not required by this design.

The minimal need is therefore preservation of node distinction and graph edges. Query comparison and transport details are tracked in [roadmap.md](../roadmap.md#deferred-cem-reference-query-and-transport-contracts). They should not force evaluation during loading or expand scope into runtime persistence policy.

## Multiple targets

The reference's resolved targets preserve the CEM-QL result's order and multiplicity. Reference construction does not sort, deduplicate, or turn sequences into sets.

For example, selecting the sequence `[nodeA, nodeB, nodeA]` retains three target entries in that order unless the expression itself requests different sequence semantics.

Single-target requirements, uniqueness, serialization as an IDREF list, and handling a missing target in an application are consumer decisions. They are not properties implicitly imposed by the general reference node.

## Diagnostics

Reference diagnostics should preserve the expression and original source location. Relevant categories include:

- Invalid reference expression.
- Unavailable or invalid query context.
- Unsupported target result type.
- Required contextual binding resolution incomplete at context closure, or an explicitly requested evaluation unable to resolve its declared dependencies.
- Target identity that cannot be represented in the declared AST context.

Expression lookup/type diagnostics should retain their CEM-QL identity rather than becoming generic browser-target errors. A valid multi-node or empty result should not be mislabeled ambiguous or missing without an explicit constraint.

Unresolved-link outcomes follow the mandatory, warning, or ignore disposition
of the applicable scope schema. Ignoring a diagnostic retains the relationship
and does not invent a resolved target. This policy does not change the strict
operand type rules or turn a successful empty selection into an unresolved one.

## Attribute operands and explicit result interpretation (adopted)

The surface `{#datadom.attributes.commandfor}` is settled, but its supplied operand contract must be distinguished from its syntax. Strict node-valued construction and the attribute-node/value distinction are adopted. Any additional expression-valued attribute protocol remains separate and is not introduced here.

### Attribute node versus attribute value

All node kinds are eligible targets. If `datadom.attributes.commandfor` denotes an attribute AST node, `#` references that attribute node. It does not automatically reference a node named by the attribute's text. Reading the attribute's value is a distinct query operation.

If the expression denotes an already supplied node sequence or reference value, the reference operator can use that typed operand under the reference construction rules. If it denotes an ordinary scalar string, that string is not itself a target node.

Therefore the same surface does not promise native HTML command-target interpretation in every context. The available query view and supplied value type determine what is being referenced. This is especially important because an element node and its attribute node are both valid targets but have different identities.

### Ordinary strings

Strict construction is adopted: `#` must not silently convert a string into an ID lookup, scoped-name search, selector, or another expression to execute.

For example, a scalar `"details"` does not establish whether it is literal text, an identifier, query source, or an application command target. `#` alone cannot choose that interpretation. Under the strict rule it is an invalid target operand, not a reference waiting for an ID match.

An attribute string cannot become permission to access additional documents or an external browser tree. Any expression evaluation uses its explicitly supplied CEM-QL context.

### Explicit ways to supply targets

There are two useful contracts, independent of HTML command behavior:

1. Supply a node/reference-valued attribute through the typed AST/context model. The settled expression can then consume that typed value directly. If the intended targets are other nodes, supply those nodes/reference rather than only the attribute node containing text.
2. Explicitly declare that an attribute contains CEM-QL expression source, and parse/evaluate that source through the existing expression mechanism when evaluation is requested. Reference construction consumes the resulting nodes. Parsing a second expression and selecting its targets is not an implicit side effect of prefixing an arbitrary string with `#`.

The second contract must identify expression source, evaluation context, execution request, and result type. The design does not invent a new `eval`, `lookup`, or string-to-reference function, nor does it adopt an expression-valued attribute protocol merely because the example uses `commandfor`.

Supplying a typed reference and supplying an expression that can later construct one are different cases. The latter remains unevaluated until explicitly requested, consistent with the adopted load/evaluation boundary.

### Adopted minimal rule

`#` constructs a reference relationship from node-valued results, including
reference nodes. It does not infer another node from an attribute's text.
Any explicitly declared scalar conversion precedes construction and belongs
to its expression/context contract. Consumer chain resolution remains separate.

Keep `{#datadom.attributes.commandfor}` as a valid surface example, conditional on its operand type. If a particular context supplies only a scalar string, the syntax remains valid but construction requires an explicit interpretation/conversion outside the operator. No native browser semantics are settled by the attribute name.

## Consumer responsibilities and examples

CEM-ML defines reference syntax, scope relationships, and shared evaluation
policy. Consumers define how to use the selected nodes:

| Consumer | Interpretation owned by the consumer |
| --- | --- |
| Schema validation | Obtain or evaluate the relationship, inspect targets, enforce schema constraints, and report source provenance. |
| Schema composition | Reuse a referenced construct instead of repeating its declaration; define validation and recursive composition semantics without copying source content. |
| `cem-element` relationships | Relate AST nodes to intended produced elements, preserve explicit IDs, and generate IDs where required. |
| `cem-element` attribute binding | Extract or project the binding's appropriate value, including an ID or ordered IDREF sequence when required. |
| External resource access | Resolve a URL and public part contract, then provide retained CEM nodes to evaluation. |
| Document inspection | Display retained nodes and graph connections without automatically resolving or expanding chains. |

For `#datadom.attributes.commandfor`, an attribute-node operand remains a
reference to that attribute. `cem-element` defines how it supplies the command
relationship and which produced element supplies the final ID. That consumer
uses the element's explicit ID or generates one when needed. CEM-ML does not
select the produced native owner or define universal text interpolation, ID
extraction, or attribute-output rules.

An evaluated reference remains a typed node relationship until the consuming
operation applies its rule. Shared browser behavior belongs in `cem-elements`
and is consumed declaratively under the
[declarative UI principle](declarative-ui-principle.md). External formats enter
through the [retained CEM AST import boundary](cem-data-import-principle.md),
and instance state follows the
[durable lifecycle](cem-element-lifecycle-principle.md).

Specific `cem-element` treatment is deferred until the CEM-ML reference design
is complete. Its intended template reference detection and element-to-ID mode,
interaction API, explicit scope crossings, and local-name compatibility are
action items in [todo.md](todo.md#deferred-cem-element-reference-consumption).

## Deferred details and implementation work

- Enclosed child override syntax is deferred in
  [roadmap.md](../roadmap.md#deferred-cem-reference-syntax-decision).
- Public query reference type, target-access and target-sequence comparison
  syntax, and graph/transport contracts are deferred in
  [roadmap.md](../roadmap.md#deferred-cem-reference-query-and-transport-contracts).
- Concrete expression/lexical representation, linkage to existing specialized
  records, runtime outcome handling, and schema traversal limits and policy
  declarations are actionable work in
  [todo.md](todo.md#ast-node-reference-implementation).
- `cem-element` consumer modes and attribute API remain deferred in
  [todo.md](todo.md#deferred-cem-element-reference-consumption).

Scenarios for later design verification are preserved next to those action
items. Deferring these details does not reopen the adopted scope, lifecycle,
reference-chain, or consumer-ownership principles.

## Design authority and limits

The accepted contract consists of the reference AST node, strict node-valued
construction, adopted precedence and CEM-ML/XML surfaces, implicit scope
defaults and enclosed override semantics, source-position lexical resolution,
runtime-supplied lifecycle evaluation without mandatory context IDs or source
target lists, any-node target eligibility, non-owning cyclic graph edges,
ordered targets, graph identity, bounded consumer chain resolution, and
scope-schema-controlled diagnostics. URL fragment access belongs to the
external resource/consumer contract.

Consumer behavior and reference updates after AST mutation remain outside scope. This design does not define changes to CEMT lookup, cem-element, browser wiring, or reactivity.

## Initial implementation choices (2026-10-04)

The initial parser representation retains `CemAstNode::Reference` with its
expression source, containing AST node handle, source maps, and optional
ordered target IDs. These are implementation choices, not requirements to
give a context an authored ID, persist a complete evaluator environment, or
publish every result onto the authored source node. `None` is
unevaluated; `Some([])` is a successfully resolved empty selection. Parsing
and XML ingestion never execute the query. Explicit callers use the ordinary
CEM-QL compilation/evaluation entry points; no consumer scheduler or mutation
API is introduced.

The initial query operator constructs a fresh reference whose targets are its
node-valued operands. A selected reference node is consequently a target in
its own right, and `##nodes` nests two references. The existing
`dom:reference` helper retains its older behavior. Neither behavior defines
consumer traversal of the retained chain. The implementation uses the existing
generic node type; public query type/access contracts remain roadmap work.

The shared `cem_ml_transform_cem_ql` query bridge now projects CEM-document
artifacts through `RetainedCemTree::from_shared` and the same native query view
used by lifecycle imports. Both paths construct ordered, repeated, nested and
empty references over original nodes; an authored reference remains unevaluated
when selected as a target. XML, JSON, YAML and CSV lifecycle fixtures retain
their native owners, and reference-looking data stays literal outside recognized
expression slots. Projection does not serialize, copy an AST or evaluate links.
Multiple projections of one source arena share node identity; independently
imported arenas remain distinct. Native children/attributes are flat sequences,
and document metadata uses `input.kind` instead of automatic record bindings.

CLI/shared CEM-QL query ingress now admits CEM source through the same native
view. `QueryPreparationRequest.source_owner` distinguishes the original external
lifecycle AST from a retained CEM tree and captured lexical snapshots. CEM parsing
uses the ordinary schema-machine/builder stream once, shares its original arena,
and omits the runtime model consumer. The adapter retains those snapshots for a
later explicit consumer; query preparation neither compiles authored reference
expressions nor supplies their runtime contexts. The native fixture attaches the
saved namespace bindings after ingress and consumes three references against the
correct original targets without source cloning or target writeback.

`CemQlNativeItemsOwner::source_owner()` exposes this distinction, and its
`lifecycle_owner()` accessor is now optional. Existing external imports retain
their original lifecycle owners; CSS selector and XPath keep their existing
input admission. CLI fixtures cover construction without authored IDs across
CEM/XML/JSON/YAML/CSV, inert malformed CEM/XML source slots, and attributed
rejection of URL/ID string operands. A resolver-counting fixture verifies that
supplied source bytes and reference construction do not trigger resource reads.
CLI JSON retains its opaque native result descriptor, rather than adopting graph
transport or a public reference-access spelling. Generic member inference now
uses the existing native field view for bare unprefixed fields on inferred node
values, including unary-reference locals and parenthesized constructions. Field
results remain dynamically typed; missing fields produce an empty selection.
Explicit calls, prefixed dispatch and registered pipeline functions retain their
existing checks. This implementation alignment does not adopt a public reference
type or target-access contract, nor evaluate retained source expressions.

The former CEM-document record projection is available only through the explicit
Rust `cem_document_record_query_stream` compatibility function. Its array-wrapped
navigation and record operand rejection remain intact. Invalid native structure
cannot silently select that projection. Artifacts without original source text
retain source maps and URI provenance without reconstructing source text or
coordinates. This bridge migration leaves public query reference type/access
syntax and native output graph transport decisions deferred.

The CEMB AST codec introduced reference expression/context/target state in
version 3; the current version 4 also persists owning native attribute-value
edges. Version 3 reference graphs and version 2 ordinary nodes remain readable.
Self-reference, cycles and ordered duplicate target edges are supported within
one CEM arena, independently of structural child edges. Invalid context/target
handles are rejected. This does not change the older native output-value
artifact's expansion and consumer projection contracts.

### Verified codec and inspection boundaries (2026-10-05)

CEMB preserves reference occurrence handles, containing-node handles, source-map
frames and optional target lists. Absent targets, resolved-empty lists and cyclic
or repeated lists remain distinct. These handles are arena labels; they do not
require authored IDs. A decoded source can be wrapped in a retained native tree
and consumed with a newly supplied runtime context. The reload fixture retains
the original source map, waits for input readiness, and evaluates against two
independent native owners without changing the binary payload.

The caller supplies the source URI/text and runtime context at reload. CEMB does
not serialize host scope registrations, crossing grants, live bindings or pending
consumer outcomes. Captured lexical snapshots are separate from the AST payload;
transport of those snapshots requires a separately specified handoff. The reload
fixture uses a context-bound expression without captured namespace overrides,
and does not claim to persist every lexical integration contract.

Retained native query views expose reference nodes and ordered target handles
without turning targets into owning children. AST-vocabulary inspection retains
the original tree owner and emits one row per owning occurrence, with separate
expression, context and target metadata. DOM diagnostics similarly report
reference kind and target IDs without graph expansion. These inspection paths
preserve absent versus empty metadata and never evaluate a link.

The formatter and XML convention writer export authored expression source.
Optional target lists are omitted; importing those surfaces constructs fresh
unevaluated occurrences and provenance for the exported source. Normalized event
inspection represents source syntax and its opaque expression payload, rather
than a persisted AST target graph. `cem_tree_nodes`/`ast_stream` are writer-facing
source projections and use the XML `cem:expr` convention; they are not the native
reference query view or a graph codec. Inspection and source export therefore do
not claim transport of execution-local results. Cross-owner native value graphs,
cyclic output expansion and pending-outcome transport remain actionable decisions
in [the roadmap](../roadmap.md#deferred-cem-reference-query-and-transport-contracts).

Adoption does not claim that these initial representations implement every
2026-10-04 rule. The implementation checklist owns alignment and verification;
cyclic graph/native output transport compatibility remains explicitly deferred.

The schema now declares standard traversal limits of 128 reference links per
path and 100000 work units per resolution. `ReferenceTraversalLimits` reads
the embedded schema defaults and validates effective scope overrides, retaining
inherited bounds when a scope omits them. The explicit consumer resolver applies these bounds together with the
effective per-link disposition. XML reference ingestion also retains query-payload source spans when
folding text or CDATA into the retained node.

The native imported CEM query view preserves reference expression, lexical
context and optional target edges. Unevaluated targets remain absent, distinct
from resolved-empty targets. Construction accepts importable source node kinds
and explicitly supplied typed error nodes; the semantic import boundary still
rejects source error nodes. Independently imported document owners retain
distinct runtime identities even for equal source content, while repeated
selections preserve the same owner's identity. Query-local memoization may
reuse an import result. Existing `data:read(...).id` selection keys remain
source-derived across fresh unchanged reads and are distinct from runtime
owner identity. These view fields do not adopt a new public query
target-access spelling or implement chain resolution.

`ReferenceScopePolicy` composes the schema-owned traversal bounds with
`ReferenceUnresolvedPolicy`. Overrides are validated before returning a new
effective policy. `apply` treats a consumer-supplied unresolved fact without
executing its expression, following targets, changing the authored graph, or
checking empty-result cardinality. Every disposition preserves the fact;
mandatory marks failure, warning reports, and neutral/ignore emit no diagnostic.
`value::reference_resolution::resolve_reference` now provides an iterative,
explicit host-driven walker. The host supplies typed nodes, runtime scope keys,
effective bounds/dispositions, edge permission and evaluation. It preserves
order, multiplicity, source graphs and original invalid-expression diagnostics.
Cycle/limit failures remain incomplete under every disposition. Pending and
invalid states remain distinct from unresolved links and resolved-empty results.
Occurrence metadata retains runtime identity and source provenance; native
constructed references need no saved AST handle or captured environment ID.

A CEM-QL consumer fixture uses the existing expression evaluator with retained
CEM-ML source and independently supplied native data trees, proving owner
retention and no source writeback. The initial schema declaration consumer is
described below. Native package lifecycle compilation is connected explicitly.
Implemented schema consumers apply their cardinality contracts separately from
resolution completeness, as detailed below. Authored crossing declarations and
public query access syntax remain deferred checklist work.

### Evaluation and policy adoption checkpoint (2026-10-05)

The shared evaluation and policy work is complete for the implemented native
consumers. `resolve_reference` returns an execution-local `ReferenceResolution`
whose typed nodes retain their original owners. Results survive disposal of the
host and context. The authored source remains unevaluated until a caller requests
consumption; pending, resolved, unresolved and invalid are runtime result states,
not mutations of that source. Compiled expression artifacts retain source-position
bindings and are distinct from evaluated target lists.

Effective schema policy supplies unresolved disposition and validated depth/work
bounds. The request cap and destination caps both constrain traversal; crossing
or reentry never replenishes work. Permission comes from explicit host grants,
independently of lexical scope inheritance. Cycles and exhausted budgets remain
incomplete under every disposition. Consumers check empty-result cardinality only
on complete selections, using their own schema contracts. These contracts do not
choose the deferred child scope-reference spelling, public query access API or
cem-element ID projection.

The verified suite mapping and remaining adoption actions are recorded in
[the reference implementation checklist](todo.md).

### Initial schema declaration consumer (2026-10-04)

The first schema consumer is declaration reuse in `{elements}` during explicit
schema compilation. A reference selects zero or more named `{element}`
declarations. Their original retained owners and declaring lexical `{uses}`
aliases are used to compile them; the source tree is not cloned or rewritten.
References occupy their authored collection position, preserving the existing
last-name-wins behavior. Consumer schema field contracts still apply after the
collection is assembled. The attribute declaration collection uses the same consumer contract, as
described below. Other collections and validation-input references remain work.

Available valid declarations are retained when another selected branch remains
incomplete. `SchemaDocumentModel.declaration_references` reports per-site and
aggregate completeness, with pending, unresolved and invalid outcomes distinct
from resolved-empty selections. Mandatory failures and invalid expressions or
target kinds remain compilation failures. Neutral or ignore never implies that
an incomplete model is ready. Target-kind/name diagnostics point to the
consuming reference; traversal diagnostics preserve the original failing link.

`compile_schema_with_declaration_references` invokes the shared resolver through
a consumer host. The host adapts retained source and terminal declaration
handles to its own typed runtime representation, so constructed references do
not require a fake source arena. Each site makes one bounded resolution request;
this is not a whole-compilation work budget or an automatic lifecycle service.
Compilation without an evaluator records unevaluated sites as pending and does
not request evaluation. The native package lifecycle stage described below
invokes the consumer before coordinated publication.


### Native CEM-QL declaration consumer (2026-10-04)

`cem_ql::schema_references::CemQlSchemaDeclarationHost` provides an explicit
compilation stage over retained CEM trees. `register_scope` accepts the retained
owner, effective reference policy and an optional `StandaloneExpressionContext`.
A missing context yields Pending when that scope's reference is evaluated;
terminal declarations do not require a context. `set_context` supplies a later
lifecycle snapshot. `compile` uses the retained source's original arena and
never updates authored reference targets or shares a selection cache.

The host evaluates the original source expression through CEM-QL and consumes
its outer `#` constructor. The common resolver follows nested source or native
reference nodes under request and destination limits. Original query errors
remain Invalid, and resolved-empty selections remain distinct from Pending.
The schema consumer enforces declaration target kind and original lexical
`{uses}` bindings after resolution.

`RetainedCemTree::ast_owner` shares the original immutable AST allocation.
`cem_ql::eval::RetainedCemNode` and `retained_cem_node` provide checked typed
source handles for imported CEM views; arbitrary host/XPath projections are not
coerced into source declarations. Consumed declarations retain their original
allocation after the query context and semantic tree views are dropped.

Runtime scope handles are specific to their host and do not add authored IDs.
`register_scope` establishes an explicit relationship boundary and retains its
runtime lifecycle context. `register_lexical_scope(parent, context, policy)`
retains a distinct lexical lifecycle snapshot within that parent's relationship
boundary and retained owner. It neither assigns source occurrences nor copies
runtime inputs from the parent. Compilation artifacts, context readiness,
context replacement and effective policy remain keyed by lexical handle;
directed crossing grants use relationship boundary identity. Nested lexical
snapshots inherit that identity, and grants remain directed. Explicit same-owner
boundaries still require grants. Existing callers of `register_scope` retain
that boundary behavior; parser snapshots enter through explicit host attachment.
`assign_subtree_scope` supplies explicit effective child scope metadata; the
nearest source ancestor selects it. `assign_following_scope` records a
caller-completed existing sibling switch through its retained boundary handle.
The boundary and earlier siblings keep their binding; following siblings and
their descendants inherit the new effective scope until a later switch or the
containing scope ends. Nested transitions restore enclosing defaults; explicit
child subtree mappings take precedence. Retained structural sibling order is
used for CEM and imported sources, without global ID lookup. Identical repeat
handoffs are idempotent and conflicting repeats are rejected. Parser/schema
snapshots can be attached through `attach_captured_lexical_scopes`; the runtime
still supplies contexts and completed specialized scope transitions. These handoffs
do not choose the deferred child scope-reference syntax. Different relationship
boundaries require a directed `allow_scope_crossing` grant, and unregistered owners cannot borrow permission
from an equal node ID or a supplied query binding. Native constructed references
inherit the evaluating scope; retained targets use their registered scope.

The parser-side `CemSchemaMachine::track_lexical_scope` is an opt-in normalized
stream wrapper accepted by `CemAstBuilder`. Before each event changes the machine,
the observer can retain `lexical_snapshot()` namespace/schema metadata alongside
the original event's source occurrence. Host and wrapping schema defaults, sibling
switches and namespace rebinding therefore remain inspectable after frames close.
The original events reach the builder; no second AST, reparse or event history is
required. One EOF notification exposes final machine diagnostics. Snapshots contain
neither evaluator environments nor relationship boundaries, and capture does not
execute reference or schema selector expressions. Completion alone establishes
neither validity nor dependency readiness.
Closed-slot lifecycle fixtures verify CEM standalone/native attribute slots and
aliased XML CDATA references against forward source targets. Loading, frame
closure, lexical attachment and context preparation invoke no native capability;
only explicit consumer resolution executes it. Repeated consumption executes
again while retaining source/artifact ownership. Balanced host slots with invalid
query syntax remain pending without runtime inputs, then report source-attributed
query errors when consumed. XML attributes continue to retain literal values.

`build_with_lexical_scopes()` now builds one fragment with a
`LexicallyScopedDocument` sidecar. It associates snapshots with the original
standalone/native attribute reference and general expression node identities
supplied by the builder, retaining only surviving occurrences after folding.
No source-coordinate matching or arena copy is used. Snapshot lookup requires
the original AST allocation; an equal node ID in a different owner is rejected.
Final machine diagnostics remain inspectable separately from builder diagnostics.

`RetainedCemTree::from_shared` projects the same AST allocation with the existing
tree constructor's structural checks. The explicit QL lifecycle method
`attach_captured_lexical_scopes(captured, prepare)` checks that owner is registered
and no occurrence already has a direct assignment, before invoking preparation.
The callback receives the original typed occurrence, its saved bindings and the
nearest existing scope. It supplies a runtime context (or `None` for pending inputs
or schema selection) and effective reference policy. All preparation completes
before attaching distinct lexical scopes, preserving each existing subtree/sibling
relationship boundary. Returned occurrence/scope handles support readiness updates
through `set_context`; attachment neither evaluates selectors nor creates grants,
compiles expressions or writes source targets. Separate host executions can share
the saved source without sharing results. Package compilation and ordinary
CEM/XML input validation expose these associations at their explicit lifecycle
hooks; callbacks supply contexts, readiness and policies. Effective destination
and request limits still constrain evaluation under the existing common resolver.

`import_xml_ast_with_lexical_scopes(document, schema)` now attaches XML occurrence
bindings during the existing typed import pass. Original event/node correspondence
selects the owning reference node; the importer retains entity/CDATA expression
source maps while folding. Expanded XML namespace names select CEM schema behavior,
including aliased host attributes and schema elements. Namespace declarations on
the expression itself apply to its snapshot. Existing schema-frame rules retain
host/wrapping defaults, no-body sibling switches and nested restoration; foreign
namespaces cannot activate CEM behavior by using the `cem` prefix. XML attributes
remain literal. The returned captured AST, semantic metadata and correspondence
share the original owner through `RetainedCemTree::from_shared` and the same QL
lifecycle handoff. Capture executes neither selectors nor reference expressions;
consumer preparation still determines schema readiness, policies, inputs and grants.

Engine verification now covers the existing schema directive, host, wrapping and
sibling forms together with inherited namespace bindings, rebinding and nested
restoration. Identical retained expression source remains distinct across saved
bindings; a pending selector waits for explicit runtime preparation. Independent
hosts can evaluate one original owner against different current inputs. Captured
lexical scope changes neither create vendor grants nor reset request depth/work;
stricter effective destination limits constrain their consumed subtree.

Existing default namespace/schema properties and named inline declarations are
also verified through both engine paths: declarations become visible after
closure, inherited declarations can be shadowed in a child, and leaving the child
restores the parent binding. Equal authored IDs and arena indices in separate
vendor owners cannot share a crossing grant. This verifies the existing scope
forms; it does not choose the deferred enclosed child override syntax.

Named inline declarations also retain their original declaring AST node identity.
The CEM builder feeds that identity back into the pending schema frame before its
attributes or closure; the XML importer supplies its existing imported node ID.
Declaration closure publishes it with the existing lexical binding. Metadata-only
machine observation has no owner or builder ID and leaves `source_node` absent.
`LexicallyScopedDocument::inline_schema(owner, occurrence, name)` returns the
visible declaration as a `SchemaDeclarationNode` only for the captured owner and
occurrence. It uses the saved binding, without matching names or byte offsets
against an arena, copying the AST, or evaluating the declaration. Inherited,
child-shadowed and restored bindings retain original owner/node identity; later
same-name declarations cannot rebind earlier occurrences. Engine lifecycle
preparation can use this handle to select its consumer context, while runtime
inputs, readiness, traversal policies and crossing grants remain explicit.

The host is opt-in consumer code. Source-only package loading remains pending.
The native package bridge supplies caller-selected lifecycle context before
coordinated publication, as described below.

### Schema readiness and coordinated package activation (2026-10-04)

Incomplete models remain available for inspection and do not participate in
final validation. `SchemaDocumentModel::is_ready_for_validation` requires
complete declaration-reference outcomes with no mandatory/invalid reference
failure. Legacy structural projections retain their existing treatment of
behavior supplied by other consumers; package publication separately rejects
hard compilation diagnostics.
`SchemaDocumentModelRegistry::get` and `inspect_for_identity` retain candidates;
`resolve_for_identity` returns the last complete active model. Initial inactive
candidates block built-in fallback. During refresh, inspection retains the new
candidate while validation continues using the last complete model.

Final validation does not run structural or behavior checks on inactive models.
It preserves original compilation diagnostics. When those diagnostics do not
already express a hard failure, the attempted final operation reports
`cem.schema_model.not_ready`. This is a consumer-operation readiness error;
it does not change the schema's neutral/warning/ignore unresolved-link policy
or insert a diagnostic into the retained model's link facts.

Initial package loading retains incomplete models for inspection and withholds
schema registration, converter routes and package artifacts. Source-only loading
records unevaluated declaration references as pending, without executing them.
Ready packages, including converter-only packages, publish on staged registries.
An incomplete or rejected replacement preserves every active schema/model,
converter route and artifact. A complete replacement removes the previous
package's registrations and lookup indexes, then publishes all new parts
together. Candidate models remain inspectable even when publication fails.

Schema and converter registries retain `SchemaPackageOrigin`: `Builtin`,
`Manifest(resolved_manifest_uri)`, or `Untracked` for directly registered entries.
The exact resolved manifest URI is the ownership key; URI aliases do not
implicitly establish equal ownership. Same-origin refresh is permitted.
Different-origin, built-in or untracked replacement requires a separate
`EngineContext.schema_package_replacement_grants` entry. Each
`SchemaPackageReplacementGrant` names the package ID, expected current origin
and incoming resolved manifest URI. All existing registry owners must be
covered; an unrelated package's schema/converter collision still fails.
Manifest inclusion alone never grants authority. Direct registry mutation
resets that package's origin to `Untracked`, requiring explicit authority
rather than borrowing an earlier manifest's ownership.

The grant and publication APIs are runtime consumer policy, not CEM-ML syntax.

Grant authority comes from explicit native CLI arguments or the embedding host's
`EngineContext` (adopted 2026-10-05). The native CLI accepts repeatable
`--schema-package-replacement-grant` JSON control values with `packageId`, a typed
`expectedOrigin` and `replacementManifestUri`. Exact ownership matching remains
in the shared engine. Config merging preserves caller grants but cannot create
them; the existing permissive run-config decoder ignores unknown grant fields.
Virtual command arguments attempting to supply grants fail before resource loading
with `cem.command.replacement_grant_host_required`. Common command preparation
clones and preserves only the embedding host's existing grants. The low-level
WASM executor accepts an optional final `host_configuration_json` argument,
strictly decoded as `CommandHostConfigurationV1` before callbacks. It supplies
exact grants to a fresh per-execution context; omission supplies none. Invalid
setup reports `cem.command.host_configuration_invalid`. Node/browser worker clients
capture optional constructor-owned `hostConfiguration` before asynchronous startup
and pass its JSON snapshot separately from request data on each execution. A new
client captures fresh setup; existing request fields and execute options cannot
replace the snapshot. The Node service wrapper forwards the same host option.

### Explicit package lifecycle compilation (2026-10-04)

`EngineContext.schema_package_compiler` is an optional native consumer hook.
`CemQlSchemaPackageCompiler` in the bridge crate prepares a fresh declaration
host for each invocation using the caller's current runtime snapshot, effective
scope policies and directed crossing grants. Ordinary query adapter registration
does not install the hook or supply data. `load_schema_package_manifest_into_context`
exposes the explicit load/refresh stage; requests containing package manifests
invoke the same stage on their enriched context. No scheduler or parser-time
reference evaluation is introduced.

The compilation request carries package/schema identities, resolved manifest
origin, the retained schema tree and `lexical_scopes` captured from its original
AST allocation. `schema_package_sources` retains source arenas and bindings by
resolved URI and byte revision, including inactive candidates;
`get_lexical_scopes(uri)` exposes their snapshots and machine diagnostics.
Unchanged source reuses both allocations when a pending candidate is compiled
later. Changed bytes replace tree and metadata together; failed parsing or
descriptor validation leaves the last valid pair available. Successful descriptor
extraction borrows the retained tree. Preparation callbacks can pass the saved
metadata to `attach_captured_lexical_scopes`, supplying current contexts, policies
and grants explicitly. Bootstrap machine diagnostics remain inspectable without
changing package admission or publication gates. The cache stores source
structure and bindings, with evaluated targets supplied by each lifecycle.
Independent engine snapshots can share the source without sharing reference selections or writing results back into the authored tree.

Replacement authority is checked before invoking the runtime compiler.
Candidate compilation must return the requested schema identity. Compiler
preparation failures retain original diagnostics and add a hard compilation
failure if none was supplied; invalid traversal limits also fail publication.
Native query failures retain their original codes and source attribution.
The existing readiness and coordinated publication gates apply to every result.

Remaining validation/composition consumers and deferred syntax/element behavior
remain actionable work, with verification scenarios, in
[todo.md](todo.md#5-integrate-schema-validation-and-construct-reuse).


### Schema consumer inventory and attribute declaration reuse (2026-10-04)

The consumer site determines target kind and cardinality. A generic reference
node does not imply that a scalar schema field can consume a collection, that
one declaration imports its entire declaring schema, or that a validation
input is rewritten. The existing sites are:

| Site | Target contract | Evaluation phase and current status |
| --- | --- | --- |
| Direct references in `{elements}` | Zero or more named `{element}` declarations | Explicit schema compilation; implemented. |
| Direct references in `{attributes}` | Zero or more named `{attribute}` declarations | Explicit schema compilation; implemented. |
| Direct references in `{behaviors}` | Zero or more `{behavior}` declarations with nonempty `@name`, `@implementation` and `@execution` | Explicit schema compilation before diagnostic binding; implemented. |
| Direct references in `{diagnostics}` | Zero or more `{diagnostic}` declarations with nonempty `@code` | Explicit schema compilation; binding follows complete declaration assembly; implemented. |
| Direct references in `{constraints}` | Zero or more `{constraint}` declarations with nonempty `@kind` | Explicit schema compilation after behavior assembly; implemented. |
| Direct references in `{field-contracts}` | Zero or more `{field-contract}` declarations with nonempty `@name` and `@target`; ordered applications | Explicit schema compilation after element and behavior assembly; implemented. |
| Element `@base` with `{uses}` aliases | One named element model from the declaring lexical alias/registry contract | The schema-owned `element-base` datatype preserves literal QName/wildcard lookup and admits an explicit native reference selecting exactly one named element declaration. |
| Attribute `@type` and diagnostic strings; behavior/function strings; constraint and field-contract target strings | Existing datatype, diagnostic, function and local-name contracts | Compile-time dependency checks and final input validation remain compatible; these strings are not implicit native reference constructors. |
| Specialized schema and namespace scope properties | Existing schema/namespace identity and runtime scope contracts | Lexical capture and explicit effective-scope handoff are implemented. Native property selection contracts and enclosed child override syntax remain tracked separately. |
| Native references in structural validation-input child positions | Ordered zero/many retained structural child nodes; parent and selected-subtree rules use the consuming schema | Explicit native structural validation API, QL host stage, per-placement behavior checks and typed node function candidates are implemented. The optional engine stage retains CEM/XML parser owners and JSON/YAML/CSV lifecycle owners; other specialized validators retain their existing paths. |

Attribute collection references select zero or more named attribute declarations
at their authored position. Selection order and repeated targets are retained;
model insertion preserves last-name-wins behavior alongside ordinary declarations.
The source tree is not expanded, and result handles retain original owners.
Reference sites across the supported collections are reported in authored order.

The shared resolver follows nested reference selections under request and
destination limits, explicit directed scope grants and the effective unresolved
policy. Missing runtime context is pending, a resolved empty selection is complete,
and unresolved/cyclic/limited branches prevent readiness even under ignore.
Valid partial declarations remain inspectable. Wrong target kinds and empty names
are consumer errors at the selecting reference, independent of disposition.

Compiled attributes use the existing datatype, default-value and diagnostic
contracts. Their source maps come from the original declaration. Reusing an
attribute does not automatically import its declaring schema's diagnostics,
behaviors or other declarations; dependency strings continue to be checked
against the assembled consuming schema. Such dependency declarations can be
supplied explicitly through their existing forms. This preserves existing
consumer behavior while their own reference-enabled collection sites are added.

Package lifecycle compilation and publication use the same readiness gates for
attribute and element references. Source-only attribute links remain pending;
the caller can complete the same retained source later with runtime inputs.
Engine validate/check reports expose `reportAst.validation` with aggregate
`complete` and an ordered list of `{input, complete}` entries. Completion is
independent of diagnostic counts, severity and hard violations. Source-only
validation leaves consumed structural references pending and defers their child
contracts and whole-document behavior hooks; it continues independent checks.
A completed report can contain schema violations. An incomplete report cannot
produce a successful CLI validation/check verdict merely because it has no
violations. Reports from older callers without completion metadata retain their
existing interpretation. CEM-ML, explicitly requested JSON, Markdown and HTML
report presentations expose the same completion state.

This reporting stage does not evaluate references during parsing or loading.
The explicit native consumer remains available for runtime-selected evaluation;
engine runtime-hook integration awaits native retained behavior evaluation.

`SchemaBehaviorEvaluator.validate_retained_structure` is an additive native
handoff; the existing `validate_document` API remains available. The new hook
receives the retained source owner, ordered roots and a read-only graph of
placement nodes. Each placement retains its original arena/node handle,
optional original declaring schema, child edges and child-selection completeness.
Repeated selections can share one source node while keeping separate placement
indices. These indices are runtime graph positions, not authored IDs or public
scope identifiers. Consumers can distinguish original source ancestry from the
consumed relationships without constructing an expanded CEM document.

The explicit `validate_structural_input_references_with_behavior_evaluator`
stage enriches declaring-schema metadata and invokes the new hook only after
structural selection is complete. Structural-only validation does not request
additional declaring-schema lookups.
Native QL exposes `validate_input_with_behavior_evaluator` using its current contexts,
policies and directed scope grants. Behavior returns diagnostics and completion
separately: a completed stage may contain violations, while pending or unsupported
behavior remains incomplete. Existing evaluators that implement only the legacy
whole-document hook are not called on a synthetic document and do not implicitly
complete the new stage. Structural-only validation remains available without
installing a behavior evaluator. Behavior checks apply per consumer placement.
Sharing one original source node never deduplicates placements: different
consumed parents or selected child relationships can produce different results
while original owner and lexical context stay shared.

Native QL exposes this graph through `RetainedValidationQueryTree` and
`ValidationPlacementNode`. Each snapshot shares original CEM arenas and retains
only placement handles/edges. Native query parent/child axes follow consumed
relationships; attributes remain typed original-source nodes whose query parent
is their owning placement. Source maps and optional declaring schemas retain
original provenance. Placement and snapshot identity distinguish repeated
selections and overlapping arena node IDs without introducing authored/context
IDs. Explicit text extraction follows consumed children, including selections
from other owners, without expanding the source tree or converting nodes into
records.

The query-tree constructor rejects incomplete stages/children and malformed
owning graphs before exposing access. A completed zero-target selection provides
an empty query forest; pending roots do not. `RetainedValidationStructure`
includes overall completion so pending roots cannot be mistaken for readiness
merely because all available placements have complete children. Views expose
only the already consumed forest; their creation does not evaluate references
or grant additional scope crossings.

Any retained behavior function candidate parameter requires an explicit `node`
type (including qualified schema node types). Existing whole-document
behavior functions keep their record-shaped `object` candidates on the legacy
path. The retained evaluator checks candidate signatures even when the current
selection is empty; object signatures fail the retained contract rather than
silently adapting or copying the node.

`CemQlSchemaBehaviorEvaluator.validate_retained_structure` now selects native
element placements from the current query snapshot and evaluates each placement
independently. Selecting the same placement twice checks it once; distinct
placements of the same original node remain independent. Non-element or foreign
snapshot selections are invalid. Attribute-name match conveniences remain scalar
bindings, while `candidate` itself is native. Function paths such as
`$candidate.parent.name`, `$candidate.name`, and `$candidate.attributes.value`
extract consumed ancestry or original scalar data. Diagnostic output objects and
arrays are an explicit reporting boundary: returning a native node there requires
scalar extraction and never implies AST serialization. Original source maps and
placement attribution accompany emitted diagnostics.

The explicit QL host supplies context snapshots and directed scope grants before
behavior access. Pending structures defer behavior; complete empty structures
remain valid empty selections.

`EngineContext.input_validation_stage` is an optional, explicitly installed
`InputValidationStage`. Parser-backed validate/check inputs with ready consuming
models share their original parsed arena through an `Arc<RetainedCemTree>` without
reparsing or copying AST nodes. The CEM pipeline captures occurrence bindings in
the same schema-machine/builder stream, including root namespace/module-map
bindings; diagnostics and version pins are finalized before sharing that owner.
`PipelineRun.document` is now an `Arc<CemDocument>` and its `lexical_scopes` shares
that allocation. Each `InputValidationRequest` supplies the retained source,
optional `lexical_scopes`, consuming model, root scope, validated default/overridden
reference policy and optional behavior evaluator. CEM inputs supply the saved
metadata. Ordinary XML inputs with a ready consuming model and an explicit stage
also use the specialized import: already parsed native XML owners are reused,
while custom-schema XML is parsed once. The query tree retains that native owner,
imported source semantics and captured aliases; XML attributes remain literals.
Source-only XML and other specialized XML-family validators keep their dedicated
paths. JSON/YAML/CSV validate/check inputs with a ready model now reuse their
already parsed lifecycle AST through `import::retain_lifecycle`, the shared query
import boundary. The returned native tree retains that original lifecycle owner;
no source reparse or record-shaped runtime substitute is used. Reference-looking
strings remain literal data, and these inputs supply `None` for `lexical_scopes`.
The stage can evaluate a retained template against each supplied native input
under its explicit runtime context and crossing grants. Missing or incomplete
models do not invoke the stage. An inspected incomplete model, failed native
parse or failed import cannot report completed runtime validation; native source
diagnostics remain present. Other specialized validators keep their existing
paths. The installed stage can call
`attach_captured_lexical_scopes` to prepare occurrence contexts and policies;
existing callbacks remain usable. The runtime supplies fresh context snapshots
and scope grants,
then invokes the existing retained structural/behavior consumer. Calls may run
concurrently for independent inputs; the engine commits outcomes in stable input
order. Query-adapter registration and parser/load requests never invoke this stage.

The selected stage replaces only the source-only schema-model consumer, avoiding
duplicate structural/behavior checks; other source pipeline checks still run.
Unavailable or blocked consuming models preserve the source-only readiness gate.
Malformed scope bounds and preparation failures remain incomplete. Original
failure diagnostics are preserved; preparation failure without a hard diagnostic
receives `cem.schema_validation.runtime_stage_failed`. A returned incomplete
outcome can have no violations, and a completed outcome can have violations.

Runtime diagnostics for selected owners carry their original URI/coordinates.
The engine projects source-pipeline diagnostics before merging runtime outcomes,
so vendor diagnostics never use the input's line index. Native behavior emission
uses the placement's own source frame for byte offset while preserving the full
original source-map stack. Explicit runtime report projection is the consumer's
responsibility. JSON/YAML/CSV lifecycle adapters already use the shared typed
import handoff described above. Other specialized validators retain their
existing paths and are not admitted by this consumer stage.

Source AST attributes now retain `value_nodes`, an authoritative sequence of
owning native arena handles. Literal attributes keep their optional text value
and an empty native sequence. CEM-ML unquoted brace expressions leave the
literal value empty and populate the native slot. A leading `#` becomes a
`Reference`; other expressions keep the existing `$` element/text representation.
The lexical containing element supplies a reference's context handle. Parsing
never evaluates the expression, fills targets or performs literal ID lookup for
native attribute values. Quoted and bare values remain literals.

The lexical foundation preserves `AttributeValueSyntax::Expression` on CEM
unquoted brace spans and `Literal` on quoted/bare CEM values and XML/HTML
attributes. Normalized `ScalarValue::Expression` events carry that distinction
into AST construction; expression-valued `type` does not trigger a static
content-type switch. Attribute and standalone expressions share the scanner
for strings, escaped/doubled quotes, CEM-QL `/* ... */` block and `// ...`
line comments, retained nested `(: ... :)` comments and nested braces. Line
comments end at LF or CR (including CRLF); a host brace inside either CEM-QL
comment form cannot close a standalone or attribute slot. Both native and editor
scanners retain the same opaque payload. This does not validate the query or
make additional comment/string conventions executable CEM-QL syntax.

Native values share their immutable retained source owner. Their source parent
is the attribute, while their lexical evaluation context remains the containing
element. They are not XPath attribute children. `attribute_value_nodes` exposes
original arena handles; native CEM document query views expose `.valueNodes` and
return native nodes from `.value` when the sequence is populated. Literal
`.value` remains compatible. No source arena is cloned into a runtime substitute.

CEMB version 4 persists owning value edges and validates native ownership;
versions 2 and 3 remain readable for literal attributes. Projection binary
version 2 records the new value edges and expression event tag; validators
continue accepting versions 1 and 2. Typed inspection records value-node edges
and unevaluated references. Explicit JSON exports preserve native value nodes,
and CEM-ML writers preserve expression spelling and quoted literal lookalikes.
The typed markup writer requires consumer resolution of native values.

The adopted first slice supports CEM-ML unquoted brace expressions. XML attribute
expression recognition is deferred as an actionable compatibility design item;
XML literals remain unchanged. This does not choose enclosed child-scope syntax
or cem-element ID binding behavior.

Schema attribute consumers require an explicit `node` type for native values.
Primitive and untyped contracts do not automatically extract scalar values from
referenced nodes. Literal inputs, scalar conversion and textual schema defaults
cannot construct values for a node contract. Contract violations retain the
original attribute source map and schema-owned type diagnostic behavior.

The implemented guards keep native slots out of lexical/facet validation while
retaining attribute presence for required-attribute and field-presence checks.
Source-only behavior validation waits for consumption. Without lifecycle context,
retained input validation remains incomplete for native references and general
expressions while running available structural checks. The source nodes and
reference targets stay unchanged. These guards do not evaluate native values;
the explicit lifecycle consumer supplies context and shared bounded resolution.

The adopted schema consumer policy requires exactly one target by default.
Any explicit `itemCount`, `minItems` or `maxItems` facet replaces that default
with the authored envelope: `minItems=0` permits an empty sequence, for example.
These facets accept node types alongside their existing name-list types and
retain existing numeric/envelope definition validation. Targets count by
occurrence; repeated selections retain order and original owner identity.

`validate_native_attribute_reference` is an explicit lifecycle entry point for
the current single-reference attribute slot. It uses the shared bounded chain
resolver and host scope/grant policy, retaining available targets independently
of completeness. Pending, unresolved, denied, cyclic and exhausted branches
never trigger count checks on a partial prefix. Complete count violations use
the schema's datatype-parameter diagnostic behavior and original attribute
source; diagnostic severity and consumption completeness remain independent.
Lexical `values` and `pattern` constraints on node types are schema compilation
errors and prevent evaluation. Native target validation belongs to node behaviors;
selecting nodes does not silently satisfy other unsupported constraints. Defensive
validation of caller-supplied models still leaves unconsumed constraints pending.
The standalone `validate_native_attribute_reference` entry point remains restricted
to one authored reference slot. Retained placement validation additionally consumes
general expression and composite native slots. `evaluate_input_expression` is an
explicit lifecycle host hook: custom hosts without it leave general expressions
pending. The production CEM-QL host evaluates the original expression text using
its registered lexical scope/context, preserves query errors, and requires retained
node or native reference results. Scalars and mixed scalar/node results are invalid;
there is no implicit scalar extraction or string-to-node lookup. Returned references
follow the shared chain resolver; returned expression-looking nodes stay authored
nodes rather than triggering another expression evaluation.

Composite slots traverse the original attribute's owning value handles in order
within one request. The attribute is an owning container, not a synthetic reference
or a new source arena. Container visits consume work without adding reference
expansion depth. General expression expansion and reference chains use the same
active identities, cumulative work and scope limits. Missing/repeated source owning
handles invalidate consumption; repeated selected target occurrences remain valid.
Pending or exhausted branches never turn an available prefix into a complete count
selection. Empty and multiple complete results follow the authored count envelope.
Primitive contracts never invoke this native expression consumer. Parsing does not
schedule evaluation, and no source targets are written back or DOM IDs generated.

Retained placement validation consumes eligible native attribute slots with
lifecycle context. Attributes inside selected structural subtrees share the
structural request's active reference identities, cumulative work and effective
scope limits. Independent authored attribute references start independent requests.
Attribute incompleteness preserves available structural and presence checks;
complete selections undergo cardinality checks independently of query readiness.

The consumer captures original owning subtree handles under the same bounded,
grant-checked traversal. It follows no saved reference context or target edges.
References authored inside selected target subtrees remain native reference nodes
until an explicit consumer resolves them. Capturing query access does not evaluate
those expressions or replace the target's authored child structure. Missing owning
handles, duplicate ownership and malformed attribute edges invalidate access.

Retained behavior attribute `.value` exposes the consumed native target sequence,
and existing local-name attribute conveniences expose that same typed sequence.
Literal conveniences retain their existing scalar behavior. The original source
handle preserves the authored attribute and reference. Query axes stay inside the
captured selected subtree: selected roots have no original parent, and outside
parents, siblings, document roots, reference contexts and saved targets are not
exposed. Broader access requires explicitly granted consumer evaluation. Native
query values retain original node identity, owner and scope when explicitly
referenced again; they do not manufacture a source arena or copied record.

The optional engine runtime validation stage invokes this retained consumer before
native behavior checks. Complete inputs supply typed attribute targets; pending
inputs remain incomplete and defer dependent behavior. Diagnostic disposition and
consumption completeness remain separate. These are schema consumer policies;
CEM-ML syntax does not prescribe attribute projection or DOM ID generation.

The remaining collection, scalar-link and validation-input work is in
[todo.md](todo.md#5-integrate-schema-validation-and-construct-reuse), alongside
scenarios for later verification. This inventory does not choose deferred child
scope syntax or cem-element ID projection behavior.


### Behavior and diagnostic declaration reuse (2026-10-04)

`{behaviors}` and `{diagnostics}` use the same explicit collection-reference
consumer as elements and attributes. `DeclarationReferenceSite.kind` records
the collection's target contract independently of expression spelling and
runtime owner identity. References accept zero or more declarations of that
kind, preserve selected order and repeated targets, and occupy their authored
position alongside direct declarations. Named behaviors and code-keyed
diagnostic definitions retain existing last-key-wins insertion behavior.
Existing diagnostic binding rules, including declarations without a behavior,
remain compatible.

All selected behaviors are assembled before diagnostic behavior/function
binding. Selected diagnostics are parsed with their arguments and original
lexical `{uses}` aliases; unqualified behavior names retain the assembled
consumer lookup contract. Qualified names keep declaring aliases rather than
capturing aliases from the consuming schema. Behavior definitions retain their
declaring schema namespace, aliases, inputs, parameters, results, inline
functions and source maps. A declaration without an explicit declaring schema
namespace uses the consumer's supplied schema identity; no authored root or
context ID is introduced. This does not extend scalar string lookup to an
implicit retained-document registry or grant additional scope crossings.

Reusing a declaration still does not import its whole declaring schema.
Dependencies can be supplied explicitly through direct or referenced collection
declarations. Inline behavior functions retain existing function binding rules;
other scalar function/alias composition follows its existing registry and
visibility contracts. Those scalar-site extensions remain separate action items.

An incomplete behavior or diagnostic collection cannot prove that a local
dependency is absent. Compilation defers the affected missing local lookups and
keeps the model inactive. When selection becomes complete, including a resolved
empty selection, the ordinary missing-dependency checks run. Known binding and
contract errors, malformed reference targets, original evaluator diagnostics
and external alias binding errors remain visible; neutral, warning and ignore
policies do not make incomplete models ready. Valid partial behaviors and
bindings remain available for inspection.

The native CEM-QL host and package lifecycle bridge use this assembly stage
before coordinated publication. Every selected owner still requires the
appropriate directed scope grant. Original arenas remain retained by result
handles; compilation does not inline declarations into the source tree or
write selected targets back into authored references.

Constraint collection references retain original source owners and lexical
aliases and bind behavior dependencies after the consuming behavior collection
is assembled. Unqualified behavior and diagnostic names stay local to the
consuming schema. Qualified behavior names keep their declaring aliases even
while another behavior collection is pending. Nested reference-resolution
constraint declarations and their source maps are projected from the original
arena. Ordinary constraint kinds retain last-kind-wins insertion order;
duplicate scope reference policy kinds remain compilation errors with the first
value retained.

A referenced scope policy is candidate schema data. Compilation uses the
request and destination limits supplied by the runtime for that evaluation;
a candidate policy cannot change an ongoing traversal budget. The runtime may
apply the completed schema policy at a subsequent evaluation stage. Pending or
invalid constraint selections keep the candidate inactive, and package
publication continues to reject hard compilation errors.

Field-contract collection references preserve every ordered application,
including repeated selections and duplicate contract names. Each application
keeps its original owner, nested choices, source maps and declaring aliases.
Its `@target` names an element in the consuming schema, so the same declaration
can apply independently in multiple consuming schemas. Behavior and diagnostic
binding occurs after their declaration collections are assembled; pending local
dependencies defer only missing-lookups, while independent declaration and
external alias errors remain visible.

Direct and referenced contracts with an absent local target report
`cem.schema_definition.invalid_field_contract` with `checkKind` set to
`field-contract-target` once element assembly is complete. Incomplete element
reference collections defer this missing-target check; resolved-empty selections
complete assembly and expose the error. This replaces the previous silent skip
for direct contracts as well. Invalid or incomplete candidate schemas remain
inspectable, and package refresh preserves the last complete active schema,
converters and artifacts until the candidate is ready and free of hard errors.

`schema::input_references::validate_structural_input_references` is an explicit
structural consumer. It retains original owner/node handles in a temporary
validation structure; those structure indices are execution-local and are not
source AST node IDs. Elements and text-like structural content are accepted;
document, attribute, error and non-retained runtime values are invalid targets
at the selecting reference. Element counts and sequences follow the existing
structural rules, retaining ordering and multiplicity. Targets keep their
original source maps while both parent relationships and selected-subtree
attributes are checked under the consuming schema.

Each authored structural reference starts one traversal request, as in the
existing declaration consumer. `resolve_reference_structure` descends into
selected element children under that same active reference stack, scope depth
and cumulative scope/work budgets. Owned containment consumes work without
incrementing reference-expansion depth. Reference → element → reference cycles
remain bounded; a crossing never replenishes work. Ordinary `resolve_reference`
keeps its terminal-only contract and does not allocate structural result data.
Unknown or opaque elements retain the existing structural validator's boundary;
references inside them are not eagerly evaluated by this consumer.

Results expose completeness separately from hard validation failure. Pending,
cyclic or limited child selection cannot appear to be a successful empty
selection, even under ignore disposition. Field contracts on an incomplete
child sequence are deferred; independent attribute checks and original
expression diagnostics remain visible. Nested incomplete children do not erase
their already-selected parent from an enclosing complete child sequence.
Resolved-empty selection is complete and ordinary missing-child checks apply.

The native QL host exposes `validate_input` against its current runtime contexts,
policies and directed grants. Independent hosts can validate the same retained
source with different selections, without source writeback or context IDs.
This API performs structural checks; it does not schedule engine requests,
create an expanded CEM document, or run whole-document behavior hooks against a
synthetic source. Engine lifecycle wiring, incomplete-report representation and
behavior-hook handoff over multiple original owners remain explicit follow-up
work.

### Native element inheritance during schema compilation

The schema-owned `element-base` datatype admits existing literal base names and
an explicit native reference constructor in element `@base`. Compilation checks
that a complete native selection contains exactly one named element declaration.
General expressions and composite slots are invalid at this composition site.
Native bases on other scalar declaration sites require separate contracts.

Source-only compilation retains native bases as pending and cannot activate the
candidate package. Explicit lifecycle compilation resolves the entire inheritance
chain under one request budget and the destination scope limits, with normal
scope-crossing grants and active-link cycle checks. Pending, unresolved, denied
and limited selections remain incomplete regardless of diagnostic disposition.
Malformed slots, wrong declaration kinds and invalid complete cardinality are
schema errors. Existing package readiness rules preserve the last complete active
package while an incomplete candidate remains available for inspection.

Inheritance uses original declaration owners and each declaration's lexical
aliases. Authored required attributes, optional attributes and child rules replace
the respective inherited fields, preserving existing literal-base semantics.
Referenced declarations do not implicitly import their schema's other collections.
The temporary compilation forest retains dependency owners without rewriting the
source AST or evaluating general attribute expressions.

### Native attribute datatype adoption boundary

Attribute declaration `@type={#datatype}` adopts an explicit native reference
selecting exactly one named schema `{type}` declaration, retaining its original
owner and lexical scope. Literal type names retain their existing behavior.
This adopts the reference site; it does not yet select executable datatype forms.

Until that consumer is implemented, compilation retains native type constructors
as pending and cannot activate the model. This applies to authored attributes and
attributes selected through declaration collection references. Local attribute
metadata stays available for inspection. General native expressions, composite
slots and missing native handles are invalid under the explicit-constructor
contract. Readiness guarding does not invoke a datatype evaluator or implicitly
turn an unconsumed native type into an untyped attribute.

General datatype compilation must be designed before enabling consumption;
the limited built-in-base/values-only consumer was not adopted. The temporary
[datatype compiler proposal](cem-datatype-compilation-proposal.md) inventories
existing kinds and rule descriptions, proposes retained descriptors and a shared
literal/native dependency lifecycle, and records the remaining semantic decisions.
Pending attribute metadata defers type-dependent facet/default checks, while
known malformed facets and local value errors remain visible. Standalone value
conversion rejects unconsumed type metadata instead of falling back to strings.

General datatype rule execution adopts explicitly registered schema-owned
implementations and preserves shipped prose as descriptions. It does not introduce
an executable grammar language. Registration grants no additional scope access,
and matching a rule description or local type name cannot acquire a primitive.
Custom binding adopts explicit native `@rule={#behavior}` selecting one named
registered datatype-compatible behavior, with literal descriptions retained.
A separate `@behavior` field is not adopted. Rule consumption remains disabled
until its typed signature and lifecycle adapter are specified. Rules validate
and report acceptance or diagnostics; an explicitly requested separate conversion
capability produces the canonical value. Rules cannot replace that value or
mutate authored source nodes. Distinct same-name datatype declarations in one
lexical schema are compilation errors; reuse of the same original declaration
and equal names in separate scopes remain valid. Scalar restrictions intersect
with their base and cannot widen it or rewrite the converted value. Explicit
declaration overrides remain deferred, with identity, authorization and dependent
activation contracts required before admission. Native selection is not redirected
by a later same-name declaration. Explicit list conversion adopts ordered typed
items with authored lexical source retained; canonical lexical output is separate
serialization. Order, duplicates and original native item identities must survive.
Conversion remains disabled pending adapters and parity fixtures. List datatype
declarations reject `values`; use item datatype restrictions and registered list
rules. Existing attribute-local `values` compatibility remains separate. Datatype
validation requires a registered compatible capability, rather than automatically
reusing diagnostic-only behavior execution. Completed rules return explicit
acceptance plus attributed diagnostics; every effective restriction must accept.
Severity remains independent of datatype acceptance, and pending/unavailable or
failed execution cannot establish acceptance. Diagnostic-only behaviors need an
explicit compatibility adapter; they do not acquire datatype capability implicitly.
General lists allow zero items unless an effective contract forbids it, preserving
shipped nonempty name-list rules. Attribute absence remains distinct from an empty
supplied list, and inherited nonempty restrictions cannot be removed. A dedicated
`node` datatype kind supports reusable retained-target contracts, preserving
existing built-in node semantics and rejecting lexical facets. Descendant references
stay retained until an explicit consumer resolves them. Symbolic-reference kinds
remain separate; neither they nor grammar rules imply URL lookup or scope grants.
Node rules validate the complete ordered target sequence once, including a
complete empty selection; incomplete resolution is not an empty successful result.
The original candidate remains distinct from the targets. Standalone value
consumers may use registered capabilities without a required candidate; capabilities
requiring one remain unavailable when it is absent. A supplied candidate must
always remain an original explicitly typed native node; synthetic ASTs and copied
candidate records cannot satisfy the input contract.
The verified inventory also distinguishes available scalar conversion from the
broader existing schema predicate coverage;
full datatype family parity remains an implementation requirement.

The retained datatype registry collection foundation is implemented with given
lexical scope handles, original declaration lookup and collision provenance.
Six native fixtures verify identity, scope independence and owner lifetime; 83
focused registry/inventory/value/declaration-reference tests pass. This registry
does not enable datatype execution or replace namespace admission and dependency
compilation. Derived declarations inherit an omitted `kind` from a resolved base,
requiring explicit list kind for item semantics. Retained source descriptors now
preserve original lexical scope and every authored field, including native base/rule
nodes and unknown or repeated fields. Four source fixtures and the adjacent tests
pass (87 total). Dependency inference and executable datatype consumption remain
guarded. Existing list-base item semantics are preserved; whole-list inheritance
awaits a separate explicit form. Retained source plans now distinguish inherited
bases, list items and native rule dependencies, preserve prose as descriptive
metadata and attribute malformed fields to original nodes. Five plan fixtures and
the adjacent regressions pass (92 total); planning does not evaluate references
or establish executable readiness. Inherited base compatibility must be declared
by registered contracts; matching value representation alone is insufficient.
Inherited-kind derivation reuses the resolved base contract, while additional
native rules and explicit cross-kind relationships require compatible registration.
Converter selection inherits the base converter unless an explicit compatible
derived converter is registered. Conversion remains an explicit consumer request
and runs once before all effective restrictions validate its result; unavailable
selected capabilities never trigger silent fallback. Scalar declaration `values`
uses registered datatype equality, retaining each inherited restriction's original
contract and binding. Equality cannot rewrite supplied values or implicitly invoke
conversion. Existing attribute-local vocabulary migration remains separate.

Bounded datatype dependency selection shares request/destination budgets, active
links and explicit grants across retained base/rule field containers. Native
references and literal QName dependencies use the same walk. Literal names pass
unchanged through an explicit lifecycle lookup hook with the original declaring
source and field, preserving that scope's aliases after imports. Consumer-owned
symbolic link metadata retains the original attribute; it does not create an AST
Reference or mutate the source. Pending lookup remains distinct from complete empty
selection, and missing declaring source context remains deferred. Thirteen traversal
fixtures and adjacent regressions pass (105 total). Selection completeness does not
establish executable datatype readiness.

### Reference and datatype workstream boundary

General datatype execution, conversion, equality and enumeration authoring are a
separate workstream in the [datatype proposal](cem-datatype-compilation-proposal.md)
and [datatype actions](todo.md#general-datatype-compilation-design). All adopted
contracts remain effective. Enumeration constant authoring remains an unanswered
future datatype decision, rather than the next decision for reference adoption.

Core reference work can continue with explicit consumer binding, lifecycle outcomes
and readiness boundaries. The particular executable native attribute `@type`
consumer remains guarded until its separately designed datatype compiler is ready.
Complete reference selection cannot by itself claim an executable type or activate
an incomplete schema package. Track the integration and its verification scenarios
in [reference adoption actions](todo.md#reference-adoption-dependency-binding-and-readiness).


### Retained expression artifact lifecycle (2026-10-05)

The explicit CEM-QL schema lifecycle host now retains compiled source expressions
by original occurrence and effective runtime scope. Standalone references and
native general attribute expressions share this path. Artifact inspection through
`compiled_source_expression` uses the original retained source handle and does not
compile or evaluate. Pending contexts produce no compiled artifact; malformed
source cannot register a valid one. Query-relative compilation provenance remains
available in the artifact, while the original occurrence retains its document source.
Precise diagnostic linkage now uses the retained format-neutral expression
embedding segments described below.

Each consumer invocation evaluates the compiled expression with the scope's current
context and capabilities. Evaluated target lists are never stored in this cache or
published onto authored reference nodes. A changing registered runtime capability
runs on every invocation, even while compilation is reused. Context replacement
invalidates all entries for that scope, including replacement with a pending context;
this avoids assuming compatibility of binding types, expected types or capabilities.
Equal expression text in distinct occurrences or child scopes has distinct entries.
Compilation reuse does not alter reference budgets, per-link grants or lexical
scope selection, and does not require an authored runtime-context ID.

Five new lifecycle/parity fixtures and the 13 existing CEM-QL schema-reference
fixtures pass, including replacement of a node-compatible binding by an incompatible
scalar type. Full Nx CEM-QL verification passes 862 tests (nine ignored), and the
transformation adapter target passes 132 tests. Namespace-aliased XML/CDATA references
compile and evaluate through
the same typed context as CEM-ML standalone references. This does not enable the
separately deferred XML attribute expression contract. The remaining source-position
linkage and expression-slot parity work is recorded with adjacent verification
scenarios in [todo.md](todo.md#reference-adoption-dependency-binding-and-readiness).


### Reference expression diagnostic provenance (2026-10-05)

Parser and import source maps now retain `ExpressionEmbedding` segments. Each
segment maps a UTF-8 byte range in the trimmed expression to its authored source
span and source identity. Equal-length segments map linearly. Decoded XML entities
and normalized CRLF segments retain their whole authored span; a selection crossing
text/CDATA boundaries retains separate source spans. XML interpretation occurs at
import, and the CEM-QL lifecycle adapter uses only this common metadata. The CEMB
codec preserves the additive source-map transform tag 14, and typed inspection
labels it `expression-embedding`. No authored reference or target edge is changed.

Compile and runtime failures from a local query retain their code, severity, message,
details and query-relative source frames, while gaining the original occurrence and
document URI. Their reporting byte offset uses the mapped authored position, and
line/column projection uses the retained document line index when source text was
given. EOF points refer to the end of the expression payload, rather than a following
CEM or XML delimiter. Already-attributed native diagnostics keep their original node,
document and coordinates. Compiled expression artifacts retain query-local provenance
for reuse; diagnostic linkage is applied for the current original occurrence.

Legacy/source-only ASTs without embedding metadata retain occurrence and document
attribution but leave precise document coordinates absent. Given maps can preserve
byte offsets without source text, but cannot fabricate line/column coordinates.
Missing metadata never turns a query-relative offset into a document-relative one.
Source-bearing imported expressions retain mapping through binary round trips.

Verification: 23 schema-reference lifecycle tests pass, including five new diagnostic
fixtures. Ninety focused CEM-ML tokenizer/parser/codec/native-slot/source-mapping
tests pass, including three new segment and binary fixtures. The complete CEM-QL
and transformation adapter Nx test targets pass.

Remaining expression-slot, lexical default/shadowing and tree-sitter parity work is
still tracked in [todo.md](todo.md#reference-adoption-dependency-binding-and-readiness).
XML attribute expression recognition, child override syntax and cem-element ID
consumption remain on their existing deferred tracks.
