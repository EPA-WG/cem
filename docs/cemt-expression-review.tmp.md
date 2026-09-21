# CEMT expression insertion — temporary proposal review

Implementation update: module hook defaults/caller inheritance, receiver input
contracts and native XPath projection are implemented. The maintained
[contract](cemt-native-values.md) describes their behavior. Portable artifacts
now write version 2 to distinguish root output occurrences from source-target
references, while retaining version-1 reads. Explicit CEM-QL template dispatch,
remaining scope/resource accounting and DX review continue in TODO.


Status: **accepted direction; implementation and verification in progress**.

R08 is resolved: portable native CEM value artifacts carry typed attributes and
reference relationships across workers, fallback and saved pipelines. The
maintained contract is [Native CEMT values](cemt-native-values.md); the
[TODO checklist](todo.md) records unfinished implementation and coverage.

This document collects the expression-insertion discussion so individual topics
can be reviewed and revised without treating an earlier conversational plan as
final. Record selected directions separately from proposed mechanics and open
questions. Documentation and implementation follow resolution of the contract.

The revision below replaces **“nodes produce copied output subtrees”** with
**“nodes reuse retained content.”** The implementation now includes retained references, scoped expression hooks
and native attribute values; verification status lives in the checklist.

## 1. Purpose and current implementation

The initial example is the Pokémon cell override in
[cell-overrides.html](../packages/cem-elements/demo/cell-overrides.html). Its
template receives the retained `name` element and finds the sibling `id` using
CEM-QL. The discussion concerns what `{$node}` should mean, both in the cell's
content and in an attribute such as `@alt="{$node}"`.

Implementation facts at the start of this review (historical baseline):

- The recent interpolation change extracts node text in body and attribute
  contexts. It does not insert a subtree.
- Explicit native result construction currently recursively materializes
  render-plan subtrees. Reference-based insertion will require an architectural
  change, not just a wording change.
