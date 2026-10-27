# HTML and CEM card

Open **Examples → Content types and schemas → Documents and UI → HTML and CEM
card**. Choose **Clone selected** for an editable saved project. The same project
also appears under **Queries and transformations**. Copies own their resources,
configurations, revisions and drafts independently of the read-only example.

This project demonstrates a small semantic HTML/CEM-ML comparison, data-bound
native HTML generation and reuse of a production declarative component. For a
larger comparison with forms, labels and required controls, use the existing
[Login project](../login/README.md).

## Resources and configurations

| Configuration | Saved input | Result |
| --- | --- | --- |
| `inspect-html` — Inspect HTML card | `data/card.html` | Retained HTML structure and original source positions |
| `inspect-cem` — Inspect CEM-ML card | `data/card.cem` | Native CEM structure and original source positions |
| `semantics-html` — Card semantics from HTML | `data/card.html` and `queries/card.xpath` | Nine strings matching `expected/semantics.json` |
| `semantics-cem` — Card semantics from CEM-ML | `data/card.cem` and the same query | The same nine-string contract |
| `build-html` — Build the card as HTML | `graphs/card.cem`, the HTML data and native template | `generated/card.html`, matching `expected/card.html` |
| `live-view` — Shared card live view | `views/card.cem` and the same HTML data | Two shared `cem-card` instances and a local action |
| `inspect-component` — Inspect component declaration | `components/cem-card.xhtml` | Native CEM-ML source-AST presentation retaining the complete mixed-quote template |
| `repair-html-policy` — Repair inline HTML policy | `failures/inline-handler.html` | Native event-handler refusal and exact source repair |

`data/card.html` uses `text/html` and `https://cem.dev/ns/data/html/1`.
`data/card.cem` uses `application/cem` and `https://cem.dev/ns/cem-ml/1`, with an
explicit XHTML default namespace. `templates/card.cemt` is a native template,
`application/vnd.cem.template+cem` with
`https://cem.dev/ns/template/cem-native/1`. The graph uses
`https://cem.dev/ns/cli/transform-config/1`.
The live view uses the existing `application/vnd.cem.element-template+cem`
source identity and `https://cem.dev/ns/template/cem-element/1`, so validation
recognizes its runtime `datadom`, slice and declared-variable bindings. The
current open-content attribute warnings remain advisory.
`queries/card.xpath` declares `application/vnd.cem.xpath` with
`https://cem.dev/ns/query/xpath/1`; `expected/semantics.json` declares
`application/json` with `https://cem.dev/ns/data/json/1`. The component snapshot
uses `application/xml` with `https://cem.dev/ns/data/xml/1`. These are distinct
source identities and explicit reference exports, without a universal operation
support claim.

## Compare the two source forms

The XPath prefix belongs to the query execution context. Standalone XPath source
validation currently does not inherit caller namespace bindings, and standalone
XPath Parse/Inspect/format are not configurations here.

Both semantic configurations use an explicit query namespace binding,
`h=http://www.w3.org/1999/xhtml`. They select the article, heading, two
paragraphs and fragment link by expanded names. The contract is:

```text
title=Harbor field notes
title-id=card-title
labelled-by=card-title
summary=Reusable presentation starts with explicit data.
details=The saved source belongs to this project.
details-id=details
link-text=Read the notes
link-target=#details
paragraphs=2
```

The query explicitly accepts unqualified HTML attributes and the equivalent
default-namespace CEM attributes. It does not silently rename them. This is a
selected semantic contract, without a claim of byte equality or universal
HTML/CEM interchangeability. CEM reference diagnostics remain separate from
querying these literal attribute values: the current CEM semantic run retains
one advisory `cem.ast.unresolved_reference` for `card-title`, while returning
the nine expected values.

The sources are independent editable variants. Saving a heading change in the
HTML source changes **Card semantics from HTML**, the graph output and the live
view. It does not synchronize the CEM-ML resource; edit and save that variant
separately when comparing the changed contract. Unsaved drafts do not replace
the persisted command inputs.

