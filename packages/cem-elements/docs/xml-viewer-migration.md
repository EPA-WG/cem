# Standalone XML viewer migration review

Reviewed 2026-09-13 against `~/aWork/custom-element/demo/`, updated 2026-09-20.
The **multi-format table view**, XML-VIEW-3 retained-document tree and
XML-VIEW-4 table inspector now cover the reviewed teaching points. `tree.xml`,
`table.xml` and `table.xsl` are mapped to their native replacements in
[`legacy-demo-cases.json`](legacy-demo-cases.json). The four legacy source
fingerprints, including `tree.xsl`, remain unchanged. This mapping describes
working replacement lessons, not execution of legacy scripts or sort scaffolds.

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

These explicit **lossy UI views** do not replace the typed CEM-tree writer.
XML-VIEW-3 supplies branch selection in its separate tree lesson. XML source
owners retain ordered events, comments and PIs; tree display keeps inert values
but is not a lexical export.
XML-VIEW-1 now supplies the separate typed inspection boundary described below.
Generic import, collection and dispatch contracts
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

#### XML-VIEW-1 typed inspection — implemented

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

The user approved a shared typed inspection projection. It is implemented by
[`cem_tree_inspection`](../../cem_ml/src/projection/inspection.rs), taking an
`Arc<RetainedCemTree>` and producing `CemTreeAstStream` directly. The stream
retains the original owner through raw, formatted and colored artifacts.
No external syntax, parser-AST traversal, JSON bridge or re-import belongs in
this projection. XML, JSON, YAML and CSV use the same code and test matrix.

The AST presentation vocabulary represents each source node as an inert
`node` row: `id`, `kind`, `parent-id`, `order`, separate `name`/`namespace`,
optional `target`/`value`, source URI and original coordinates. Attributes and
children preserve their source order. IDs address the source arena within the
retained owner; separate CDATA, empty CDATA and whitespace nodes do not inherit
XPath's coalesced identity. Exact payload strings are attribute values, so
formatter indentation cannot change inspected whitespace and source names or
values cannot become executable directives. Source-map stacks reach output
spans, including zero-length attribute values. Namespace declarations remain
source provenance; the table and tree's data-attribute details continue to
exclude them.

For example, `<![CDATA[<raw>🍒]]>` produces a node row with `kind=cdata` and
`value="<raw>🍒"`; `<?keep inert?>` produces `kind=processing-instruction`,
`target=keep`, `value=inert`. Both survive the tabular writer and terminal
colorizer. The ordinary content writer retains its existing encoding; use the
inspection projection for structural display. Inspection preserves the source
CEM node model, not byte-for-byte XML formatting: original lexical spelling,
prefixes and event boundaries remain in the import-owned source provenance.

XML import now supplies the actual PI target and data separately to CEM-QL and
inspection. The source data retains literal line endings; XPath uses its existing
normalized semantic value. XML declarations retain target `xml` and stay omitted
from XPath. Original PI lexemes and ranges remain retained. The CEMT table
view explicitly combines target and data, preserving its visible parity with
the XPath viewer, including empty PI data. Consumers must not split PI text.

Public `inspect --show ast/tree` and textual `parse --format ast` use this
projection for imported data. Import selects supported lifecycle owners and
rejects invalid input without falling back to a partial tree or plain source
text. Diagnostics include source identity and available coordinates. Existing
CEM parser documents use the same typed vocabulary. Explicit JSON debug exports
and ordinary content serialization remain separate named boundaries.

The import-owned 64-level / 4096-event XML check is shared by string, byte and
retained-lifecycle entrypoints. The standalone browser tree/branch-selection
lesson is still XML-VIEW-3; this native completion does not mark that UI done.

### XML-VIEW-2 — grouping and stable sorting

