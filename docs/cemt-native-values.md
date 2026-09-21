# Native CEMT values and component transport

The accepted expression contract reuses immutable CEM nodes. Transformations
leave their source trees unchanged. `{$values}` inserts atomic text and retained
node occurrences into body content; it does not deep-clone a source subtree.
Each occurrence has an output location while its targets retain source identity
and source-parent navigation. `{node}` still constructs a literal element.

| CEM-QL operation | Meaning |
| --- | --- |
| `dom:text(values)` | Concatenate node string values and atomic lexical values, without separators. |
| `dom:text()` | Read the nearest established focus. A matching template establishes its processed node; an expression hook establishes the whole expression sequence, including an empty sequence. Missing focus is an error. |
| `dom:reference(values)` | Explicit reference to an ordered native sequence, retaining repeated targets. |
| `dom:clone(values)` | Clone each selected node and its reachable graph, preserving node kinds and internal target sharing with fresh identities. Atomic values stay values. |
| `dom:element(elements)` | Construct empty element shells with the selected names and namespaces; ordinary attributes and children are not inherited. |
| `dom:children(node)` / `dom:parent(node)` | Navigate retained nodes without copying or importing them again. |

Named calls inherit focus. A parameter called `node` does not itself rebind
focus. Explicit selection and variable bindings retain native values.
Records and arrays are not inferred to be document nodes.

### Choosing a native-value operation

Use `{$node}` to insert a retained subtree, `{$dom:text(node)}` for its text,
and `{$dom:children(node)}` to insert its children without the parent element.
`dom:reference(values)` makes the retained reference explicit in a query;
`.targets` selects its ordered values. `dom:clone` is for independent node
storage and identities. `cemt:apply_templates(values, mode)` instead invokes
the active renderer's match rules; pass `""` for the default mode.
Construction and insertion do not implicitly dispatch templates.

A reference is a node in its own right. Given `r = dom:reference((name, name))`:

| Expression | Result |
| --- | --- |
| `dom:clone(r)` | One new reference whose two links share one cloned target. |
| `dom:clone(r.targets)` | Two independently cloned elements. |
| `dom:element(r.targets)` | Two new empty elements with the targets' names and namespaces. |
| `dom:element(r)` | Type error: a reference is not an element. |
| `dom:text(r)` | The target text twice, in order. |

These rules hold for direct references, output occurrences and portable values.
Cloning preserves nested and empty reference nodes. Selected clone roots are
detached, internal child parents are rebuilt, and source provenance survives.
Cloning checks host access restrictions even though source ancestors are omitted.
The source tree remains unchanged. Migration: direct references previously
unwrapped in both constructors; explicitly select `.targets` for that behavior.

Only `dom:text()` offers an implicit-focus overload. The other operations require
the arguments shown above. `dom:reference` is the supported full spelling;
`dom:ref`, `dom:copy` and `dom:copy_of` are not aliases. `dom:element` accepts
existing element nodes, not a name string or a name/content overload. Use
ordinary CEMT construction, such as `{name | ...}`, to author a named element.

### Compact template bodies

Matching templates, named templates and expression hooks accept direct content
without a `{body | ...}` wrapper, including inside imported CEMT modules.
Matching supplies `node`; hooks supply `value` and `context`. Declare parameters when a named call needs additional
inputs, not merely to repeat those implicit bindings:

```cem
{template @name=label | {strong | {$dom:text()}}}
{template @mode=cell @match='node.name == "name"' |
    {td | {call @template=label}}}
{template @on=expression @into=attribute | {$dom:text()}}
```

The named call inherits its caller's focus. Wrapping the same content in `body`
does not change that focus or the insertion rules. Use direct content or one
explicit `body`; combining them, or declaring multiple bodies, is a compile
error. Parameter declarations and formatting whitespace may accompany either
form. Output comments count as content and belong inside the chosen body.
Named and matching declarations inside a module still require `@name`;
expression hooks are exempt. Visibility and function-body syntax are unchanged. The module's own default entrypoint still uses `body`.

The inline [Pokémon cell example](../packages/cem-elements/demo/cell-overrides.html)
uses the compact matching form and its implicit `node` binding. Module import
preflight and direct compilation enforce the same body rule. Previously,
preflight rejected compact named bodies and direct compilation silently chose
the first explicit body when declarations were mixed. Move all output into one
body, or remove the wrapper and use direct content.

