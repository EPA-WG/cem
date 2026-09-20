# Standalone XML viewer migration review

Reviewed 2026-09-13 against `~/aWork/custom-element/demo/`. The subsequent
user-requested **multi-format table view** implements part of the table lessons;
the lossless XML tree/branch-selection migration remains open. `tree.xml` is
still unported, while `table.xml` and `table.xsl` are partially migrated in
[`legacy-demo-cases.json`](legacy-demo-cases.json). Their four source files,
including `tree.xsl`, still match the audit's SHA-256 fingerprints.

Execution follows the XML-VIEW checkitems in
[`docs/todo.md`](../../../docs/todo.md). The legacy files are functional
references, not authority for syntax, architecture or accessibility.

## Implemented multi-format table lane

DATA-TABLE-1 adds [`data-table.html`](../demo/data-table.html), linked from the
demo index. Existing native XML/CSV/YAML/JSON parsers feed the generic
`cem-data` reader / `data:read` function and typed CEM AST query views.
Native code does not discover tables or manufacture row/cell metadata.

The shared [CEMT viewer](../demo/data-table-view.cemt) detects repeated XML
siblings and arrays, unions columns, derives cells and sort keys, and renders
nested tables and tree disclosures. Only reusable grouping and stable sorting
belong to CEM-QL. Source IDs survive sorting; source edits change the version.

The fifth example imports the unchanged viewer and adds
[match-based aspects](../demo/data-table-aspects.cemt): notes become a tree,
an IP-filter record becomes an editable form, and visits remain a table.
Turning aspects off restores the defaults. Form edits affect a local preview,
not the source document or a firewall. No demo-local JavaScript behavior is
used.

Ordinary `cem-element` declarations preflight the static module graph using
their declaration URL and scoped resolver. The worker and its main-thread
fallback receive content-hashed source modules, include them in cache identity,
and return imported static styles for declaration-scoped installation. A
source-only binary artifact cannot stand in for that dependency closure.