## Build data-bound HTML

The graph imports saved `data/card.html`, applies the public `main` template in
`templates/card.cemt`, and exports the explicit entry artifact
`generated/card.html`. The template constructs an HTML article with the selected
heading, paragraphs and fragment link, plus **Rendered from saved HTML data.**
Its accessible heading reference uses the generated `rendered-card-title` ID.
The native engine imports and processes the source as a retained CEM tree;
neither a browser parser nor application JavaScript implements the mapping.

Select the project and choose **Build project** to use the shared output preview.
For source navigation, run the **Build the card as HTML** configuration, then
open **Commands and results → Results → Provenance** and select a row in
**Source-map provenance**. Generated ranges retain the authored template and
the source boundaries the native operation establishes. Computed values do not
imply arbitrary character-by-character lineage to the input.

Edit the saved HTML title or template caption and rerun to observe the new
artifact. Restore and save the original text to recover the exact expected
bytes. Native CLI equivalent from the project folder:

```sh
cem-ml transform --config graphs/card.cem --config-content-type application/cem \
  --config-schema https://cem.dev/ns/cli/transform-config/1 --scope-budget queueSize=1024
```

This output is an explicitly authored HTML projection. It does not compile a
whole page of DCE declarations or provide automatic DCE SSR/hydration.

## Reuse the shared declarative card

Run **Shared card live view**. The saved CEM-ML view reads the same admitted HTML
data through declarative `http-request`. A shared `cem-card` receives its heading
through the named `title` slot and its body through the default slot. A second
instance uses the same packaged declaration with different projected content.
The project does not copy the component implementation into a new custom tag.

Choose **Show details**. Existing `cem-action` slice bindings reveal the saved
details text and update the shared `cem-alert` to **Shared CEM interaction
received.** No project-authored JavaScript handles the action.

`components/cem-card.xhtml` is an inspectable snapshot of the canonical
[shared declaration](../../../packages/cem-components/src/components/cem-card/cem-card.xhtml).
It uses the generic XML identity for source inspection. Editing this project
resource does not replace the trusted runtime implementation. Live preview
loads only the packaged `cem-card`, `cem-action` and `cem-alert` declarations
named by the configuration's `preview-component:*` tags. Project-emitted
`cem-element` declarations and arbitrary new registration authority remain
outside this preview's policy.

The inspection command uses the default native CEM-ML source-AST presentation.
Line-fenced literal attributes preserve the complete template text, including
both quote styles and embedded rich-content delimiters, without evaluating it.
The original source-map fields remain available. An explicitly requested JSON
export remains a named presentation boundary, never a runtime tree handoff.
The Parse/Inspect command publication currently supplies no owned artifact-source
references to the Provenance panel. Its printed coordinate fields do not become
links; use `build-html`'s real output maps or the native policy diagnostic for
source navigation.

The live view owns an explicit light theme and no storage keys. Changing Studio's
theme does not persist an example preference. Its script-free frame denies
ambient network, storage and navigation; only admitted project resources and
shared component/theme assets are available. Static graph output and interactive
live view are separate presentations of the same saved HTML data.

## Failure, repair and offline copies

**Repair inline HTML policy** uses a separate invalid resource outside the graph
inputs. `onclick="return false"` is rejected with
`cem.html.event_handler_rejected` at line 1, column 39, original byte offset 38.
Remove the complete ` onclick="return false"` attribute, save and rerun the same
configuration. Its one error clears. XML/HTML source parsing never grants an
inline event-handler execution capability; the live example's action uses shared
declarative bindings instead.

After Studio's runtime, project resources and requested shared declarations have
been cached, saved copies reopen and execute offline. Each copy preserves its
own data, template, commands and snapshot edits. The catalog original remains
unchanged. This example contains no credential fields; entered passwords and
one-time codes elsewhere continue to block recovery until explicitly cleared,
and automatic whole-Studio recovery remains disabled.