`dom:text` reads values decoded by CEM-ML import, including XML entity and
line-ending normalization. It extracts each selected CEM text/CDATA node
independently; XPath can expose those same neighbours as one coalesced text node.
Original lexical fields and source ranges remain available for source inspection.
Selected attributes, comments and processing instructions yield their own
text; element/document text excludes descendant attributes, comments and
processing instructions. Empty nodes and sequences yield an empty string.
XML, JSON, YAML and CSV use the same downstream operations; formats are
interpreted only by the CEM-ML importer.

## Expression hooks and attributes

A scoped expression template receives the complete expression result as `value`
and context metadata as `context`. Its direct expressions return native values.
The compact text conversion is:

```cem
{template @on=expression @into=attribute | {$dom:text()}}
```

Use `@into=content` to change body insertions in the following siblings and their
descendants. Scope exit restores the previous behavior. `@match` can inspect
`context.attribute.name`, and `@returns` requests shared conversion and validation.
A receiver's declared contract cannot be relaxed by an expression hook.

Within a scope, higher priority wins, then the later declaration. A nearer
lexical scope wins over every outer priority. Plain top-level hooks activate
when encountered and capture the bindings available there. Module-level hooks
are defaults for that module's entrypoints; caller scopes override those
defaults across named calls, imported calls and the native transformation
adapter. A declaration inside the called body establishes a nearer scope.
Hook predicates and bodies fail the render on evaluation errors. Direct return
expressions bypass redispatch, and active declarations cannot invoke themselves
implicitly. Stable declaration identities survive compiled artifact reloads.

Attribute interpolation applies the hook to each expression's complete sequence,
preserving literal segments between expressions. An attribute body establishes
the same attribute insertion destination as `@value`. Direct expressions and
expressions reached through conditionals, loops, named/matching templates and
imported calls use attribute hooks with that destination's metadata. Atomic
types and native node identities survive these calls until the destination
applies conversion and validation.

Constructed nodes establish their own content destination: expressions inside
an element payload use content hooks, and its attributes use their own attribute
hooks. This also applies to native result constructors and bodies of comments,
CDATA and processing instructions. Returning to the enclosing attribute restores
its destination. Direct expressions in an expression hook still return native
values without redispatch; lexical scopes and active-hook exclusion are unchanged.
Explicit `result-sequence` retains its native return semantics.

For example, both attributes below become `attribute:ivy`; the rich payload
becomes `<b>content:ivy</b>` before final attribute escaping:

```cem
{template @on=expression @into=attribute | {$"attribute:" + dom:text()}}
{template @on=expression @into=content | {$"content:" + dom:text()}}
{p |
    {attribute @name=direct @value='{"ivy"}'}
    {attribute @name=conditional | {cem:if @test=true | {$"ivy"}}}
    {attribute @name=rich @content-type=text/html | {b | {$"ivy"}}}}
```

