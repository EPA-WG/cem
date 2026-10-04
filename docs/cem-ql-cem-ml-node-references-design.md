# CEM-QL / CEM-ML AST node references design

Status: accepted design, promoted on 2026-10-03. Adopted rules are normative; explicitly identified open decisions remain unresolved. Implementation is separate and is not claimed by this document.
Date: 2026-10-03.


## Adoption summary

The following rules govern implementation of the accepted portion:

- Reference is an AST node type designating node(s) through CEM-QL; targets are not copied or inserted.
- `#` has prefix-unary precedence 9, with existing tighter postfix/type/dot operations and right-to-left prefix nesting.
- `{#...}` and XML `<cem:expr>#...</cem:expr>` retain semantically equivalent typed reference nodes. No separate `cem:reference` vocabulary is introduced.
- The reference model uses existing contextual scope/binding associations and source-position semantics. Context closure resolves AST-specific context relationships; loading does not automatically execute reference selection queries.
- Any AST node kind may be a target. Self-reference and cycles are permitted as graph edges, without implicit recursive evaluation.
- Existing AST identity and graph serialization distinguish reference occurrences and preserve ordered target edges. No global ID service or persistent identity across reparses is required.
- `#` accepts node-valued results and performs no implicit string lookup or expression execution. Referencing an attribute node is distinct from interpreting its scalar value.
- Consumer interpretation and AST-change evaluation/update mechanics remain outside scope.

Promotion does not adopt the unresolved existing-reference preservation alternative or select a concrete storage layout, type spelling, target-access API, or new comparison operator. Those decisions remain explicitly open below. Where an exploratory alternative conflicts with the adoption summary, the adoption summary governs.
## Purpose

Introduce a reference as a CEM AST node type. A reference node designates another AST node or an ordered sequence of AST nodes through a CEM-QL expression. It preserves the relationship without copying the referenced nodes into the reference's location.

The reference is a general document-model concept. It is not limited to templates, elements, attributes, `datadom`, or a particular query root. A reference's targets and available context follow the ordinary CEM-QL evaluation contract.

This design covers reference syntax, the reference AST node, target selection through CEM-QL, identity, and context-specific binding resolution during parsing/finalization. AST loading retains selecting expressions; target selection runs only when explicitly evaluated. The use of references by any consuming system is outside this design. Re-evaluation after AST changes is also outside scope.

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
- The expression's evaluation-context identity and source provenance.
- Context-specific binding resolution and retention of forward selection expressions during parsing/finalization; no implicit query execution.
- Target identity, ordered multi-target results, and reference diagnostics.

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
    -> reference AST node
    -> initial target node sequence
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

Semantic parity is required, not assumed from the existing implementation. Both surfaces must preserve the same reference AST kind, typed target identities, target ordering and multiplicity, and resolution state for equivalent expressions and contexts. Each surface retains its own source provenance. XML ingestion must not stringify the reference result or copy its target subtree.

The structural/expression entry points are therefore:

- `{name ...}`: a structural node.
- `{$ ...}`: a general CEM-QL expression.
- `{#...}`: a CEM-QL reference expression, retaining its leading reference operator.

A separate `{cem:reference @select=nodes}` vocabulary is not introduced. It adds no required semantics to the adopted surface. Its potential introduction for a future concrete metadata requirement would be a separate decision; it is not needed for this design.

## Adopted precedence and open type refinements for `#`

The precedence rules below were adopted on 2026-10-03. The type rules and existing-reference behavior remain proposals for discussion. None of these additions are implemented. The precedence extends the current CEM-QL [Pratt precedence table](cem-ql-stack-design-impl.md), whose implementation has unary precedence 9, type forms 10, dot/pipeline 11, and calls/indexing 12.

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
| `##a` | `#(#a)` | Apply reference preservation to the inner reference |

Parentheses expose the boundary between selecting nodes and constructing a reference. Syntax must not change precedence based on inferred operand types. In particular, `#` should not silently swallow a lower-precedence traversal or union merely because its result might contain nodes.

To operate on the reference value itself, use a grouped result, such as `(#nodes).operation(...)`; whether that operation exists is determined by ordinary method/type rules. Ungrouped `#nodes.operation(...)` applies the operation to `nodes` first.

### Query type and AST node kind

A reference is an AST node kind. CEM-QL also needs to distinguish its typed reference value from an ordinary selected target node. Conceptual notation `Ref<T>` means a reference node whose target sequence contains nodes of type `T`; this notation does not prescribe the public type syntax or Rust representation.