These explicit **lossy UI views** do not replace the typed CEM-tree writer or
claim complete XML-VIEW-1/3 parity. XML source owners retain ordered events,
comments and PIs; tree display keeps inert values but is not a lexical export.
The original checkitems remain open for broader lossless inspection and
branch-selection coverage. Generic import, collection and dispatch contracts
are documented in the [CEM-QL README](../../cem_ql/README.md#native-data-import-and-presentation-dispatch).

## What the local sources actually demonstrate

Both XML entry points contain the same XHTML-namespaced Pokémon data; only
their `xml-stylesheet` processing instruction differs. They are standalone
documents, not additional `html-demo-element` cards.

| Lesson | Local evidence | Finding |
| --- | --- | --- |
| Recursive tree disclosure | `tree.xml:2`; `tree.xsl:14–31` | Initially open native `details` show element names, attributes and text; summaries toggle individual branches. The `./text()` value-of emits only the first selected text node before child elements, so this is not lossless mixed-content inspection. |
| Tree/table grouping | `table.xsl:120–171`, `174–245` | Repeated sibling names become tables; unique names stay in a tree. Nested repeated child groups become nested tables. Grouping uses lexical `name()`, not expanded namespace identity. |
| Collapse and selection | `table.xsl:54–68`, `140–152` | Two independent checkboxes per non-leaf branch drive collapse and a red selection border. Selection is branch-local and non-exclusive, not row selection, recursive selection, or an exported data slice. Hidden inputs and label-only controls provide no usable keyboard focus path. |
| Table columns and values | `table.xsl:176–199`, `226–245` | Headers come from the first row's attributes and child elements. The source explicitly notes the missing union of columns across rows. Direct text has no column: the example `li` rows can show `pokemon-id` but omit `ivysaur`/`venusaur`. |
| Sort controls | `table.xsl:24–34`, `102–107`, `198–223` | **Scaffold only.** The key is `@stub-will-be-replaced`; headings are `href="#"` links carrying `xv:sortpath`. Direction/priority labels inspect source `xsl:sort` nodes. No local handler replaces the key or reruns the transform. Working interactive sorting is not evidenced by this checkout. |
| Compatibility machinery | `table.xsl:6–12`, `27–39`, `78–79` | The stylesheet uses EXSLT functions/node-set and an MSXML JScript block containing `eval`. These are not part of the bounded native compatibility path and must not be enabled for the migration. |

The sorting finding comes from reading the complete stylesheet and searching
the local checkout for `sortpath`, `StartSort`, `stub-will-be-replaced`, and
`XmlViewRendered`. Only the stylesheet contains those hooks. This is a source
review; it does not claim that the legacy table stylesheet executes unchanged
in a current browser. No legacy scripts were executed.

`priority="100"` filtering and generated XPath-like IDs are old viewer
implementation details. Do not silently discard ordinary user data with that
attribute, or use a generated display path as mutable selection identity.

## Current capabilities and gaps

- [`tree.cemt`](../demo/tree.cemt) recursively inspects the current instance
  island, including attributes, namespaces and comments. It is not an
  arbitrary standalone XML-document loader. The bounded
  [`data-island-tree.xsl`](../demo/data-island-tree.xsl) is a loader compatibility
  lesson, not evidence of general XML viewer parity.
- [`for-each.html`](../demo/for-each.html) covers authored rows and a checkbox
  that inserts/removes the entire table. It does not discover heterogeneous
  columns, sort records, or maintain selected source rows across sorting.
- [`cem_ml::import`](../../cem_ml/src/import.rs) maps the native XML parser
  owner into a retained common CEM tree. Its source-oriented arena keeps mixed
  content, CDATA, whitespace, comments, PI bodies and source ranges; the
  separate semantic view supplies XPath text coalescing and decoded values.
  CEM-QL and XPath consume this same retained tree. The original XML events
  remain provenance owned by import, never a downstream evaluator interface.
- The HTTP and lifecycle migrations retired `xmlElementToRecord` and the
  document-record binding. The browser transports bytes and request metadata
  to CEM-ML import; HTTP consumers retain native CEM nodes. The normative
  [import boundary](../../../docs/cem-data-import-principle.md) also applies to
  inspection: do not revive parser-AST traversal or source decoding in a
  viewer, query or formatter.
- [`sequence.rs`](../../cem_ql/src/stdlib/sequence.rs) now registers bounded
  generic Tier B `group_by` and `sorted` primitives. Other operators in
  [AC-QO-6](../../../docs/cem-ql-ac.md) remain separate planned work.

## Migration boundary

Use explicit source loading into the existing native XML owner. Treat
`xml-stylesheet` as inspectable, inert source data, never as permission to fetch
or execute a stylesheet. Keep source identity and provenance attached through
projection, grouping, sorting and rendering.

Default structural presentation/export is tabular CEM-ML from the typed tree
writer. Tree and table presentations are explicit views of the same source
owner, not replacement XML/JSON document models. JSON is allowed only at a
named external export boundary; no `serde_json::Value`, DOM-to-record conversion,
serializer/re-parser, or inferred DTO may sit between native stages.

Author viewer UI declaratively in CEM-ML with static scoped styles and public
hooks. Use existing native disclosure and visible labeled controls. If source
loading, identity-preserving projection or state binding needs a new reusable
capability, implement it in the shared native/runtime layer first. Do not add
page-, component-, or Storybook-local UI behavior.

Do not copy `table.xsl` into the compatibility demo, enable browser XSLT,
MSXML/EXSLT scripts, or add a browser `eval`/sorting handler. The sorting lesson
will be a new working native implementation of the prototype's intention,
not a claim that its placeholder controls already worked.

## Executable follow-up

### XML-VIEW-1 — typed inspection boundary

Start with package-owned small fixtures and native tests over the existing
XML AST and its typed projection. Cover same local names in different
namespaces, two prefixes for one namespace, same-named attributes/elements,
empty versus missing attributes, Unicode, comments, processing instructions,
CDATA and mixed text before/between/after children. Preserve event order,
source ranges and original identity; do not normalize source whitespace away.
Malformed XML must produce useful source diagnostics. External entity
resolution stays forbidden; depth/size/expansion limits remain bounded.

Prove the typed writer output before adding a standalone UI. Add any missing
shared browser/WASM loading boundary only after its native contract is green.
Keep inspected markup, scripts and stylesheet instructions inert.

#### XML-VIEW-1 presentation decision — pending

The 2026-09-19 native boundary audit is recorded in
[`xml_inspection_boundary.rs`](../../cem_ml/tests/xml_inspection_boundary.rs).
It verifies expanded names (including aliases for one namespace), separate
attribute/element identities, empty versus missing values, Unicode, ordered
mixed content, unnormalized source whitespace, comments, CDATA and original
byte ranges. The retained parser owner preserves the complete ordered lexical
input. Malformed XML has line/column diagnostics; DTDs and unresolved entities
are rejected. The current reader's 32 KiB, 64-level and 4096-event boundaries
are tested at and immediately above their limits. This audit does not
introduce new resource-limit policy.

The final typed presentation remains incomplete. This input:

```xml
<r empty=''>before<![CDATA[<raw>🍒]]><!--note--><?keep inert?>after</r>
```

reaches `cem_tree_nodes` with every payload, but its current tabular writer
output is:

```cem
{r @empty="" |
    before
    {node}
    {comment @value=note}
    {xml}
    after
}
```

CDATA content and PI target/data disappear. Formatting also inserts whitespace,
and an ordinary source element named `comment` is not distinguished from the
comment presentation by a dedicated inspection vocabulary. The source-oriented
CEM PI currently has `target="xml"`, `data="keep inert"`; the shared semantic
node correctly has target `keep`, value `inert`. The existing
[`xslt_xml_model_boundary.rs`](../../cem_ql/tests/xslt_xml_model_boundary.rs)
records that split. A viewer must not split the lexical PI body itself.

**Recommendation:** add a shared, format-independent typed CEM inspection
projection over `RetainedCemTree`, using explicit node-kind, source-node ID,
expanded-name and payload fields in the existing AST presentation vocabulary.
Keep the source arena order and source-map stacks, including separate CDATA,
empty CDATA and whitespace nodes that XPath coalesces or omits. Carry source
identity/ranges through the projection and writer output spans, retaining the
original owner. Source-node IDs address the source arena; do not reuse XPath's
coalesced text identity for separate CDATA or whitespace nodes. Escape source
values as data so inspected elements or PIs cannot become execution directives.
Keep namespace declarations as retained provenance; exclude them from viewer
data attributes as already decided. This is a presentation of the original
owner, never a new query document or an XML/JSON handoff. Use the shared tabular
writer and the terminal colorizer where applicable.

In the same change, correct XML PI target/data fields **at import**, retain the
original lexical body/range in the source owner, and migrate the source-view
consumers/tests. Preserve XPath's existing decoded PI behavior. This is an
explicit source-view contract migration; format-specific decoding remains
confined to import. Cover the shared inspection projection with XML, JSON,
YAML and CSV imports so it cannot become an XML-only evaluator.

The alternative is to change the ordinary CEM writer's canonical encoding of
CDATA/PI/source nodes, affecting all serialization callers. The recommended
dedicated inspection projection makes the node-kind and exact-value contract
explicit and follows the existing typed AST presentation approach. It requires
a decision because the current inspection API uses ordinary tree formatting,
and the PI source fields are an existing public CEM-QL contract. Execution is
paused here under the user's stop-at-decisions instruction. XML-VIEW-1 stays
open; no standalone tree UI is claimed complete by these boundary probes.

### XML-VIEW-2 — grouping and stable sorting

Specify the minimal shared helper contract against the existing Tier B AC
before implementing it. Test grouping by expanded element name within one
parent, first-seen group/column order, distinct attribute-versus-element keys,
and a union of columns from **all** rows. Missing cells are not empty strings.
Retain an explicit text-content column and inspectable mixed/nested content.

Sorting tests must cover ascending/descending text and numeric keys, duplicate
keys, missing values, malformed numeric values and resource budgets. Define
one deterministic missing-value policy; keep equal-key rows in source order
in both directions. Reversing the complete ascending result is not a stable
descending implementation. Preserve source IDs and leave the source AST
unchanged. Do not imply locale-aware or multi-column sorting without a
separately tested contract.

### XML-VIEW-3 — standalone tree teaching points

After XML-VIEW-1, add an explicit-source viewer with local editable/reloadable
examples and separate success/failure cases. Show CEM-ML structural output,
keyboard disclosure, and visible independently labeled branch selection.
Use small fruit/Unicode values where they clarify the example. Selection and
collapse must remain independent: collapsing a branch does not clear its
selection. Reset selection when replacing the source document unless a later
identity-reconciliation contract explicitly says otherwise.

Do not call branch selection row selection. Test the ordinary Space/Enter
control behavior, source-relative loading, namespace labels, empty values,
and recovery from malformed input. Neither a copied legacy XML URL nor a
processing instruction is the new viewer's execution entry point.

### XML-VIEW-4 — table inspector teaching points

After XML-VIEW-1 and XML-VIEW-2, add focused cases for heterogeneous repeated
rows, text-only rows, nested groups, and live sorting with persistent row
selection. Preserve `ivysaur`/`venusaur`-style text that the legacy table omitted.
Use semantic tables, captions and `th[scope]`; sort buttons expose `aria-sort`
on the active heading and readable names for ↑/↓ actions. A selected row is
keyed to source identity, never its current display index. Check multiple
selected rows, equal-key sorts and sibling/instance isolation.

Prove every authored case in native tests, source contracts, source-loaded
Storybook and standalone fixture verification. Check desktop two-card layout
and mobile containment. Update the inventories and Nx cache inputs as actual
fixtures arrive; only then replace the `unported-xml-viewer` classifications.

## Review verification

The case-map unit guard keeps this review marked `table-view-implemented-tree-open`,
records sorting as `scaffold-only`, and resolves each pending XML-VIEW item to
this document and the open TODO checklist. The review document is a unit-test
cache input. CI uses the checked-in audit, not a developer's legacy checkout.
The XML-VIEW-1 boundary probe is linked in the native evidence inventory. It
characterizes the current writer gap; it does not make that lossy output the
future acceptance contract. This checkpoint adds no runtime capability or
browser viewer and needs no new browser-layout assertion of its own.

Verification 2026-09-13: all seven case-map tests and the full 341-test unit
suite pass through Nx, as does lint. The four local source fingerprints,
review links and resolved Nx cache input were also checked.

Verification 2026-09-19: 20 native tests pass across `xml_inspection_boundary`,
`cem_import_boundary`, `import_strings` and `xpath_xml_view`; all eight case-map
checks pass through `cem-elements:test:unit`. The probe is included in that
target's Nx cache inputs. XML-VIEW-1 remains open at the presentation decision.