Migration: attribute-body expressions previously bypassed hooks when direct,
or invoked content hooks when wrapped in a control statement. They now
consistently use attribute hooks. The accepted decision and original evidence
are retained in the [proposal review](cemt-expression-review.tmp.md#expression-hooks-in-attribute-bodies-r05r07r12).

An attribute retains an ordered native value sequence independently of its
browser string projection. `type`, constraints and representation are separate:

```cem
{child-card |
    {attribute @name=count @type=integer @minInclusive=1 @value=002}
    {attribute @name=day @type=date @value=2024-02-29}
    {attribute @name=code @type=string @pattern="[A-Z]{2}-[0-9]+" @value=AB-12}
    {attribute @name=label @type=node @content-type=text/html @value="{$name}"}}
```

Built-in scalar conversion covers strings, integers, finite decimal numbers,
booleans, dates, times and date-times. Temporal text uses an explicit ISO-style
lexical profile: four-digit nonzero years, valid calendar dates, ordinary
24-hour times and optional `Z` or numeric zones. No locale or timezone is inferred.
Regex constraints match the full string. Invalid conversion discards the result.
HTML, XML and plain text are final representation choices; strings containing
markup never become nodes by implicit parsing. Native child components receive
the original sequence rather than parsing the projected DOM attribute.

Integer values within `i64` use the integer evaluator. Larger positive or
negative integers use the existing decimal evaluator, consistently in direct
native values, hook returns, receiver conversion and portable values. Their
lexical digits and `integer` datatype metadata remain intact: for a larger
value, `value is decimal` is true while the native value and artifact retain
the `integer` datatype metadata.
Portable artifacts keep the integer datatype; no artifact version change is
needed.

The existing numeric rules still apply. For a larger integer, `value + 1.0`
uses exact decimal arithmetic, while `value + 1` requires an explicit numeric
conversion because the operand types differ. Decimal evaluation remains bounded
by its checked `i128` coefficient and scale operations; preserving a larger
lexical integer does not guarantee every arithmetic operation can represent it.
Overflow, unrepresentable decimal operands and division by zero remain errors.
No implicit conversion to floating point occurs. The
[accepted large-integer decision](cemt-expression-review.tmp.md#large-integer-query-representation-r06r08)
retains the original transport inconsistency and alternatives.

Named types supplied by `TemplateData.value_types` retain their base model.
Local facets add independent restrictions; every restriction must hold. For
example, a named integer minimum of `3` plus a local minimum of `1` still rejects
`2`, while a local minimum of `5` narrows the accepted values further. Regex
patterns intersect by separate validation, without rewriting either regex.

The shared `AttributeValueContract` stores `model`, `restrictions` and
`content_type`. Conversion and whitespace normalization precede validation of
the final value against every model. Whitespace uses the strongest requested
normalization (`preserve`, then `replace`, then `collapse`); a local declaration
cannot weaken an inherited setting. Malformed bounds, patterns and other facet
definitions are errors. Native `node`/`any` content does not silently accept scalar
facets. A host destination owns final scalar conversion, and both its constraints
and the template's constraints validate that result. Receiver and hook conversion
use the same shared implementation. In an expression hook,
`context.attribute.pattern` is the ordered sequence of applicable regex patterns.

The receiving component declares its input contract in its template prelude:

```cem
{attribute @name=count @type=integer @minInclusive=1 @required=true}
{attribute @name=day @type=date @required=true}
{attribute @name=label @type=node @required=true}
{article | {$label}{p | Next count: {$count + 1}}}
```

The native renderer validates inputs before evaluating the body. Receiver
constraints apply independently of the sender's contract, including after
worker transport or saved-artifact loading. Scalar conversion updates both the
named binding and `datadom.attributes`; node inputs retain their identities.
Missing optional inputs remain absent. `@required=true` rejects missing inputs.
Native hosts can additionally supply `TemplateData.input_attribute_contracts`;
these constraints also apply and are separate from output attribute contracts.
An input error publishes no partial result.

## XPath over constructed values

XPath functions accept constructed CEM output and portable CEM values through
a shared CEM-ML semantic projection. It builds a cached native index on first
use, without exporting XML, JSON or reparsing text. The original values remain
immutable and retain their owners; subsequent queries reuse the index.

XPath sees inserted references as their content at the output location. Repeated
insertions have distinct occurrence identities and output parents. Detached
roots have no fabricated document parent. Explicit reference targets keep
source parents and identity; CEM-QL accesses them through `.targets`. Adjacent
text is coalesced according to XPath semantics. Attribute string values derive
from native segments, while CEM-QL retains those segments and their contracts.
Namespaces, source-selection keys, origin URIs and line numbers survive the
projection. The native projection checks graph, expansion, depth and byte limits
and polls execution control while constructing its index.
Selected native XPath attributes retain their full value contract as metadata;
their XPath string-value semantics stay unchanged. Cloning, explicit native result
construction and portable export preserve these constraints.

The output owner retains at most its most recently used query scope's index.
Changing scope rebuilds the index under that scope's access checks. Cache eviction
never invalidates returned XPath nodes: their tree owner retains its memory permit
until the last reference is released. Cached access is checked against the current
execution scope's memory and depth allowances, including when another operation
built it. Lower limits revalidate the native graph and the expanded index's value
and byte counts; cache reuse never grants the builder's larger limits. A rejected
or cancelled child stage returns no partial result, releases its temporary memory
permits and leaves its parent scope usable.
Portable graphs retain one scope-independent index after validated import, with
the same execution-limit checks on each use.

Native intermediate output can feed another render directly or through a CEMV
artifact. Repeated artifact round trips preserve shared reference targets,
source ancestry and typed attribute contracts, while inserted occurrences retain
their output parents. Earlier input handles may be released once the next stage
owns its values. A live XPath selection also retains everything it needs for
navigation and later export. Once the input/cache owners are gone, dropping the
last result releases the index permit.

## Explicit template dispatch from CEM-QL

`cemt:apply_templates(values, mode)` invokes the active renderer's match rules and
returns native result content. Both arguments are required; `""` selects the default
mode. It shares mode selection, priority, declaration order, imported rules, caller
hooks and focus restoration with `{apply-templates}`. Unmatched values produce an
empty sequence. It does not inherit query-local variable names as template parameters;
template lexical bindings remain available and the selected item becomes `node`.

```cem
{template @mode=label @match='node.name == "name"' | {$node}}
{p | {$cemt:apply_templates(label, "label")}}
```

The result can feed `dom:text`, a native attribute or another query before insertion.
The containing expression applies its normal hook to the returned whole sequence;
active hook declarations still cannot invoke themselves implicitly. This distinction
allows a caller's text hook to flatten the complete result of a matching template.
Query recovery can catch data errors raised by a called template. Recursion,
resource failures and cancellation remain terminal. An ordinary standalone query
without a CEMT host reports `cem.ql.template_context_missing`.

The host is borrowed only during evaluation. Compiled artifacts store the call,
without serializing renderer state or acquiring a browser dependency. Native module
entrypoints retain their match declarations when their body is selected.

## Portable artifacts: accepted R08 transport

Browser slice events use the same native graph transport as component attributes.
A constructed native `slice-value` attribute supplies its authoritative value
sequence to the named slice. `DataIslandSnapshot.nativeSlices` and the explicit
`cem-native-slices-v1` saved envelope preserve that sequence without projecting
it to DOM text or JavaScript document records. Import/export helpers are public
alongside the native attribute helpers. See the
[lifecycle contract](cem-element-lifecycle-principle.md#native-slice-values) and
[JSON storage demo](../packages/cem-elements/demo/local-storage.html) for native
edits and CEM-ML JSON export.

CEM-ML owns the `CEMV` version-3 binary value artifact. It contains a flat native
value graph with ordered roots, children, attributes, reference targets and
native attribute segments. Scalars keep their datatype; attributes keep their
shared schema contract and representation. Records retain source frames and
available origin URI, source-selection key and line metadata.
Version 2 additionally distinguishes output occurrences from standalone target
references, including root insertions, and preserves an explicitly empty native
attribute sequence separately from an imported attribute's lexical value.
Version 3 adds composed attribute restrictions. The importer revalidates the base
model and every restriction, including when no named-type registry exists in the
receiving worker. Version-1 and version-2 artifacts remain readable with their
original semantics; they cannot declare version-3 restrictions. Unknown newer
versions are rejected. The named browser transport envelope is unchanged; the
binary artifact carries its own version.

Repeated references within one artifact address the same target record. A
receiving worker creates a new local owner and identities, preserving those
alias relationships. It cannot preserve a memory address from another worker.
Artifacts also retain the source ancestry needed for parent and sibling queries.
A restricted host view must deny access outside its grant; transport does not
turn a restricted node into an unrestricted source owner.

The envelope uses a version marker, a BLAKE3 integrity digest and MessagePack
encoding. CEM-ML validates graph ownership, backlinks, IDs, types, cycles,
reference expansion, depth and size before publishing decoded values. Native
CEM-QL adapters consume that graph directly. Neither JavaScript nor the query
engine reconstructs it from XML or JSON document objects.

The browser render boundary exports one artifact for a render's native
attributes and attaches root indexes to its final DOM patch metadata. Workers
and fallback carry `ArrayBuffer` bytes. Imported handles are local and are
released after rendering; returned nodes retain their decoded owner. Unclaimed
output artifacts do not accumulate. Source and packaged runtimes share the
versioned attribute store on their page.

DOM hydration and explicitly requested JSON state exports use the named
`cem-native-attributes-v1` envelope with deduplicated base64 artifact bytes.
This is binary transport encoding, not a JSON AST. Saved render plans retain
binary content, and their cache identity includes its bytes. Metadata changes
must update a component even when its displayed attribute string stays equal.

`CemElementRuntimeOptions.nativeValueLimits` defines environment ceilings for
`maxBytes`, `maxValues` and `maxDepth`. Defaults are 16 MiB, 100,000 expanded values
and depth 128. `CemDeclarationScopeOptions.nativeValueLimits` follows ordinary
CEM ancestry: descendants inherit limits and may only lower them. Native Rust
callers supply `CemValueArtifactLimits` at import/export boundaries.

Controlled Rust boundaries are `encode_values_with_control`,
`decode_values_with_control`, `project_attribute_value_with_control` and
`project_render_plan_with_control`. Projection/export receive a query capability
scope separately from the execution scope. Each uses the supplied native value
limits, capped by the execution scope's memory and logical-depth policy. Child
scopes can only lower these ceilings. HTML/XML serializers and the WASM DOM patch
boundary use this path; failed projection publishes no prefix or artifact handle.

CEM-QL item and call budgets use the tighter of the evaluation context and the
effective execution-scope policy. A context cannot raise a supplied host or child
scope's ceilings. Standalone evaluation keeps its context-configured item and
call limits. Exhausting a child query's budget does not cancel its parent or
sibling queries; a failed query returns no partial node sequence.

`dom:text` charges both visited work and lexical bytes against the query policy's
memory-derived ceilings, and reserves extracted bytes against operation memory.
Final text extraction accepts borrowed source fragments and owned atomic lexical
segments through `QueryItemView::text_segments`; the existing borrowed-only
`text_fragments` API remains available. Neither API permits fallback around denied
node access. Artifact graph validation and cached XPath reference expansion poll
execution control. Binary codecs run between checked boundaries on size-bounded
input. Memory permits account retained graph records, edges, lexical payloads, source
frames, provenance, schema metadata and conservative semantic-index storage; this is explicit resource accounting, not a
measurement of every allocator overhead byte.
Restrictions contribute to retained metadata bytes and artifact work limits.
`convert_attribute_value_with_check` polls between models and separates invalid
data from interrupted execution. Cancellation and lowered child-scope budgets
discard the result.

XML, JSON, YAML and CSV continue to enter through the shared CEM-ML import
layer. There are no format-specific evaluator or renderer branches. See the
[import principle](cem-data-import-principle.md).

## Verification and remaining review

The [cell override demo](../packages/cem-elements/demo/cell-overrides.html)
includes node reuse, explicit text conversion, a scoped content hook and a
parent/child handoff carrying an integer, date and name subtree.

Native fixtures cover expression focus, reference identity, construction,
contracts and artifact round trips. `packages/cem_ql/tests/native-values-wasm.mjs`
checks separate WASM workers, saved artifact bytes, fallback, lowered limits and
handle disposal. Browser stories check the declarative demo.
`packages/cem_ql/tests/native-constraints-wasm.mjs` generates its binary fixtures
through the Rust `named_attribute_constraints` test, then verifies saved-file
reads, separate WASM workers and fallback. Correctly hashed invalid artifacts
test inherited and local constraints independently; JavaScript never reconstructs
the document or native value graph.

The [implementation checklist](todo.md) tracks remaining coverage and DX work.
The `native_value_contract`, `attribute_value_matrix` and `portable_values`
fixtures cover scalar lexical profiles, precision, temporal zone validation,
numeric/string facets, invalid contracts and mixed native segments through
direct/portable handoff and text/HTML/XML projection. The large-integer cases
also cover signed `i64` boundaries, query type checks, exact decimal arithmetic,
receiver/hook conversion, retained metadata, template reload and unchanged
arithmetic errors. The native-value WASM fixture checks the same larger integer
before transport and after saved-file, worker and fallback handoff.
The [temporary review](cemt-expression-review.tmp.md) retains the discussion;
R08's portable artifact choice is settled.

The [constraint review](cemt-expression-review.tmp.md#named-attribute-constraints-r06r07)
records the selected composition direction and its original failing probes.