Proposed construction rules:

| Evaluated operand | Result |
|---|---|
| One non-reference AST node of type `T` | One reference node with target sequence `[node]` |
| A sequence of non-reference AST nodes | One reference node with that ordered target sequence |
| Empty node sequence | One resolved reference node with zero targets |
| Existing typed reference `r` | Preserve `r` and its resolved targets; do not wrap it in a new reference layer |
| Scalar, string, record, or collection with non-node items | Type error unless an independently declared expression conversion first supplies nodes |
| A sequence mixing node and typed-reference values | Reject implicit flattening; an explicit expression must establish the intended target sequence |

An operand statically known to be incompatible is rejected by type checking. Dynamically typed operands receive the same check when initially resolved. There is no conversion of strings to ID lookups, target text, or browser handles.

A reference is not a subtype of its target type: `Ref<Element>` is not `Element`. It can be classified as an AST reference node for generic node-model operations, but this does not grant element attributes, child axes, or target traversal. Target access requires an explicit operation under the query contract; spelling that operation is not settled here.

### Existing reference as operand (open alternative, not normative)

Recommend semantic idempotence: `#r` preserves an existing typed reference's identity, target order, originating evaluation context, and resolution state. It neither executes its expression again nor resolves it in the current caller's context. In this unadopted alternative, applying `#` to an unevaluated reference would preserve that state; explicit query evaluation, rather than parse finalization alone, would produce target identities.

The new use-site expression still has its own source provenance. Preserving the reference does not overwrite where it was originally constructed. AST ownership and placement must represent reuse without reparenting or copying its target nodes; the exact expression-result representation remains an open question.

Distinguish reference preservation from a reference whose target is a reference AST node. `#r`, where `r` is a typed reference value, means preservation. A query deliberately selecting a reference AST node as an ordinary node could request a reference to that node. Whether and how CEM-QL exposes that node/value distinction is not yet settled; until it is, reference-to-reference construction should remain unsupported rather than be guessed from the same operand spelling.

### Empty results, truth, and equality

A zero-target reference is still a reference node; it is not automatically null, an empty query value, or false. `#a ?? b` therefore must not become a hidden fallback for zero targets. Query truth conversion must not follow targets implicitly.

Construction does not define a new equality operator. Reference-node identity and resolved-target-sequence equality are different comparisons. Ordinary identity can compare the reference nodes; any comparison of their targets must be explicit and must preserve order and multiplicity. The exact available comparison surface remains open.

## Reference AST node

Conceptually, a reference node carries:

| Field | Meaning |
|---|---|
| Node kind | Reference, distinct from element, text, and ordinary scalar nodes |
| Reference expression | The parsed CEM-QL expression designating target node(s) |
| Evaluation context | The context/bindings under which the expression is initially resolved |
| Target identities | Ordered identities of the selected AST nodes after resolution |
| Resolution state | Unevaluated; or pending, resolved, unresolved, or invalid for an explicitly requested evaluation |
| Provenance | Authored syntax, source range, and relevant expression diagnostics |

This is a semantic outline, not a proposed implementation struct. The exact placement of expression artifacts, context identity, and resolution annotations in the existing AST model remains open.

A reference node has its own AST identity and source location. Its targets retain their existing identity, ownership, and structural position. They do not become children of the reference node merely because it references them.

All AST node kinds are eligible targets, including document, element, attribute, text, and reference nodes. Consumer restrictions do not narrow the general reference node type.

## Existing representation and the remaining AST question

This section records inspected implementation surfaces and recommendations for discussion. It does not adopt a storage layout or authorize implementation.

### Existing layers

| Layer | Existing representation | What it does not currently establish |
|---|---|---|
| Parser document | `CemDocument` owns an arena of `CemAstNode`, addressed by `AstNodeId` | The inspected parser enum has no distinct reference variant |
| Parser reference slot | `NameSlot` contains an owner scope, target name, optional resolved node ID, and source map | A general CEM-QL expression, multiple targets, and the full reference-resolution contract |
| CEM-QL syntax | `Expression` represents names, paths, pipelines, operators, and other expressions with byte ranges | Retention of that expression on a parser reference node |
| CEM-QL context | `QueryContextScope` and query node views carry evaluation/access context; views expose representation and node identity | A portable capture of every evaluation binding on a reference |
| Native reference value | `CemReference<T>` retains an ordered target sequence; `ReferenceView` exposes it as a query node with `kind`, `targets`, and runtime identity | Original selecting expression, evaluation-context capture, or pending-resolution status |
| Constructed native content | `CemValueNode::Reference` contains a reference value and source map | A parse-time expression or pending state |
| Portable value graph | `CemValueRecord` supports `kind="reference"`, ordered `targets` indices, source information, and provenance | A serialized selecting query/context or an explicit parse-resolution state |

