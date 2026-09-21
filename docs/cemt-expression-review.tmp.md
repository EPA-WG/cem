# CEMT expression insertion — temporary proposal review

## Compact module template bodies (R11/R12)

**Selected by the user on 2026-09-21: align shared module preflight with compact
named/matching template bodies.** CEM-ML now collects parameters, calls, let
bindings and encoding expressions from direct bodies using the existing subtree
walker. Entrypoint visibility, module defaults and transform-function bodies
retain their existing rules. A shared layout check rejects mixed direct output
and explicit bodies, and duplicate bodies. Parameters and whitespace are allowed
beside a wrapper; output comments count as content. Invalid declarations do not
render an arbitrarily selected body.

The inline Pokémon rule now uses its implicit `node` and direct content. The
maintained [contract](cemt-native-values.md#compact-template-bodies), native schema
and CEM transform schema describe both forms. Native parser, renderer, adapter,
WASM preflight and browser verification are tracked by `CEMT-COMPACT-TEMPLATES`.
Render engine 1.5.6 invalidates earlier previews.

Verification passed: 23 focused CEM-ML tests, 133 CEM-QL tests, 96 adapter tests
and workspace test compilation; build, typecheck, lint (two existing warnings),
408 runtime unit tests, the compact-body WASM fixture and both portable-value
transport fixtures. All three browser stories and focused standalone/source-loaded
interactions pass, together with real-card checks at 1440px, 390px and 320px.
Two older manual render-plan fixtures now populate native attribute streams;
their expected output and source-map assertions are unchanged. The immediate
cell/expression/DX work is complete. Deferred Storybook stabilization is next.

Original review and reproduction:

The 2026-09-21 DX audit found a parser/renderer disagreement while simplifying
the inline Pokémon rule. Removing its redundant `param @name=node` works.
Removing its `body` wrapper also passes the nine native cell-rendering tests,
but the browser's shared native module preflight rejects the declaration before
rendering. Restoring only the wrapper restores both Pokémon images. The demo
keeps that working form while this syntax choice is reviewed.

The minimal form under review is:

```cem
{module |
    {template @name=label @mode=cell @match=true |
        {td | {$node}}}
    {body | {apply-templates @select=node @mode=cell}}}
```

`compile_template_module_closure` accepts direct template content.
`templateModuleImports`, which calls CEM-ML's
`parse_cem_native_template_module_options`, reports fatal
`cem.transform_template.declaration_unsupported` for `td`; the real Pokémon
rule also reports it for `variable`. `lower_template` currently accepts only
`param` and `body` children of named module templates. Expression hooks are
already excluded from named-entrypoint validation and support direct bodies.
This is a shared declaration grammar choice, not a viewer presentation change.

| Direction | Benefits | Costs |
| --- | --- | --- |
| **Recommended: align shared module preflight with the renderer's compact named/matching template bodies.** Keep explicit `body` valid, separate `param` declarations from direct content, preserve entrypoint visibility and expression collection. Define and reject ambiguous mixtures of direct content and explicit bodies. | One template-body convention works with and without imports; the Pokémon rule can use its implicit `node` and a direct body. | Extends the shared module declaration grammar and requires native parser, adapter, WASM preflight and browser regressions. |
| Keep explicit `body` mandatory in named module templates. | Retains the strict declaration/content boundary and needs no new syntax. | The compact form remains context-dependent; direct module compilation must also reject it so native and browser behavior agree. |

The reference-constructor increment kept the explicit wrapper. The user's
selection above authorizes the grammar alignment; `CEMT-COMPACT-TEMPLATES`
in [TODO](todo.md) tracks implementation and verification. Storybook stabilization
remains after the immediate cell/expression work.

## Constructor reference inputs (R01/R09/R11)

**Selected by the user on 2026-09-21: preserve reference nodes.**
`dom:clone` now preserves the selected reference node and shared target graph;
`dom:element` requires explicit `.targets` selection. The concrete-reference
unwrapping path has been removed, so direct, constructed and portable values use
the same graph constructor. Cloning also checks host access before detaching
ancestors; a new native fixture reproduced and closes the prior scope bypass.

Six native cases cover reference kinds, repeated target aliases, empty/nested
references, scalar and invalid shell inputs, detached ownership, internal
parents, provenance, XML/JSON/YAML/CSV imports, lowered child limits and
cancellation. Worker, saved-artifact and fallback checks verify the same
constructor results before and after transport. Render engine 1.5.5 invalidates
older previews. The maintained [operation guide](cemt-native-values.md#choosing-a-native-value-operation)
explains explicit target selection without introducing copy aliases or overloads.

Verification passed: 120 focused CEM-QL tests, 96 adapter tests and workspace
compilation; build, typecheck, lint (two existing warnings), 408 runtime unit
tests, both WASM transport fixtures, all three cell browser stories and focused
standalone/source-loaded interactions. The three real demo cards also pass
1440px two-column and 390px/320px containment checks. Base viewers are unchanged.
At that checkpoint, the compact module-body choice above was the remaining DX item.

The original reproduction and alternatives below remain as review history.
Implementation and verification are tracked by `CEMT-CONSTRUCTOR-REFERENCES`
in TODO.

Original decision review:

The 2026-09-21 DX audit reproduced a transport-dependent public result in
`dom:clone` and `dom:element`. Implementation was paused under the user's
stop-at-decisions instruction: the existing signatures described nodes and
element inputs, but do not settle whether these constructors follow a reference
or operate on that first-class node itself.

The input is one explicit reference with two occurrences of the same target:

```cem-ql
let name = data:read("<r><name>ivy</name></r>", "xml").root.children.children;
dom:reference((name, name))
```

Two temporary native probes evaluated that value directly and after
`encode_values` / `decode_values` using the default CEMV limits. Both equality
checks failed:

| Operation | Direct reference | After CEMV round trip |
| --- | --- | --- |
| `dom:clone(values)` | Two element roots, each with a distinct clone identity. | One reference root; its two targets share the identity of one cloned element. |
| `dom:element(values)` | Two new empty element shells. | No result; `cem.ql.type_error`: `dom:element requires element nodes`. |

`eval/values.rs::clone_item` recursively follows only the concrete
`ReferenceView`. Portable `GraphView` references instead reach the native graph
clone or the element-kind check. Constructed output occurrences also use a
different native view, so the eventual regression matrix must include them.
Transport must not determine result kind, cardinality or repeated-target aliasing.

| Direction | Benefits | Costs |
| --- | --- | --- |
| **Recommended: preserve first-class reference nodes.** `dom:clone(reference)` clones the reference and its reachable target graph; repeated target links within that graph still alias. `dom:element` accepts elements only; use `reference.targets` explicitly for its target elements. | Matches the user's first-class reference model and the portable graph contract. `clone` preserves the selected node's kind and internal sharing; target selection stays visible in the query. | Changes current direct-reference behavior. Callers wanting a result for each target must write `dom:clone(reference.targets)` or `dom:element(reference.targets)`. |
| Follow reference targets consistently in both constructors. Recursively unwrap references and construct a result for each target occurrence. | Preserves current direct-reference convenience; both constructors work on references without an explicit selector. | `clone` changes a reference node into its targets. Repeated targets become independent clones, and reference wrappers/alias relationships disappear; portable behavior must change. |

The recommendation follows the user's requirement that a reference is a node
in its own right. `dom:text(reference)` can still extract its target text, and
body insertion can still render target content; those are presentation
operations, not evidence that cloning should erase the reference node.

Under the recommended rule, cloning each item in an input sequence remains an
independent operation. `dom:clone((name, name))` therefore produces two independent
element clones, while `dom:clone(dom:reference((name, name)))` produces one cloned
reference whose repeated targets share a cloned element. Selected roots are
detached; source provenance survives, and the original tree remains immutable.
Neither option requires an external-format branch, a new artifact version,
new aliases, or changes to the data-table/XML viewer templates.

After selection, implement one rule for direct references, constructed
occurrences and portable references. Add regressions for nested/empty references,
repeated targets, scalar and invalid shell inputs, parent/provenance behavior,
scope rejection, cancellation, lowered budgets and worker/fallback transport.
Then finish the operation guide and compact template examples, audit the cell
demo layout, and proceed to the deferred Storybook stabilization.

The two temporary failing probes were removed after recording their exact
outcomes here. The existing constructor, expression and portable-value suites
remain the baseline: all 39 tests in `expression_values`, `portable_values`
and `retained_node_values` pass. They do not cover the reproduced reference
transport mismatch. That original note did not authorize runtime or demo changes; the user's
selection above supersedes the pause. Verification is tracked as
`CEMT-CONSTRUCTOR-REFERENCES` in [TODO](todo.md).

## Native pipeline lifetime and cached scope limits (R03/R08/R10)

The 2026-09-21 pipeline audit reproduced a limit bypass in constructed and
portable values: projecting a 40-level tree under a child with depth 16 failed
when the XPath cache was cold, but succeeded after a parent query warmed it.
The accepted environment-defined limit contract already requires child scopes
to respect lower ceilings, so this correction needs no new semantic decision.

The shared CEM-ML projection now checks current limits on cache reuse. When
limits shrink, it revalidates the immutable graph and checks expanded index
counts without rebuilding the index. CEM-QL supplies effective limits for both
constructed and portable inputs on every access. The existing memory admission,
query capability checks and terminal control errors remain in force.

Permanent native regressions cover cold/warm lowered depth, parent recovery,
expanded index limits, cancellation and failed-import permit cleanup. They also
cover two render stages and artifact round trips preserving aliases, source and
occurrence parents, rich attribute values and scalar constraints. XPath results
remain navigable and exportable after input handles are dropped; their memory
is released with the final live result. Verification is tracked as
`CEMT-PIPELINE-LIFETIME` in [TODO](todo.md).

Verification passed: 136 focused CEM-QL tests, 96 adapter tests, two shared
projection tests and workspace test compilation; WASM/component build,
typecheck, lint (two existing warnings), 408 runtime unit tests, both portable
value transport fixtures and all three cell-demo browser stories. Two stale
XPath reader assertions now expect retained subtrees from `{$node}`, preserving
their source-change and owner-lifetime checks. Render engine 1.5.4 invalidates
older previews. Pipeline coverage is complete; DX review and the cell demo's
compact-layout audit remain before the deferred Storybook stabilization.

No external-format handling or viewer template changes are involved. XML and
JSON continue to enter through the shared CEM-ML importer as retained CEM trees.
The maintained contract is in [Native CEMT values](cemt-native-values.md#xpath-over-constructed-values).

## Large integer query representation (R06/R08)

**Selected by the user on 2026-09-21: use decimal evaluation consistently.**
Direct typed values and portable values now share the integer-to-query-atom
mapping. Values within `i64` remain integer atoms; larger integers use decimal
atoms while retaining their integer datatype metadata and exact lexical value.
Receiver conversion and hook returns follow the same mapping. Artifact format,
numeric operator rules and decimal arithmetic limits are unchanged. The
maintained behavior is documented in [Native CEMT values](cemt-native-values.md).
Permanent regressions cover signed boundaries, query type checks and arithmetic,
receiver/hook conversion, metadata, template reload and existing failures.
The worker/file/fallback fixture now includes a larger integer and direct
evaluation checks. Verification is tracked as `CEMT-LARGE-INTEGER` in TODO.

Verification passed: 175 focused CEM-QL tests, 96 adapter tests and workspace
test compilation; WASM/component build, typecheck, lint (two existing warnings),
408 runtime unit tests, both worker/file/fallback fixtures and three cell-demo
browser stories. Render engine 1.5.3 invalidates older previews. The attribute-type
matrix is complete; native-value pipeline coverage and DX review remain in TODO.

Original review and reproduction, preserved below:

The 2026-09-21 attribute-type audit confirmed that scalar conversion, lexical
precision and declared datatype survive native and portable component handoff,
but query behavior for integers outside `i64` depends on transport. The
maintained contract requires typed native values across that boundary; it does
not specify how those integers participate in CEM-QL's numeric types.

The native producer is:

```cem
{child |
    {attribute @name=value @type=integer @value=922337203685477580812345}}
```

A receiver that queries the binding without declaring a conversion produces:

| Probe | Direct native handoff | After CEMV encode/decode |
| --- | --- | --- |
| Query atom | `String("922337203685477580812345")` | `Decimal("922337203685477580812345")` |
| `{p \| {$value * 0.0}}` | `<p></p>` with numeric-operand errors | `<p>0</p>` |
| `{p \| {$value + 1.0}}` | `<p></p>` with numeric-operand errors | `<p>922337203685477580812346</p>` |
| Native datatype / lexical value | `integer`, unchanged digits | `integer`, unchanged digits |

`render/attributes.rs::TypedValue::atom` exposes every wrapped typed value as a
string, including an integer too large for `AtomValue::Integer(i64)`.
`eval/portable.rs::GraphView::atom` instead uses a decimal atom for those
integers. A receiver declaring `@type=integer` converts the restored value back
through `TypedValue` and returns to string-like query behavior. These paths
must agree; choosing their shared numeric behavior affects type checks,
arithmetic and callers that currently see strings.

| Direction | Benefits | Costs |
| --- | --- | --- |
| **Recommended: consistently use the existing decimal evaluator for integers outside `i64`, retaining `integer` datatype metadata.** | Preserves already accepted values and the portable path's numeric behavior; requires a bounded adapter correction, without a new artifact format. | Query type checks see a decimal atom for larger integers. Existing matching-numeric-operand and decimal arithmetic limits still apply; this does not add arbitrary-precision arithmetic. |
| Reject integers outside `i64` at shared conversion/import. | Every accepted integer fits the current integer evaluator. | Rejects previously valid values and portable artifacts; narrows the shared schema contract. |
| Extend the shared CEM-QL integer representation and evaluator. | Larger integers can remain integers in query type checks and arithmetic. | Expands evaluator, operators, casts, budgets and public value APIs; needs a separately bounded design and precision policy. |

The user's selection supersedes the original stop-at-decisions pause.
The approved implementation plan was to share or align the typed-atomic adapters,
preserve datatype/provenance and existing operator rules, then add permanent
tests for direct handoff, receiver conversion, hook returns, positive/negative
boundary values and portable reload. Cover arithmetic and type checks, including
expected overflow and mixed-numeric errors, followed by worker/fallback checks.
Do not modify the data-viewer templates or external-format import boundary.

Evidence: a temporary `large_integer_query_behavior` test failed the direct vs
portable atom comparison and printed both arithmetic reproductions above.
It was removed after recording the results. The committed passing matrix covers
scalar lexical conversion and precision, numeric/string facets, temporal zones,
invalid contracts, mixed native segments and final text/HTML/XML projection.
The implementation is tracked as `CEMT-LARGE-INTEGER` in [TODO](todo.md).
Verification of the independent matrix passed: 8 shared-contract tests,
70 focused CEM-QL tests and workspace test compilation. No runtime or browser
assets changed in this increment.

## Expression hooks in attribute bodies (R05/R07/R12)

**Selected by the user on 2026-09-21: attribute insertion scope (option 1).**
The renderer now tracks the insertion destination independently of lexical
nesting and direct expression-hook returns. Attribute bodies, transparent
controls and named/matching/imported calls inherit the attribute destination
and its metadata. Constructed node content resets to the content destination;
nested attributes establish their own destination. Scope exit restores the
enclosing destination. Native atomics and node references survive module calls
without flattening. Final destination conversion and constraints still apply.
Native result attributes preserve their scalar construction and text adjacency;
explicit `result-sequence` continues to return native values directly.

The maintained contract and migration note are in
[Native CEMT values](cemt-native-values.md#expression-hooks-and-attributes).
The third `cell-overrides.html` lesson now demonstrates a conditional attribute
body with a count hook and a retained label subtree. Data-table, inspector and
tree viewer implementations remain unchanged. Verification is tracked under
`CEMT-HOOK-ATTRIBUTE-BODY` in [TODO](todo.md).

Verification passed: 242 focused CEM-QL tests (including nine new scope cases),
96 adapter tests and workspace test compilation; WASM/component build,
typecheck, lint (two existing warnings), 408 runtime unit tests, both native
worker/file/fallback fixtures, three cell-demo browser stories and all standalone
and source-loaded cell lessons. Render engine 1.5.2 invalidates older previews.
The broader attribute-type/pipeline matrix and DX review remain open in TODO.

Original review and reproduction, preserved below:

The hook audit on 2026-09-20 found that equivalent attribute expressions take
different hook paths depending on how their body is written. The accepted
contract describes expression destinations and native attribute values, but
does not settle whether an attribute body is an attribute insertion scope or
a block that returns native values directly. Changing this affects authored
templates, so implementation is paused under the user's stop-at-ambiguity rule.

Two temporary native Rust probes used these hooks:

```cem
{template @on=expression @into=attribute |
    {$"attribute:" + dom:text()}}
{template @on=expression @into=content |
    {$"content:" + dom:text()}}
{p |
    {attribute @name=via-value @value='{"one"}'}
    {attribute @name=via-body | {$"two"}}
    {attribute @name=conditional |
        {cem:if @test=true | {$"four"}}}
    {attribute @name=rich @content-type=text/html |
        {b | {$"three"}}}}
```

Observed final attribute values (markup shown before HTML escaping):

| Attribute | Value before implementation | Selected value |
| --- | --- | --- |
| `via-value` | `attribute:one` | `attribute:one` |
| `via-body` | `two` | `attribute:two` |
| `conditional` | `content:four` | `attribute:four` |
| `rich` | `<b>content:three</b>` | `<b>content:three</b>` |

`constructed_attribute_value` captures direct body expressions as native return
values, bypassing expression hooks. An `if` creates another render scope, so
its expression falls outside the capture depth and runs a content hook instead.
The actual destination remains the same attribute. Nested element content also
runs content hooks, which is appropriate for the `<b>` payload in this example.

| Direction | Benefits | Costs |
| --- | --- | --- |
| **Recommended: attribute bodies establish an attribute insertion scope.** Direct expressions and expressions reached through `if`, loops and named calls use attribute hooks. Constructed elements establish content scope for their own children and attribute scope for their own attributes. | Equivalent authoring forms behave consistently; control statements do not change the output destination; rich payloads retain their native structure. | Existing bodies that relied on bypassing attribute hooks change behavior. The renderer must track insertion context separately from direct hook-return capture. |
| Keep attribute bodies as native return blocks; only `@value` interpolation invokes attribute hooks. Preserve that direct-return behavior through control statements and named calls, while constructed elements establish their own content scope. | Retains an explicit authoring distinction between interpolated attributes and native body construction. | Authors must learn two attribute expression contracts. Wrapping a value in an attribute body bypasses scoped attribute hooks; the current conditional path still needs correction. |

For the recommended direction:

1. Carry the insertion target and destination attribute metadata through
   control statements and template calls. Keep lexical hook precedence,
   active-hook exclusion, whole-sequence focus and scope restoration unchanged.
2. Separate direct expression-hook returns from the attribute body's insertion
   context, so a hook does not redispatch its own returned sequence.
3. Preserve literal/native segments and apply shared destination conversion
   and validation after construction. Do not flatten nodes or parse string
   payloads to implement hooks. Imported data still enters only through CEM-ML.
4. Add native cases for both probes, loops/calls, nested element attributes,
   rich values, empty input, metadata restoration, recursion/cancellation and
   artifact reload; then verify the existing worker and browser consumers.
5. Update the maintained contract and the native-value lesson in
   `cell-overrides.html` where it clarifies the distinction. Keep data-table,
   inspector and tree viewer implementations unchanged.

The two probes originally failed against the recommended outputs above. The
user's selection supersedes the original decision pause. Their temporary test target was
removed after recording the results. Five independent, passing native cases in
`packages/cem_ql/tests/expression_hooks.rs` cover mixed attribute expression
sequences/literals, lexical loop captures, nested query focus, active-hook
fallback and restoration of attribute context and reserved bindings. The
implementation and permanent regression coverage are tracked as
`CEMT-HOOK-ATTRIBUTE-BODY` in TODO.

## Named attribute constraints (R06/R07)

**Selected by the user: compose constraints and version the artifact.** The
implementation adds flat `AttributeValueContract.restrictions`, writes CEMV
version 3, and reads versions 1 and 2 without accepting new restrictions under
old headers. Output, receiver and hook conversion share validation of every
constraint against the final normalized value. The strongest requested
whitespace normalization applies. Clones, XPath attribute selection and native
result construction retain the complete contract. The maintained behavior is
documented in [Native CEMT values](cemt-native-values.md).

Original review and reproduction, preserved below:

The follow-up audit reproduced an existing output-attribute gap with two native
Rust probes. A host-defined `positive-count` type has `value_type=integer` and
`minInclusive=3`; a host-defined `uppercase-code` type has `value_type=string`
and `pattern=[A-Z]+`. Before the fix, these declarations published invalid values:

```cem
{output |
    {attribute @name=count @type=positive-count @minInclusive=1 @value=2}}
{output |
    {attribute @name=code @type=uppercase-code @pattern="[A-Za-z]+" @value=abc}}
```

The probes expected no output, but observed `<output count="2"></output>` and
`<output code="abc"></output>`. `constructed_attribute_value` cloned the named
contract and then replaced individual facets with local ones. Receiver
declarations already checked the named type independently; constructed output
did not. This behavior predated the explicit-dispatch follow-up.

Checking both contracts during rendering alone would leave the transported
contract incomplete: `AttributeValueContract` stored only one model,
and the portable graph validator rechecked that model. A pair of regex constraints
cannot generally be reduced to the single supported regex field. The decision
therefore covers retained contracts as well as initial validation.

| Direction | Benefit | Cost |
| --- | --- | --- |
| **Recommended: compose inherited and local constraints.** Preserve the resolved base contract and additional restrictions in the shared native value contract; require the value to satisfy all of them. | Named types remain authoritative while templates can add restrictions; workers and saved pipelines retain the same validation rules. | Extend the public contract shape and version the portable artifact so older readers cannot silently ignore restrictions. |
| Reject local facet overrides on named types. | Keeps the existing portable contract shape and makes unsupported overrides explicit. | Template authors must define another named type to change an inherited facet; it removes useful local narrowing. |

Implementation proposed during review, now approved:

1. Add a flat collection of additional constraint models to the shared contract.
   Keep type conversion/normalization distinct from validation: the final typed
   value must satisfy the base and every local restriction. Do not silently relax
   base normalization or replace a base regex. Use the same path for output,
   receiver and hook/destination validation.
2. Preserve the complete contract in native attributes, clones, XPath adapters
   and component handoff. Write a new CEMV version and retain version-1/version-2
   reads. Reject an unsupported newer version rather than dropping constraints.
3. Promote the two failing probes to regression fixtures, add accepted local
   narrowing and conflicting-pattern cases, then verify tampered portable values
   fail the inherited constraints after encode/decode. Cover worker, saved-file
   and fallback handoff, plus scope/memory accounting of the added metadata.
4. Update the maintained contract and acceptance criteria after the decision.
   CEMT syntax and the data-table/XML viewers need no changes.

Verification evidence: a temporary `named_constraints_review` Rust integration
target ran two tests and both failed on the invalid output above. The temporary
target was removed after reproducing the issue; it is not part of the passing
regression suite. Implementation and permanent fixtures are tracked by
`CEMT-NAMED-CONSTRAINTS` in [TODO](todo.md). The user's selection supersedes the
original decision pause. Permanent native tests now cover both reproductions,
accepted narrowing, conflicting/invalid patterns, normalized final values,
host/receiver/hooks, clone/XPath preservation, version compatibility, tampered
artifacts and resource controls. The separate WASM fixture uses binary artifacts
produced by these Rust tests. Verification passed: 5 shared-contract tests,
162 focused CEM-QL tests, 96 adapter tests, workspace test compilation, both
WASM native-value fixtures, 408 runtime unit tests and 3 cell-demo browser stories.
Build, typecheck and lint pass, with the two existing lint warnings. The viewer
templates remain unchanged; deferred gallery/Storybook stabilization stays in TODO.

## Implementation updates

Implementation update: explicit `cemt:apply_templates(values, mode)` now shares
native template dispatch and survives artifact reload. Controlled text/markup and
artifact boundaries enforce scope, cancellation and memory limits; XPath indexes
retain bounded scope caches and owner memory permits. The maintained contract is
[Native CEMT values](cemt-native-values.md). Earlier review notes below remain as
discussion history; the remaining matrix and DX work are tracked in TODO.

Earlier implementation update: module hook defaults/caller inheritance, receiver input
contracts and native XPath projection are implemented. The maintained
[contract](cemt-native-values.md) describes their behavior. Portable artifacts
wrote version 2 to distinguish root output occurrences from source-target
references, while retaining version-1 reads. The follow-up above completes
explicit dispatch and its resource controls; broader DX review continues in TODO.


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

## 8. Navigation and value verification follow-up (2026-09-20)

The native cell fixtures now exercise the agreed inline local-name match,
sibling `id`, immutable label references and final attribute text projection.
They cover repeated/missing fields, nameless siblings, namespace-independent
ID lookup, nested tables, source repair and selection through sorting. The
shared `retained_node_values.rs` fixture exercises navigation, source metadata,
text, references, clones and empty element construction after XML, JSON, YAML
and CSV import, including XPath views and restricted hosts.

This audit reproduced an R10 enforcement gap: a child execution scope with a
queue cap of four allowed twenty navigation results because the evaluator
only combined execution memory/stack caps with the query context. The correction
also combines the existing queue-, CPU- and I/O-derived ceilings. Standalone
query entrypoints retain their context-configured budgets; supplied operation
scopes cannot be relaxed by a query context. Native cases verify item limits,
active call depth, cumulative call counts and parent/sibling isolation. This
enforces the accepted scope rule without adding a new capability or changing
the viewer templates. Current verification evidence is tracked in `docs/todo.md`.

`dom:text` consumes import-decoded values; XML line-ending normalization belongs
to CEM-ML import. Source lexical fields/ranges remain available separately, and
selected source text/CDATA nodes remain distinct from XPath's coalesced view.
No external-format branches were added to CEM-QL or rendering.
