# Native DOM helper review

Status: implemented and verified, 2026-09-21. The user
approved unqualified strings plus namespace descriptors, including scoped native
attribute access. The active work item is in [TODO](todo.md); existing node fields
and base viewer templates are unchanged.

## Reproduced gap before implementation

The registry declared `dom:descendants(node)` and `dom:attribute(node, QName)`.
Both default evaluator branches returned an empty sequence without validating
their arguments. The design table named `QName` without defining how an author
supplies one to this helper. CEM-QL's ordinary atomic values do not include a
QName constructor.

A Rust-native probe using `compile` and `evaluate`, with no browser or host
override, produced these results for `<r id='a'><child>text</child></r>`:

| Query after shared CEM-ML import | Items |
| --- | --- |
| `data:read(source, "xml").root.descendants` | 3: root element, child element, text |
| `dom:descendants(data:read(source, "xml").root)` | 0 |
| `data:read(source, "xml").root.children.attributes` | 1: the `id` attribute |
| `dom:attribute(data:read(source, "xml").root.children, "id")` | 0 |

Every query compiled and completed without errors. The zero counts demonstrated
missing implementation; they were not accepted parity expectations. The earlier
opaque `cemml:parse` fixtures cannot establish native behavior.

## Accepted contract

`dom:descendants(node)` accepts zero or one native node, excludes that node,
and traverses its children in depth-first source order. Empty input stays empty.
It retains identity, provenance, ownership and access restrictions; it never
clones, serializes or reparses the input. Attributes and reference targets are
not extra traversal axes: use their explicit native accessors when needed.
The pipeline form applies this operation to each input in sequence order,
without implicit sorting or deduplication.

For `dom:attribute`, use an explicit name selector:

```cem-ql
dom:attribute(node, "id")
dom:attribute(node, {namespace: "urn:catalog", name: "id"})
```

The accepted meaning of a bare string is an exact **unqualified** name:
`"id"` matches local name `id` with an empty namespace. The descriptor matches
both namespace and local name exactly. This descriptor is query control
metadata, not a JavaScript representation of an imported document.

For an imported `<row xmlns:p="urn:catalog" id="local" p:id="qualified"/>`,
the first call selects the attribute whose text is `local`; the descriptor
selects the one whose text is `qualified`. Under the local-name alternative,
the first call would select both attributes.

Do not infer namespace bindings from XML source prefixes, query-library aliases
or host DOM. Prefix strings, wildcard syntax and malformed descriptors should
fail explicitly. A missing attribute returns empty; a match returns the native
attribute node, including any typed native value, rather than its text.

Both helpers should follow the existing children/parent cardinality and error
rules: strings, records, opaque node identifiers and multiple direct input nodes
are type errors. A node view that cannot expose the requested axis fails closed;
it must not be treated as having no nodes.

## Attribute-name alternatives

| Choice | Advantage | Cost |
| --- | --- | --- |
| **Exact unqualified strings plus explicit namespace descriptors (recommended)** | Predictable namespace identity; prefix-independent; matches the design table's expanded-name intent. | Authors must name the namespace when selecting a qualified attribute. |
| Local-name strings across all namespaces, plus exact descriptors | Short calls mirror `attribute.name == "id"`. | A call can return several attributes with different namespaces; adding a namespace may silently broaden matches. |
| Prefixed strings resolved against the input node | Familiar to XML authors. | Requires another namespace-resolution contract and has no equivalent prefix environment for every native input or constructed/portable value. |

The user's earlier Pokémon sibling simplification remains valid: that sample
explicitly filters `.name` and needs only a local-name comparison. It does not
settle a general helper's namespace contract.

## Shared access and limits

Implement descendants using the existing scoped `QueryItemView::children`
operation. Add a scoped attribute iterator alongside it for attribute access;
its default implementation returns `Unsupported`. Implement that iterator for
imported CEM, XPath, constructed and portable nodes and preserve restrictions
through wrapper views. Do not fall back to unscoped `field("attributes")`.

Each adapter keeps its own established node view. CEM source attributes remain
source attributes; the XPath view retains its existing namespace-node exclusion.
Do not add XML-specific filtering or external-format parsing in the evaluator.
XML, JSON, YAML and CSV still enter only through CEM-ML import.

Poll cancellation during traversal, charge work and emitted items against the
environment and lowered CEM scope limits, and discard partial output on failure.
Preserve the current restriction and unsupported-view diagnostics.

## Implementation and verification

1. Add native tests for order, cardinality, empty/missing values, namespaces,
   imported/constructed/portable values, references, identity and provenance.
2. Cover restricted hosts, lowered limits and mid-traversal cancellation before
   implementing the scoped attribute axis and both helpers/pipeline forms.
3. Replace the old opaque/empty-output browser rows in
   `cem-ql-rust-first-parity.stories.ts` with real native examples;
   update the function reference and its XPath/CEM-QL companion use cases.
4. Verify native tests first, then WASM and browser integration. Leave the base
   data-table and XML viewer implementations untouched.

The selector and shared access extension are approved together. Stop only for
new decisions or ambiguity outside this contract.

## Implemented result

The evaluator now uses scoped native child and attribute iterators. Descendant
traversal is iterative, charges each queued node before retaining it, and stays
bounded even for a cyclic host view. Attribute scans charge work even when no
name matches; both helpers discard partial output on cancellation, denial or
budget failure. Native attributes preserve identity, provenance, contracts and
typed contents through lookup. Reference targets remain explicit; references
have no structural children or attributes.

The 14 retained-node tests cover the four imports, original and XPath views,
constructed/portable values, namespace selection, invalid selectors, repeated
pipeline inputs, source maps, references, restricted hosts, lowered environment
and child-scope limits, cyclic hosts and cancellation. Constructor/portable
regressions and the authored XPath/CEM-QL pairs pass. The broader native run has
615 passing tests and the same three pre-existing `xslt_data_view` failures:
XSLT's selection cell is `td`, while CEMT's is `th`. Those failures remain in the
separate viewer-scope TODO; neither viewer was changed or its parity weakened.

The XPath functions page now uses `dom:attribute(node, "qty")` and
`dom:text(item)` in its CEM-QL pair, and explains qualified selectors and the
`descendant::node()` pair. Browser parity rows use native imported nodes and
explicitly reject opaque nodes and malformed selectors. XML, JSON, YAML and CSV
still resolve exclusively through shared CEM-ML import.

Both WASM builds and the runtime build pass, along with 413 runtime unit tests,
191 browser stories, typecheck and lint (two existing warnings). The complete
gallery passes for 28 standalone pages and 34 source-loaded documents. The
authored XPath functions page fits two cards per row at 1440px and has no page
overflow at 390px or 320px. The remaining decision is the separately tracked
XSLT/CEMT selection-cell parity scope, not a blocker for these native helpers.
