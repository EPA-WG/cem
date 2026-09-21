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
| `dom:clone(values)` | Create independent native node storage and identities, retaining origin metadata. |
| `dom:element(elements)` | Construct empty element shells with the selected names and namespaces. |
| `dom:children(node)` / `dom:parent(node)` | Navigate retained nodes without copying or importing them again. |

Named calls inherit focus. A parameter called `node` does not itself rebind
focus. Explicit selection and variable bindings retain native values.
Records and arrays are not inferred to be document nodes.

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

## Portable artifacts: accepted R08 transport

CEM-ML owns the `CEMV` version-1 binary value artifact. It contains a flat native
value graph with ordered roots, children, attributes, reference targets and
native attribute segments. Scalars keep their datatype; attributes keep their
shared schema contract and representation. Records retain source frames and
available origin URI, source-selection key and line metadata.

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

The [implementation checklist](todo.md) tracks remaining coverage and DX work.
The [temporary review](cemt-expression-review.tmp.md) retains the discussion;
R08's portable artifact choice is settled.