- Retained source trees use shared owners and expose read-only node access.
  [AC-Q-2](./cem-ql-ac.md#0-cross-cutting-requirements) requires evaluation to
  leave the host AST unchanged. Output construction does not mutate that source.
- CEM AST attributes currently have string values. The render plan additionally
  carries a typed `value_stream`, but mixed attribute content is flattened early.
  This is insufficient for retained rich values across a transformation pipeline.
- There is no uniform public CEM-QL node-text helper or shallow/deep-copy helper.
  `dom:children` exists, and retained parent/child navigation has been implemented.
- Current template match rules are collected globally; the proposed expression
  hooks require a separate, genuinely scoped dispatch mechanism.

Relevant implementation references:
[retained CEM tree](../packages/cem_ml/src/parser/tree.rs),
[interpolation](../packages/cem_ql/src/render/interpolation.rs),
[native result construction](../packages/cem_ql/src/render/construction.rs), and
[rendering](../packages/cem_ql/src/render.rs).

## 2. Selected directions

These directions were selected during the discussion. The detailed mechanics in
later sections remain subject to review.

1. **Typed insertion by default.** A body expression may produce nodes or atomic
   values. Changing the value's type may intentionally change output structure.
2. **Reuse immutable nodes.** Node-valued expressions reuse retained CEM nodes
   and their subtrees without duplicating subtree storage. Source identity and
   provenance are preserved. Deep cloning is an explicit operation.
3. **Read-only source.** CEMT transformation does not modify the retained source
   data tree. New surrounding output structure does not require changing reused
   source content. Final browser DOM updates are a separate boundary.
4. **Explicit text extraction.** Provide a CEM-QL operation when an author wants
   text rather than structural insertion.
5. **Explicit expression hooks.** Use `@on=expression` and
   `@into=content|attribute`, independently of ordinary template modes such as
   `cell`.
6. **Whole-sequence input.** A hook receives the complete expression result,
   rather than being invoked separately for each item.
7. **Inherited rendering scope.** Active hooks govern subsequent siblings and
   their descendants, including called/imported templates. Nested declarations
   can override the inherited behavior.
8. **Typed attributes and native handoff.** Preserve native rich values in
   intermediate CEM trees and across CEM component boundaries. Serialize for an
   actual DOM attribute or external output at the applicable final boundary.
9. **Explicit return contracts.** A declared return type requests a defined
   shared conversion followed by validation. Types may include numeric,
   constrained string and temporal values; insertion destination alone does not
   determine the return type.
10. **Resume focused tests for implementation.** The earlier testing deferral was
    lifted for this work. This document-writing task does not itself start tests
    or implementation.
11. **Prefer semantic CEM names.** Do not adopt `copy`/`copy-of` as public
    CEM-QL names merely to mirror XSLT. Describe reference construction, cloning
    and element construction directly. The revised candidate names are below.
12. **General attribute value contracts.** Attribute handling must understand
    numbers, dates/times, constrained strings and other schema-defined values,
    as well as structured content. HTML is one representation example, not the
    boundary of this capability.

All external document syntax continues to resolve only at the
[CEM-ML import boundary](./cem-data-import-principle.md). Reusing content is not
permission to introduce JSON objects, browser DOM substitutes or format-specific
evaluation branches.

## 3. Reuse, occurrences and explicit copying

Proposed replacement wording:

> Node-valued expressions reuse retained CEM nodes and their subtrees. Insertion
> preserves source identity and provenance without duplicating subtree storage.

An output occurrence identifies **where** reused content is inserted. Inserting
one source node twice requires two output occurrences, but does not require two
copies of its retained subtree. Occurrence metadata must remain distinct from
source node identity. The proposed explicit reference is itself a first-class
construct, not just an undocumented shared-pointer implementation detail. Its
relationship to output-occurrence identity is part of R01/R02.

Destination-specific namespace adjustments or presentation metadata must not
modify the source. They belong to output construction or final projection. A
later transformation can construct a replacement while reusing unchanged
content; replacement is not mutation of a shared source node.

Reuse avoids unnecessary deep-copy traversal and allocation. It does not remove
the cost of eventually visiting, validating or serializing the output. Browser
DOM creation also creates separate DOM nodes for separate output occurrences.

Explicit copying still has a purpose when independent identity or storage
ownership is requested. A small reused subtree can otherwise retain its entire
source document owner. Copying should permit independence from that arena while
retaining the provenance required by the source-map contract.

### Proposed CEM-QL operations

These names and detailed signatures are proposals, not claims of existing APIs.

| Operation | Intended behavior |
| --- | --- |
| `dom:reference(values)` | Construct an explicit retained reference to an ordered value sequence, without cloning its node targets. This is the explicit counterpart of default `{$values}` insertion. |
| `dom:text(values)` | Extract node string values and atomic lexical values in order; proposed default is concatenation without inserted separators. |
| `dom:text()` | Proposed context-input overload: extract text from the active query focus. At expression-hook entry, that focus represents the complete evaluated expression result. |
| `dom:clone(values)` | Explicit deep clone with fresh native node identities and independent subtree storage; retain source provenance. Atomic values remain values. |
| `dom:element(elements)` | Construct a new empty element for each supplied element, using its expanded name and namespace information. Do not inherit its ordinary attributes or children. This constructs elements; it is not a filter that selects existing elements. |
| `dom:children(node)` | Select existing children without copying them. Inserting the result reuses those children without the parent wrapper. |
| `cemt:apply_templates(values, mode)` | Invoke matching templates through the active CEMT host and return native result content. |

`dom:clone` replaces the earlier `dom:copy_of` candidate. `dom:element` replaces
the element-shell use case previously described as `dom:copy`. Neither old name
is proposed as an alias. Element construction accepts element inputs, rather
than giving text, attribute and document inputs unrelated shallow-copy behavior.
Construction from an explicit name and supplied content is a possible overload;
its signature remains an R11 review item, not an approved API.

Prefer `dom:reference` as the canonical spelling because it states the operation
in full. `dom:ref` is a possible abbreviation, not a second operation; no alias
is selected yet. A reference preserves target order and repeated targets. Its
own identity/provenance must be distinguishable from the identities/provenance
of the referenced nodes.

Default insertion and explicit reference construction should use the same native
reference mechanism. This equivalence describes the default expression behavior;
the scoped expression hooks remain independently applicable. Specify handling
of already-referenced values so `{$dom:reference(values)}` does not accidentally
create redundant wrapper layers. This remains part of R01/R05.

Default `{$node}` insertion does not call `dom:clone`. Neither reference
construction nor cloning automatically performs template matching. Constructing
an element shell and explicitly dispatching its selected attributes/children
can preserve surrounding structure while overriding selected descendants.

Template dispatch remains a CEMT capability. Its proposed CEM-QL entry point is
host-provided and must fail explicitly when no applicable template context exists.
It must not make ordinary CEM-QL evaluation depend on a browser renderer.

### Validation against existing reference documentation

The existing documents support the reference model, but distinguish two levels:

1. **A query instruction is a node in the CEM-QL tree.** The
   [CEM-QL IR design](./cem-ql-stack-design.md)
   describes a tree-shaped instruction graph. The
   [concrete IR shape](./cem-ql-stack-design-impl.md)
   includes binding/function/template references and a `Reference` instruction.
   The corresponding variants exist in
   [the implementation](../packages/cem_ql/src/ir.rs).
2. **Referenced source content is not cloned into the referencing tree.**
   [AC-F-5](./cem-ml-ac.md) explicitly requires references without
   cloning. The
   [CEM-ML stack design](./cem-ml-stack-design.md) likewise says reference slots
   bind to their resolved targets instead of cloning referenced content.

The existing `Reference` instruction specifically means following `.target` on
a node carrying a document reference slot, as defined by
[AC-QD-4](./cem-ql-ac.md). It is a reference-resolution operation, not an existing
general-purpose `dom:reference(values)` constructor. The current documented
runtime item kinds do not separately define a general output reference node
holding a value sequence.

Therefore the user's model is consistent with the existing principles, and
reference instructions already are CEM-QL tree nodes. The constructor and its
first-class retained result still require an explicit contract. Do not silently
repurpose the `.target` instruction or claim the output-reference API is already
implemented. Extend the common CEM node/value model rather than introduce a
parallel renderer-only DOM.

Review must distinguish the constructor's query-IR node, the retained reference
it produces, its targets and its occurrences in output. It must also distinguish
references to already evaluated immutable values from the existing unresolved
forward-reference slots. Whether target access follows automatically or uses an
explicit resolution step belongs in R02/R09.

## 4. Expression hooks and return contracts

Proposed syntax, not currently supported:

```cem-ml
{template @on=expression @into=attribute
    @match='context.attribute.name == "count"'
    @returns=integer |
    {param @name=value}
    {param @name=context}
    {body |
        {result-sequence @select=value}}}
```

The expression is evaluated once. `value` receives its complete result, including
an empty result. The proposed `context` describes the containing element,
insertion target, attribute name when applicable, content type and destination
type contract. Expanded names must be available where namespace distinctions
matter.

### Compact hook form and implicit input

Proposed preferred form for applying text extraction to all attribute expression
insertions in a scope:

```cem-ml
{template @on=expression @into=attribute | {$dom:text()}}
```

The short form does not need `param`, `body`, `match` or `returns` declarations:

- `value` and `context` are implicit bindings supplied by the special expression
  hook signature. Explicit parameter declarations remain available for contracts
  and documentation, but are not required just to access these bindings.
- Direct template content is its body. The existing template renderer already
  recognizes this shorthand for ordinary templates; expression hooks must retain
  it rather than require an extra `body` wrapper.
- Omitted `@match` means all expression insertions for the selected `@into`
  target, subject to the hook's scope and precedence.
- `dom:text()` reads the current expression input, not the containing element,
  the attribute's name or a previously bound cell-template `node`. At hook entry
  it is equivalent to `dom:text(value)`.
- A single node input yields that node's text. Multiple input items contribute
  text in order; the helper must not silently take only the first item. Empty
  input produces the empty string. These preserve the selected whole-sequence
  hook contract.
- Omitting `@returns` does not disable destination validation. This helper
  already produces text; an explicit return contract remains available when
  conversion or a constrained type is wanted.

For the single-node case, `<name>ivy<em>saur</em></name>` therefore supplies
`ivysaur`. An equivalent explicit-input short form is:

```cem-ml
{template @on=expression @into=attribute | {$dom:text(value)}}
```
Once proposal approved for implmeentation, add to todo list the matching short template version for other template types.

The proposed overload belongs to CEM-QL's host-provided focus mechanism, not
string substitution by the template compiler. Existing CEM-QL documentation
defines a current item inside pipeline steps; the current evaluator and native
function interface expose that item. It does not yet define a zero-argument
`dom:text` or a whole-sequence expression-hook focus. The latter must be added
explicitly, potentially through the proposed first-class reference to the
expression result, rather than changing the meaning of existing pipeline focus.

Refined focus proposal: the current template input supplies the base focus.
A template selected for a `name` source node can therefore use `dom:text()`
without declaring or passing `node`. Here, "current template node" means the
invocation's data node, not the CEMT declaration, currently executing instruction
or output element under construction.

Treat this as a stack of evaluation contexts rather than a fallback search for
a variable named `node`:

- Template dispatch establishes its selected input as the base focus.
- An expression hook pushes its complete evaluated input as a nearer focus.
- A nested pipeline step pushes its current item; leaving the step restores
  the hook or template focus. Leaving the hook restores the template focus.
- `dom:text()` reads the nearest established focus. `dom:text(value)` remains
  the explicit way to name the hook input even inside a nested pipeline.
- An explicitly empty input is still an established focus and produces empty
  text. It must not fall through to an outer template node. Invalid input also
  must not trigger fallback.
- A context with neither a template input nor a nearer focus reports missing
  context. It must not infer a document root or read template source text.

Named-template calls need an explicit rule: the proposed default is to inherit
the caller's focus, with rebinding only through a declared input mechanism.
Passing an ordinary parameter named `node` must not silently change focus.
This distinguishes semantic context from variable spelling and avoids changing
behavior when parameters are renamed. These rules remain an R12 review proposal.

The current implementation binds `node` during match dispatch but initializes
query `current_item` separately. The proposal requires a shared invocation-focus
contract; it is not already implemented by the existing `node` binding.

For this compact syntax to work, a direct hook-return expression must be captured
as replacement content rather than sent through the same automatic hook again.
It must preserve native result types instead of converting the result to text
merely because `$` syntax was used. This is the proposed direct-return rule for
R05; recursive expressions in constructed content and template calls still need
their dispatch/delegation rules reviewed. The long `result-sequence` form remains
an explicit emission alternative.

Keep three concepts separate:

| Declaration | Responsibility |
| --- | --- |
| `@into` | Select body-content or attribute-value insertion. |
| `@returns` | Request conversion to, and validation against, an explicit result type. |
| Destination attribute contract | Define value type, constraints and content representation, including eventual projection. |

Reuse the existing CEM type and schema machinery, including named constrained
types, regex patterns and numeric bounds. Define temporal conversions without
implicit locale or timezone guesses. A hook cannot weaken the destination's
schema contract. The exact conversion matrix and timing remain review topics.

Proposed mechanics to review:

- Omitted `@match` means true; omitted `@returns` preserves typed content subject
  to the destination contract.
- Nearest applicable scope wins; unmatched cases fall through to outer scopes.
  Priority and declaration order resolve candidates within a scope.
- Imported module-level hooks provide defaults below active caller overrides.
  Declarations encountered inside a called body establish a nearer scope.
- A hook's direct `result-sequence` output bypasses automatic expression hooks,
  providing a way to emit a value without recursive redispatch.
- Actual expressions inside hook bodies require a defined recursion and
  delegation contract; no implicit unbounded self-invocation is acceptable.

## 5. Attribute value contracts and transformation pipelines

An attribute should retain an ordered native value sequence, including literal
segments, atomic values and CEM nodes. Preserve its type/content-type contract
alongside that sequence. A string projection must not replace the authoritative
content prematurely.

An attribute's contract covers more than a markup content type. Model these
related dimensions explicitly, using shared schema/type machinery:

- **Value type:** string, integer, decimal, boolean, date, time, date-time,
  node content or another supported schema-defined type.
- **Constraints:** regex pattern, bounds, precision, length, enumeration,
  cardinality and other restrictions supported by the shared schema.
- **Representation:** a declared lexical format or registered content type,
  such as plain text, HTML, XML or CEM-ML, when applicable.

| Attribute example | Native contract | Projection responsibility |
| --- | --- | --- |
| Count | Integer with a minimum/maximum | Serialize the validated integer when the destination requires text. |
| Amount | Decimal with precision/range restrictions | Use a declared numeric representation; do not infer locale formatting. |
| Date or timestamp | Date/time value, or a string with an explicitly declared temporal lexical contract | Validate/convert using that contract and serialize its specified representation; do not invent a timezone. |
| Product code | String constrained by a regex or named schema type | Validate the pattern and retain the string. Pattern validation does not imply rewriting it. |
| Description | Retained node content with an HTML representation | Hand off native content to CEM consumers; serialize markup at a string boundary. |
| Structured payload | Retained CEM content with another registered representation | Use the declared shared projection/import capabilities, not attribute-specific parsers. |

The destination supplies this contract even when no expression hook declares
`@returns`. A hook may request an explicit conversion, but its result still has
to satisfy the attribute's own contract. The hook context should expose the
resolved type, constraints and representation so rules can match attribute
semantics rather than maintain lists of attribute names.

Numbers and temporal values should remain typed native values across CEM
component handoffs; constrained strings remain validated strings; structural
values retain native nodes/references. The fact that an eventual HTML DOM
attribute is a string does not make every intermediate attribute value a string.
Type conversion/validation and final serialization are separate phases; their
precise scheduling and declaration syntax remain R06/R07 review topics.

Ordinary HTML attributes ultimately need text. Explicitly HTML-valued attributes
need markup serialization followed by the outer attribute's escaping. CEM
components receiving rich values should receive retained native content rather
than serialize and re-import it between components.

Proposed constructor syntax for review:

```cem-ml
{attribute @name=description @content-type="text/html" |
    {b | Important}
    {br}
    Further details}
```

Quoted expressions remain useful for short cases such as `@alt="{$node}"`.
The destination's output context determines its eventual projection. A plain
string containing angle brackets does not become structure by shape inference;
actual external markup is imported through CEM-ML.

The intermediate representation must support further native queries and
transformations. Its relationship to attribute navigation, XPath's string-valued
attribute model, artifact persistence and the final DOM boundary needs explicit
resolution before implementation.

## 6. Review register

Use these identifiers when discussing or closing individual topics.

| ID | Topic | Question or remaining decision |
| --- | --- | --- |
| R01 | Node reuse and identity | Selected: reuse by default. Define the first-class reference constructor/result, output-occurrence identity and target identity, including already-referenced values. |
| R02 | Parent/navigation context | A source handle keeps its source parent. How does a node selected from intermediate output expose its output parent, order and repeated occurrences without modifying the source? |
| R03 | Ownership and lifetime | Specify retained-owner lifetime, explicit-copy independence and the source-map resources a copy still retains. |
| R04 | Hook scope and precedence | Resolve module defaults, caller scopes, local declarations, lexical bindings, scope exit and ties. |
| R05 | Hook return/recursion | Review compact direct-return expressions without automatic redispatch; define other recursive expressions, delegation to outer behavior and failure propagation. |
| R06 | Types and conversion | Specify types/cardinality, conversion timing, destination constraints, regex semantics and temporal values without premature flattening. |
| R07 | Attribute value contracts | Resolve type/constraint/representation declarations, schema inheritance, native value access and component handoff for scalars, constrained strings, temporal and structured values, including final projection. |
| R08 | Pipeline representation | **Selected:** portable native CEM value artifacts, preserving types, references, source metadata and native attribute content across workers, fallback and saved pipelines. See [the maintained contract](cemt-native-values.md#portable-artifacts-accepted-r08-transport). |
| R09 | Query/XPath behavior | Define source versus output navigation, reference target access versus existing `.target` resolution, explicit-clone semantics, XPath projection and the CEM-QL template-host interface. |
| R10 | Resource controls | Apply host-defined limits, child scopes that can only lower them, cancellation and scope access checks without publishing partial output. |
| R11 | DX and migration | Review `dom:clone`, `dom:element` and `dom:reference` signatures; decide whether `dom:ref` is useful. Audit current implicit-text consumers and distinguish implemented behavior from proposed behavior. |
| R12 | Implicit hook input | Review template-input base focus, whole-sequence hook focus, nested pipeline focus, empty versus absent input, named-call inheritance, and optional parameter/body syntax. |

## 7. Documentation, examples and verification work

After the review is complete:

- Promote the accepted contract from this temporary review into maintained
  documentation. Update CEM-ML syntax, the CEM-ML/CEM-QL acceptance criteria,
  shared value/type and scope descriptions, package references and the browser
  component handoff documentation.
- Add actionable implementation and fixture items to [TODO](./todo.md), including
  **DX polishing for text extraction, references, element construction, cloning
  and explicit template dispatch**. Reconcile earlier entries that still say testing is
  deferred.
- Update [cell-overrides.html](../packages/cem-elements/demo/cell-overrides.html)
  with separate lessons for default node reuse, explicit text extraction and
  scoped expression overrides. Preserve the simple `name` cell match and
  sibling-ID lookup.
- Include numeric, date/time, regex-constrained string and rich-description
  attribute examples showing native values across intermediate stages,
  destination validation and the final projection.
- Start with focused native tests, then verify affected WASM/browser consumers.
  Cover immutable source trees, repeated reuse without deep copying, occurrence
  navigation, explicit-copy identity/lifetime, mixed content and namespaces,
  hook scoping/imports, typed conversions, rich attribute handoffs, artifact
  round trips, cancellation and lowered resource limits.
- Validate XML/JSON/YAML/CSV through their common CEM import result; do not add
  format-specific evaluation or rendering paths.

Unrelated viewer features remain outside this increment. Storybook stabilization
remains after the immediate cell/expression work. The temporary review document
does not authorize implementation of unresolved design choices.