Relevant source files: [parser AST](../packages/cem_ml/src/parser.rs), [parser document](../packages/cem_ml/src/parser/document.rs), [native values](../packages/cem_ml/src/value.rs), [portable value graph](../packages/cem_ml/src/value/artifact.rs), [CEM-QL expressions](../packages/cem_ql/src/parser.rs), and [query reference view](../packages/cem_ql/src/eval/values.rs).

The existing native reference container is significant prior work. The design connects parse-time reference syntax to that semantic model rather than claim all reference representation is missing. However, this does not mean the adopted syntax or parse-resolution contract is implemented.

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

These are contextual references in the semantic model. The current implementation represents them through specialized frames, bindings, and slots; it does not uniformly expose them as a `CemAstNode::Reference` variant. Some declaration identities refer to context records rather than parser element nodes. The design must specify that mapping rather than equate all existing numeric handles with `AstNodeId`.

The common pattern is:

```text
reference occurrence + designation/expression
    -> context/binding association effective at that occurrence
    -> initial resolution against the available declarations/nodes
    -> typed target identity or a source-addressable resolution outcome
```

For a general `#` reference, CEM-QL supplies the designation/selection. For a namespace or schema-context reference, the existing construct supplies its own designation and target-kind constraints. Sharing the resolution model does not require giving each construct the same surface syntax or accepting every target kind.

Consequences for this design:

- Associate a reference with existing semantic context/binding identities; do not copy a complete evaluator environment into every reference node.
- Preserve the context effective at the occurrence's source position. Deferring initial target resolution until parse finalization must not silently use bindings that were introduced or rebound later at the same source scope.
- Reuse the existing scope chain and shadowing rules where the applicable context defines them. This does not introduce a template-only scope or singleton ID lookup.
- Distinguish binding resolution from query evaluation: fixing which declaration a name denotes does not necessarily mean that a forward target node sequence is already available.
- Use a shared initial-resolution outcome model, while keeping per-construct target kinds, cardinality, dependency readiness, and diagnostic policy explicit.
- A general expression selecting several nodes is a valid multi-target relationship; a namespace binding or active schema selection can retain its own single-target constraint.
- Finalize pending relationships at their declared dependency/context completion boundary. Do not force all contextual references to wait until document end when parsing needs them earlier.
- Preserve the established no-retroactive-rebinding principle for resolved contextual bindings. AST mutation and reference updates after finalization remain outside scope.

What remains open is the common AST/context representation and how its typed references connect to the specialized implementations. The design does not mandate replacing every frame or binding table with the same storage struct.

### Four different pieces of information

1. **Query expression:** what selects the targets. Preserve the parsed expression or a stable expression-artifact link and its source map; keeping only its rendered string loses type/source information. This is the expression describing construction, not a stored procedure that will be automatically rerun.
2. **Evaluation context:** the bindings/current item/document access available for that initial evaluation. Use the ordinary query context; do not introduce a template-only context. A scope number alone is not the complete set of bindings. Pending initial evaluation must associate with the existing binding/context state effective at the reference occurrence, including source-position identity when required, rather than copy a live evaluator or use the final mutable scope state. After resolution, a complete live evaluator/environment need not be kept just to identify targets.
3. **Target identities:** the resulting ordered node identities, distinct from the expression and from the reference node's own identity. Use owning-document/graph context with node handles. Native portable graph indices and parser arena indices are different representation-local handles; neither may be treated as a universal singleton ID.
4. **Resolution state:** distinguish a retained unevaluated expression from outcomes of an explicitly requested evaluation: pending, resolved, unresolved, or invalid. An empty target sequence cannot distinguish those conditions.

### Minimal conceptual form

```text
ReferenceNode
    own node identity
    expression artifact + source provenance
    evaluation status:
        Unevaluated(context association)
        Pending(requested evaluation)
        Resolved(ordered target identities)
        Unresolved(reason + diagnostics)
        Invalid(reason + diagnostics)
```

This is a conceptual tagged state, not Rust API syntax. It avoids treating both a pending slot and an empty resolved target list as the same `None`/empty value. It also avoids contradictory independent fields such as `state=resolved` beside an unevaluated expression marker.