Completed 2026-09-20. The audit found the required native helpers already
implemented by DATA-QUERY-1: `seq:group_by`, `seq:sorted`, and the first-seen
`seq:union` used for columns. They satisfy the required subset of
[AC-QO-6](../../../docs/cem-ql-ac.md#5-stream--set-operations).
The [collection contract](../../cem_ql/README.md#collections-and-presentation-dispatch)
defines the existing behavior. No additional API or runtime change is needed.

| Requirement | Shared behavior and native evidence |
| --- | --- |
| Parent-local, namespace-aware groups | The caller supplies one parent's element children and an expanded-name key. Prefix aliases group together; different namespaces and different parents stay separate. First-seen groups retain source member order. `sequence_collections.rs` verifies native identities and source maps. |
| Group-key identity | Empty keys, empty strings and different atomic types stay distinct under CEM identity. Keys must be empty or one atom. This does not adopt XPath's promotion or multiple-key grouping rules. |
| First-seen columns from all rows | CEMT builds cells and unions their keys. The authored-view regression checks exact heading order, separate `@same`/`same` columns, later-only fields and namespaced child columns. Namespace declarations stay excluded. |
| Missing and mixed content | Missing cells remain `∅`; present empty attributes remain `""`. The authored-view regression keeps direct text plus CDATA in `#text`, nested tables and text-only `ivysaur`/`venusaur` rows. This presentation remains deliberately separate from lossless source inspection. |
| Stable text and numeric sorting | The key is evaluated once per item. Text uses string comparison without locale collation; number mode accepts finite numbers. Both directions preserve equal-key order. Missing keys stay last; empty, malformed and nonfinite numeric keys join that last group in original order. |
| Source retention | Sorting returns the original nodes, with unchanged source IDs, lines and maps. Tests compare the typed source tree before/after and prove the owner lives until the returned nodes are dropped. |
| Bounded evaluation | Host scope policy supplies collection and call limits. The scope tree permits lower child limits and rejects relaxation. Tests cover collection-budget diagnostics with source maps, fatal budget errors through `try`, late key failures without partial output, cancellation and sequential key calls on a one-worker host. Existing `call_budgets.rs` verifies cumulative function-call limits. |

The strengthened tests are
[`sequence_collections.rs`](../../cem_ql/tests/sequence_collections.rs) and
[`data_view_templates.rs`](../../cem_ql/tests/data_view_templates.rs).
Existing [`data_import.rs`](../../cem_ql/tests/data_import.rs) and
[`xslt_data_view.rs`](../../cem_ql/tests/xslt_data_view.rs) cover the four-format
import boundary and CEMT/XPath viewer parity. Downstream helpers still receive
retained common CEM nodes; no external-format decoding is introduced.

This closes the native helper prerequisite. The audit itself added no demo
cards or browser behavior. The standalone tree implementation follows below;
the focused table teaching points are implemented under XML-VIEW-4 below.

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

#### XML-VIEW-3 inspection query — implemented

The 2026-09-20 boundary probe in
[`tree_inspection_boundary.rs`](../../cem_ql/tests/tree_inspection_boundary.rs)
found a missing CEMT query surface. An imported native document is available to
the template (`document.kind` is `document`), but `cemml:format(document)` emits
an empty string. Its current implementation takes the first string-like value
and passes it through; it does not call the typed writer. It also leaves an
input CEM-ML string unchanged. These are characterization assertions, not a
promise to preserve the placeholder as the final formatting contract.

The same retained owner already works with `cem_tree_inspection` and the shared
tabular writer. The probe covers XML, JSON, YAML and CSV, verifies retained
ownership through formatted output, and keeps source spans, CDATA and PI data.
The implemented query below exposes that native capability to authored CEMT.
Re-reading the source with another browser API or spelling CEM-ML by concatenating strings
inside the viewer would bypass this boundary.

**Accepted public contract: `cemml:inspect(document)`.** This is a Tier B
structural presentation operation in `cem:stdlib/cemml`, available through
ordinary CEM-QL and CEMT calls:

- Accept zero or one retained native CEM document. A retained XPath document
  backed by the same CEM owner is also accepted; use the source arena for
  inspection, without XPath coalescing. Empty input returns empty sequence.
- Return one display string produced by `cem_tree_inspection` and the shared
  tabular CEM-ML writer, with no terminal color escapes. This is final text
  presentation, not an AST handoff or text to execute as a template.
- Reject strings, ordinary records, multiple items and non-document nodes with
  a typed query diagnostic. The viewer needs whole-document inspection; a new
  subtree projection contract is outside this proposal.
- Preserve the original owner and source maps through native writer stages.
  Do not reparse source, serialize a runtime AST, traverse external parser ASTs
  or add format-specific query branches. Import remains solely in CEM-ML.
- Use the enclosing operation control and environment-defined scope budgets;
  child scopes may lower limits. Cancellation, limit exhaustion or writer
  failure must not return partial text. Add native acceptance tests for these
  cases before WASM/browser wiring.

The alternative is extending `cemml:format` to accept retained documents, but
that combines formatting CEM source/content with the distinct inert AST
inspection vocabulary. The named `inspect` operation makes the requested output
explicit. Neither option requires a viewer-specific native function.

Implemented 2026-09-20 under the user's standing instruction to proceed with
recommended options. The native acceptance fixture now calls `cemml:inspect`
through both retained document representations for all four import formats,
compares exact shared-writer output, and verifies escaped CEMT text, invalid
input, lowered budgets, cancellation and discarded writer failures. The WASM
adapter fixture is
[`tree-inspection.spec.ts`](../src/lib/tree-inspection.spec.ts).

Source nodes charge the query item budget; source payload and completed display
bytes are bounded by the effective `memory_bytes` policy. Enclosing
operation-control permits also honor ancestor memory limits and existing
charges. These payload limits do not account for every temporary formatter
allocation. Cooperative checks run during arena preflight and before/after
the existing synchronous projection/writer. No completed text is accepted
after cancellation, limit exhaustion or writer failure. The
[query reference](../../cem_ql/README.md#retained-document-inspection) describes
these controls and diagnostics.

#### XML-VIEW-3 standalone viewer — implemented

[`data-tree.html`](../demo/data-tree.html) has four separate lessons: editable
XML, JSON through the same retained tree, malformed XML and repair, and local
HTTP loading/release. The reusable [`data-tree-view.cemt`](../demo/data-tree-view.cemt)
uses `cemml:inspect(document)` for escaped structural output. Its branch view
traverses only common CEM node children and attributes. Namespace labels, empty
values, Unicode and mixed content remain visible; namespace declarations stay
in structural provenance and do not appear as data attributes. Script elements,
CDATA and processing instructions remain inert inspection text.

Visible labeled checkboxes allow independent branch selection; native
`details`/`summary` controls provide keyboard disclosure. Collapsing a branch
keeps its selection. The editable viewer uses the public source-event revision
to invalidate selections after any edit or reload, including identical source
text. Within one source revision, fixed child paths identify branches; they are
not identities for reconciliation across documents. Slice keys stay bounded by
the visited tree shape. Selection adds a visible word as well as color.

The request lesson imports the same inspection template and passes
`datadom.slices.resource.data` directly from the HTTP loader. CEM-ML imports
response bytes once; the viewer never decodes an external format or rebuilds a
JavaScript document. `resourceRevision` invalidates previous selections when
switching or reloading files. Choosing No source removes the request directive
and releases the retained document. The helper resolves repository XML/JSON
files relative to its declaration and offers the resolved source as a download.
No stylesheet is fetched or executed from an inspected `xml-stylesheet` PI.

The viewer authors `checked` as a conditional boolean attribute. The shared
browser patcher now synchronizes the live input property when this attribute
is added or removed, including dirty inputs. Unchanged attributes preserve
user edits, focus and unrelated controls. This is a shared HTML patch correction;
there is no demo-local JavaScript behavior.

Native [`data_tree_view.rs`](../../cem_ql/tests/data_tree_view.rs) covers all four
import formats, namespaces, inert mixed content, source-generation selection,
repair and retained HTTP documents. Browser stories exercise editing, selection,
disclosure, release/reload and the checkbox patch. The gallery inventory covers
all four lessons as standalone pages and source-loaded documents, including
Space/Enter keyboard controls. The source contract and migration inventory track
the new files and their Nx cache inputs. Existing host and scope import/query
limits still apply; this viewer adds no format-specific limit policy.

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
fixtures arrive; only then close the remaining partial table classifications.

#### XML-VIEW-4 focused inspector — implemented

[`table-inspector.html`](../demo/table-inspector.html) separates four lessons:
first-seen columns from heterogeneous rows, text-only rows, independent nested
tables, and multiple selection through stable sorting. The existing
[`data-table-view.cemt`](../demo/data-table-view.cemt) enables this presentation
with `inspector="true"`. Row discovery and cell projection remain shared; the
original CEMT and XSLT format-comparison examples retain their default controls
and pass the same native semantic-DOM parity tests.

Each inspector table has a caption, column-scoped headings, Text/Number
comparison, named ascending/descending buttons and an active heading's
`aria-sort`. Source order removes that active sort. Equal keys remain stable;
missing or invalid numeric keys stay last in either direction. These are
existing `seq:sorted` semantics, with no browser-local sorting or new native
viewer functions. Namespace declarations remain excluded from data columns.

Row checkbox slices use the shared opaque `data:node_key` provenance key, never
a display index. Each table's sort/comparison slices use its first original
member's key, before sorting; siblings and nested collections therefore keep
independent controls even when their display labels match. Instances own their
slice state, including two instances with identical source bytes. Source event
revisions invalidate selections on edits and resets, including identical-source
reloads. Captions count only their own rows; selected text and checked controls
make the state visible without color. Collapsing keeps selection intact.

The browser probe also exposed a shared patch bug: a dirty checkbox can be
reused for a different row before its previous selection render commits, while
both plans omit `checked`. Merely patching changed attribute presence leaves
the previous live value on the new row. Patch application and full render-plan
reconciliation now apply the authored checked state when a checkbox/radio's
slice binding changes. The comparison uses retained authored attributes even
after the adapter consumes directive attributes. Unchanged bindings keep user
edits, focus and unrelated controls. No new binding API or demo-local behavior
is introduced.

[`table_inspector.rs`](../../cem_ql/tests/table_inspector.rs) proves the authored
mode over all four imports, union columns, mixed and text-only content, source
selection, stable ties, independent nested controls, source resets and parse
recovery. External decoding stays exclusively in CEM-ML import. Browser stories
add dirty-input reconciliation, actual editing, captions, active sort headings,
and sibling/identical-source instance isolation. The standalone/source-loaded
inventory exercises Space on checkboxes and Enter on a sort button.

## Review verification

The case-map unit guard keeps this review marked `tree-and-table-lessons-implemented`,
records legacy sorting as `scaffold-only`, and resolves completed and pending
XML-VIEW items to this document and the TODO checklist. The review document is a unit-test
cache input. CI uses the checked-in audit, not a developer's legacy checkout.
The XML-VIEW-1 native contract tests are linked in the evidence inventory. They
verify typed projection, writer output, retained ownership, import rejection
and public inspection. XML-VIEW-3 adds the authored tree and request evidence.

Verification 2026-09-13: all seven case-map tests and the full 341-test unit
suite pass through Nx, as does lint. The four local source fingerprints,
review links and resolved Nx cache input were also checked.

Verification 2026-09-19: 20 native tests pass across `xml_inspection_boundary`,
`cem_import_boundary`, `import_strings` and `xpath_xml_view`; all eight case-map
checks pass through `cem-elements:test:unit`. The probe is included in that
target's Nx cache inputs. This was the decision checkpoint before implementation.

Implementation verification 2026-09-19: all 24 native import/inspection tests
and 22 CEM-QL/XPath import/viewer tests pass, including PI target/data consumers
and XML declarations. The focused CLI inspection and native presentation tests,
AST schema-package verification, WASM builds, lint and browser typecheck pass.
All 390 browser unit checks and five data-table Storybook interactions pass.
The fixture verifier passes all 25 standalone pages and 31 source-loaded
documents. Desktop layout fits two cards per row; all seven cards stay
contained at 1440px, 390px and 320px widths.

XML-VIEW-2 verification 2026-09-20: all 31 focused native tests pass across
`sequence_collections`, `data_view_templates`, `data_import`, `call_budgets`
and `xslt_data_view`. The 390-test browser unit suite, lint and typecheck pass
through Nx. This audit changes tests, contract documentation and completion
inventory only; no browser templates, runtime code or demo layout changed.

XML-VIEW-3 boundary verification 2026-09-20: both native boundary tests pass,
including all four import formats. The 390-test unit suite and lint pass
through Nx; the resolved unit target includes the new native evidence file.
That probe preceded the subsequently implemented inspection query and viewer.

XML-VIEW-3 viewer verification 2026-09-20: all five native viewer cases pass,
along with 398 unit checks, four tree-viewer Storybook cases and five existing
table-viewer cases. Build, lint and typecheck pass through Nx. The final gallery
verifier passes 26 standalone pages and 32 source-loaded documents, including
Space/Enter disclosure, independent selection/deselection, same-source reload,
parse recovery, correct initial source choice and preserved CEM-ML indentation.
The page fits two cards at 1440px and remains contained at 390px and 320px.
The browser network audit confirms that the inspected stylesheet PI is not
requested. The XML-VIEW-4 implementation follows that checkpoint.

XML-VIEW-4 verification 2026-09-20: all 18 native cases pass across
`table_inspector`, `data_view_templates` and `xslt_data_view`, preserving the
default CEMT/XSLT semantic-DOM comparison. All 401 unit checks and 13 focused
Storybook cases pass, including four inspector cases and the existing table
and tree viewers. The inspector stories also pass after adding identical-source
instance isolation. Build, lint and typecheck pass through Nx. The full gallery
verifier passes 27 standalone pages and 33 source-loaded documents. The page
fits two cards at 1440px and stays contained at 390px and 320px. All reviewed
XML-VIEW teaching-point fixtures are now complete; the separate browser startup
wait and repeated-mount stylesheet lifecycle work remains in `docs/todo.md`.
