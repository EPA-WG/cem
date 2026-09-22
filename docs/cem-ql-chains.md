# Native CEM-QL chains

`dom::chain(values)` creates an immutable `Chain<T>` over native CEM values.
It accepts an empty sequence, one value, or a sequence; wrapping an existing
chain is idempotent. Nodes retain their source owner, identity and provenance.
There is no source-tree mutation or implicit subtree clone.

```rust
dom::chain(node)
    .parent()
    .children()
    .find(|sibling| sibling.name() == "id")
    .text()
```

The [live DOM examples](../packages/cem-elements/demo/functions/dom.html) cover
the methods below. The [cell override](../packages/cem-elements/demo/cell-overrides.html)
uses this expression to find a Pokémon image identifier.

## Syntax and values

Methods use `receiver.method(arguments)` and callbacks use `|item| expression`
or `|item| { expression }`. `::` resolves a module alias just like the existing
single-colon CEM-QL name form. Existing `fn(item) => expression`, namespace
functions and field access remain available. This is a Rust-shaped CEM library
API, not a Rust compiler or an implementation of `Iterator`.

In particular, `Chain::find` returns another chain, unlike Rust's
`Iterator::find`, which returns an `Option`. Method calls are distinguished from
field access: unknown methods produce diagnostics, never empty field reads.

Callbacks receive one individual value. On native nodes, `name()` returns a
local-name string when a name exists; `text()` returns the shared CEM node
string value. An element supplied by `children()` has a name, so
`|element| element.name() == "id"` requires no optional-value wrapper.

An empty chain remains empty through navigation, selection, extraction and
mapping. Its callbacks are not invoked. Invalid arguments and methods are
still rejected on empty input. Callback exceptions, denied access, cancellation
and exhausted budgets remain errors and never produce partial output.

## Navigation and extraction

| Method | Result |
| --- | --- |
| `parent()` | Each member's existing parent; absent parents contribute nothing |
| `children()` | Element children, flattened in input order and child order |
| `child_nodes()` | All structural child nodes, including text and comments |
| `ancestors()` | Ancestor elements, excluding self, nearest first for each input |
| `closest(predicate)` | First matching element on the self-to-root walk, at most one per input |
| `name()` | Local-name strings; nameless nodes contribute nothing |
| `text()` | One string value per node, including an empty string for an empty element |
| `attribute(selector)` | Matching native attribute nodes, retaining their typed values |

`parent().closest(predicate)` searches strictly above the starting node.
Results retain duplicates: two children can select the same parent twice.
Reference targets are not structural children and remain accessible explicitly
through `.targets`.

`attribute("id")` selects an unqualified attribute. For a qualified attribute,
use `attribute({namespace: "urn:catalog", name: "id"})`. Prefix strings and
wildcards are not attribute selectors. Call `.text()` to extract attribute text.

The existing `.children` field and `dom:children(node)` still include all child
nodes. Only the new chain method `children()` has the element-only contract.
All external formats resolve at the [CEM-ML import boundary](cem-data-import-principle.md).
There are no XML-, JSON-, YAML- or CSV-specific evaluation branches.

## Selection, mapping and questions

| Method | Result |
| --- | --- |
| `filter(predicate)` | Every accepted member |
| `find(predicate)` | First accepted member across the collection, or empty |
| `find_last(predicate)` | Last accepted member across the collection, or empty |
| `first()`, `last()` | First or last member, or empty |
| `take(n)`, `skip(n)` | Keep or omit a prefix; `n` is a nonnegative integer |
| `map(callback)` | One result per member; a returned sequence/chain becomes a nested collection value |
| `flat_map(callback)` | Concatenate callback results, flattening one collection level |
| `any(predicate)` | Boolean; false for empty input |
| `all(predicate)` | Boolean; true for empty input |
| `is_empty()` | Boolean; true for empty input |
| `count()` | Integer, including duplicates; zero for empty input |

Predicates must return exactly one boolean. The explicit scalar terminals end
the chain; a boolean containing `false` is never represented as a truthy wrapper.

Adjacent streaming methods pass items incrementally. `find`, `first`, `take`,
`any`, `all` and `is_empty` stop upstream enumeration when their answer is known.
`find_last` evaluates the predicate in forward order and retains the last match.
Assigning a chain to a binding materializes that expression's results, so a
later `find` cannot undo earlier traversal work.

## Immutable ordering

| Method | Result |
| --- | --- |
| `sorted(direction?, mode?)` | Sort atomic members by their own values |
| `sorted_by_key(callback, direction?, mode?)` | Sort original members by a computed atomic key |
| `reversed()` | Reverse the collection's current order |

Sorting uses shared `seq:sorted` behavior: default `"ascending"` and `"text"`,
with explicit `"descending"` and finite `"number"` comparison available. It
evaluates each key once. Empty keys sort last; number mode also puts malformed
and nonfinite keys last. Equal keys keep their original order in either
direction. Text comparison has no locale collation. Node sorting requires an
explicit key rather than implicit string conversion.

Ordering buffers values/handles and keys under the host's resource limits.
Child scopes can lower those limits. Reordering never changes node parents or
the order of children in the imported tree. Query work, calls, nesting and
retained output use the shared evaluator budgets and operation control.

## CEMT and portable values

Rendering a chain inserts its members through the existing CEMT native-value
rules. Node-valued body expressions reuse nodes; attribute values retain their
native content until the final string/DOM boundary. `.text()` requests explicit
text extraction. Use `dom:clone` only when independent node identity is wanted.

Portable transport materializes node and atomic chain members into the existing
native CEM value artifact. Nested CEM-QL arrays remain query values: use
`flat_map` to expose their members before native export. Unsupported value kinds
are rejected at that boundary, never silently flattened or converted. It transports values and references, not executable callbacks
or a suspended iterator. Use `dom::chain(received_values)` to continue querying
after transport. No JavaScript document objects or alternate format parsers
are involved.

## Verification

Rust coverage exercises parser/type behavior, each method, CEM-ML imports and
portable nodes, identity/provenance, CEMT body/attribute rendering, short-circuit
access, failures, cancellation and resource limits. The dedicated
`CEM Elements/CEM-QL DOM Functions` Storybook group loads the authored samples
and checks navigation, selection, extraction, scalar questions and ordering.
The gallery verifier checks the page standalone and source-loaded.