The expression and provenance may be attached by artifact/annotation identity rather than duplicated inside each resolved native reference container. The resolved target sequence should have one authoritative representation; parser annotations, query views, and portable export should derive from it rather than maintain divergent copies.

### Identity and persistence limits

`CemReference<T>` currently uses shared container identity (`Arc`) for runtime reference identity. Cloning retains that identity without copying targets. That address-derived identity is not a portable identifier across serialization or processes. The portable graph carries its own reference record and graph-local target edges.

A serialized finalized reference principally needs its own graph identity, target edges, and provenance. Serializing a pending reference would additionally need a portable expression and sufficient initial-context information, or an explicit restriction against exporting that pending state. This design has not settled that transport requirement.

These representation choices concern construction and initial resolution only. Keeping expression provenance must not imply AST-change subscriptions, automatic re-evaluation, or any consumer-specific use of the reference.

## Target selection and context

CEM-QL supplies selection and context. The reference node does not introduce an independent template registry, a new DOM scope, a special ancestor-search rule, or automatic document-global lookup.

A supplied context can contain nodes or references made available by the normal query contract. Whether a query may traverse the current document, a nested structure, or an explicitly supplied document is governed by that contract, not by a reference-specific template restriction.

Construction rules proposed here:

- A node result can establish a target identity.
- A node sequence can establish an ordered target sequence.
- Non-node results must have an explicitly defined reference interpretation or produce an invalid-target diagnostic; strings must not silently become ID searches.
- An existing reference result must have an explicit preservation/dereferencing rule. It must not be reinterpreted from its textual spelling.
- Multiple selected nodes are a legitimate result, not inherently ambiguity. No first-match selection is implicit in `#`.

If the query contract reports missing bindings or an ambiguous expression, the reference preserves those diagnostics. This design does not impose single-target cardinality on all references.

## Identity and graph structure

Resolved targets are identified by their owning AST document and node identity, not only by lexical names, import IDs, or reference expression text. A bare arena index is meaningful only within its owning document.

Two reference nodes can designate the same target sequence while retaining different source locations and expressions. Equality of the reference nodes themselves must be distinguished from equality of their resolved target sequences.

The structural AST remains a tree or arena as defined by the existing model; references add non-owning graph edges. Following an edge is not structural child traversal. No implicit recursive dereferencing, subtree insertion, or cycle expansion is introduced.

Self-reference, cycles, and references to reference nodes are permitted as non-owning graph relationships. They do not imply recursive evaluation or expansion. Circular evaluation dependencies are distinct from graph cycles; their handling belongs to the explicit evaluation contract. The distinction between preserving a typed reference value and deliberately selecting a reference AST node as a target remains open.

Any portable representation of resolved references must retain adequate document/node identity. It must not serialize a live browser pointer or duplicate the target subtree. Detailed transport encoding remains an open representation question.

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

After initial resolution/finalization, consumer evaluation/re-evaluation and updating reference targets are outside scope. This design does not prohibit explicit later evaluation when a consumer supplies the required context.

## AST loading, context closure, and explicit evaluation (adopted)

Loading/parsing an AST does not by itself execute embedded reference selection expressions. Preserve the expression node, its provenance, and its applicable contextual associations.

Context closure remains the definite boundary for AST-specific context/binding resolution. It establishes the context associations needed to interpret the document. It is not a universal instruction to evaluate every embedded query and populate target sequences.

Distinguish:

1. Parsing and context resolution: establish syntax, lexical bindings, context identity, and relationships required to interpret the document.
2. Explicit query evaluation: execute a reference's selecting expression and obtain its target sequence when an evaluation is requested.

The evaluation request can come from a CLI, transformation, or other evaluator. Its expression can already reside in the AST; expression origin and execution trigger are separate. A caller may explicitly request evaluation during loading, but loading alone does not imply it.

Forward AST target nodes need not exist when the reference expression is first parsed. The AST preserves the expression without prematurely executing it against partial input. An explicit evaluator decides when its required context is ready and which completed AST/context to supply. This design does not add an automatic scheduler or re-evaluation protocol.

Context/binding associations must retain the established source-position and shadowing semantics. Deferring evaluation does not make illegal forward lexical declarations valid or silently substitute later bindings. Context closure must not discard the associations needed to interpret the retained expression when the active parser frame is popped.

At parse finalization, an unevaluated expression is a valid retained expression, not automatically a failed target lookup. Parsing errors, illegal bindings, and required contextual-resolution failures remain diagnostics under the applicable construct. A successful evaluation returning zero nodes is different from not having requested evaluation.

