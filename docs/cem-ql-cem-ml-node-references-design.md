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

Concrete linkage between typed reference views and specialized records is
implementation work tracked in [todo.md](todo.md#ast-node-reference-implementation).
No replacement of every frame or binding table is mandated.

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
Reaching a limit or detecting a cycle stops that traversal. Diagnostic policy
does not permit unlimited traversal. Numeric defaults and schema policy
declarations are actionable work in
[todo.md](todo.md#ast-node-reference-implementation).

The applicable scope schema determines unresolved-link treatment:

| Schema requirement | Evaluation treatment |
| --- | --- |
| Mandatory resolution | Report failure through the scope's existing diagnostic and error policy. |
| Warning | Report the unresolved link and retain its unresolved outcome for the consumer. |
| Ignore | Tolerate the unresolved link without a diagnostic; do not fabricate a target or remove the authored edge. |

Apply policy to each link under its effective scope schema. A resolved empty
selection is distinct from an unresolved link; schema cardinality determines
whether an empty result is acceptable. Consumer-specific use of the resolved
nodes, including element-to-ID extraction, remains outside CEM-ML.

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

The CEMB AST codec version 3 carries retained expression/context/target state
and allows self-reference or cyclic target edges while retaining version 2
read compatibility. These graph links remain separate from structural child
edges. This does not change the older native output-value artifact's expansion
and consumer projection contracts.

Adoption does not claim that these initial representations implement every
2026-10-04 rule. The implementation checklist owns alignment and verification;
cyclic graph/native output transport compatibility remains explicitly deferred.

The schema now declares standard traversal limits of 128 reference links per
path and 100000 work units per resolution. `ReferenceTraversalLimits` reads
the embedded schema defaults and validates effective scope overrides, retaining
inherited bounds when a scope omits them. This prepares consumer policy;
shared chain traversal and unresolved-link disposition are still implementation
work. XML reference ingestion also retains query-payload source spans when
folding text or CDATA into the retained node.