Later a consumer may supply the required context and explicitly evaluate or re-evaluate the expression. For example, cem-element could provide datadom to a CEMT transformation, which evaluates the retained reference expression. This is an example only; the consumer's scheduling, updates, output, and lifecycle mechanics remain outside scope.

A reference may target any AST node kind. Self-reference and cyclic target edges are allowed as graph structure. They do not require the parser or loader to follow those edges. A cycle of evaluation dependencies that prevents a requested evaluation from completing is a different issue from a valid cyclic target graph.

## Identity and graph preservation (adopted)

Reuse existing AST node identity and graph serialization to distinguish reference occurrences and preserve target edges. This minimal identity direction is adopted; no new wire format, equality operator, global identity service, or persistent identity across independent reparses is adopted.

Reference-node identity distinguishes two reference occurrences even if their selecting expressions or selected target sequences are identical. The existing AST node-identity machinery can provide this; no separate global reference naming service is implied.

Target identity distinguishes the selected node itself from another node with equal content. Two identical elements remain different nodes. Within one AST arena, a node handle is sufficient. Where evaluation explicitly uses more than one document, pair the handle with its owning document/graph context to avoid collisions.

Target-sequence equality, if needed, compares target node identities position by position, preserving order and multiplicity. `[A, B]` differs from `[B, A]` and `[A, B, A]`. This does not make two distinct reference AST nodes identical. It also does not require a new public equality operator in this design.

Portability is conditional. If only source expressions are serialized, their future targets are obtained by an explicit evaluation and no resolved target identity transport is required. If a finalized/resolved graph is exported, its reference edges must reconnect to the correct exported nodes when loaded. Graph-local indices and normal AST graph remapping can meet that requirement; globally persistent UUIDs, URLs, source hashes, and identity across independent reparses are not required by this design.

The minimal need is therefore preservation of node distinction and graph edges. Separate open choices are the precise query comparison surface and the binary representation. They should not force evaluation during loading or expand scope into runtime persistence policy.

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

`#` constructs a reference from node-valued results, with the proposed separate existing-reference preservation rule. It does not infer another node from the textual value of an attribute. Non-node designation requires an explicit expression/context contract before reference construction.

Keep `{#datadom.attributes.commandfor}` as a valid surface example, conditional on its operand type. If a particular context supplies only a scalar string, the syntax remains valid but construction requires an explicit interpretation/conversion outside the operator. No native browser semantics are settled by the attribute name.

## Consumer examples only

These examples illustrate possible uses; they do not define this design's scope or acceptance criteria:

- CEMT could materialize a reference to a template node as an ID of the corresponding projected element.
- A validator could examine the referenced nodes while preserving reference provenance.
- A document tool could display connections between referenced AST nodes.

The actual interpretation, transformation, traversal, output, and lifecycle behavior of each consumer belong in separate proposals.

## Open decisions (not adopted)

1. Review the proposed aggregate target-sequence type, idempotent existing-reference behavior, and explicit reference-node/value distinction above; settle the public type and target-access syntax. Unary precedence and grouping are adopted.
2. How does initial evaluation represent the source reference-expression entry point and the resulting reference AST node while target resolution is pending? The `{#...}` and XML `cem:expr` surfaces and their semantic parity are adopted.
3. The shared contextual-resolution model is adopted. Settle its concrete AST/context representation and linkage to existing specialized records and native/portable reference values.
4. Context closure resolves AST-specific context/binding relationships; loading does not automatically evaluate reference selection. Settle the representation of unevaluated expressions versus explicit evaluation outcomes, without introducing consumer scheduling.
5. Any AST node kind, self-reference, and cyclic graph edges are adopted. Settle the explicit distinction between preserving an existing reference value and selecting a reference AST node as a target.
6. Reuse of existing AST identity and graph serialization is adopted. Verify any missing graph-preservation guarantees separately; query target-sequence comparison syntax remains a separate decision.
7. Strict node-valued construction and the attribute-node/value distinction are adopted. Additional scalar interpretation or expression-valued attribute protocols are outside the operator and require a separate explicit contract; none is introduced by this design.

## Design authority and limits

The accepted contract consists of the reference AST node, strict node-valued construction, adopted precedence and CEM-ML/XML surfaces, shared context/binding resolution at context closure, explicit query evaluation, any-node target eligibility, non-owning cyclic graph edges, ordered target sequences, reuse of AST identity/graph serialization, and source-addressable diagnostics.

Consumer behavior and reference updates after AST mutation remain outside scope. This design does not define changes to CEMT lookup, cem-element, browser wiring, or reactivity.
