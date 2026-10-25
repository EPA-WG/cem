# HTML/CEM card and shared component workflows, 2026-10-25

- [x] Add the ordinary [HTML and CEM card project](../../examples/projects/html-cem-card/README.md)
  with 11 source resources and eight configurations under Documents and UI and
  Queries and transformations. Reuse Login for the larger form semantic contract
  and credential-policy example.
- [x] Compare nine selected fields in equivalent HTML/CEM sources using one
  namespace-aware query. Build an exact HTML entry artifact from saved HTML data
  through a native CEMT template. Use the same saved HTML data in a live CEM view.
- [x] Compose the production `cem-card` declaration twice using its named title
  and default body slots. Reuse shared action/alert components and declarative
  slice bindings for local interaction; add no sample or application UI JavaScript.
- [x] Expose an unchanged snapshot of the canonical shared declaration as an XML
  resource with an explicit JSON AST inspection configuration. Preserve component
  registration authority and document the existing CEM-ML writer bound.
- [x] Add native configuration, exact-output, diagnostic/repair, independent-copy
  and source-provenance fixtures. Add worker and deployed acceptance for saved
  sources, live interaction, declaration refusal, native builds and offline use.
- [x] Preserve explicit Inspect format through the shared native request and
  worker publication, retaining legacy CEM-ML defaults and named JSON exports.
- [x] Repair shared projected slice-event ownership, including initial dispatch,
  movement and surface-grant revocation, with failing browser regressions first.
- [x] Include the canonical declaration in Studio archive generation/verification
  and native test cache inputs. Regenerate normal portable archives and update
  the checked content-type inventory and Studio guide.

## Source and runtime contracts

The equivalent HTML/CEM sources expose a heading, its accessible reference, two
paragraphs and a fragment link. The comparison maps unqualified HTML attributes
and explicit default-namespace CEM attributes by expanded names; it does not
claim universal interchangeability. Both sources remain independently editable.
Changing the HTML source updates its query, native build and live view; it does
not rewrite the CEM resource.

`graphs/card.cem` imports the HTML data and applies `templates/card.cemt` to
produce `generated/card.html`, an explicit entry artifact. The expected output
is exactly 299 bytes. Template instructions retain their original URI/ranges,
including after Unicode input and caption edits. Computed output does not claim
arbitrary character lineage to the input. This is an authored HTML projection,
not a whole-page DCE compiler or automatic hydration implementation.

The live view uses the existing element-template identity,
`application/vnd.cem.element-template+cem` with
`https://cem.dev/ns/template/cem-element/1`. This recognizes its runtime bindings
without the hard unknown-variable errors caused by generic source parsing.
The current source validator retains 18 advisory open-content warnings; the
literal CEM data also retains one advisory unresolved-reference warning.
Neither warning class is suppressed or promoted into an execution grant.

The canonical `cem-card` wrapper is an XML element declaration, not a complete
XHTML `html/head/body` document. Its entire embedded template text, including
both quote styles and rich-content fences, remains intact. The current CEM-ML
attribute-string writer deliberately refuses that mixed-quote text. This one
configuration requests JSON as a named human-facing structural export; no JSON
AST becomes a runtime input. Lossless general CEM-ML encoding remains an explicit
native follow-up in the active TODO.

Editing the declaration snapshot does not replace the trusted packaged component
used by live preview. The host continues to preload only explicitly named shared
declarations; template-emitted `cem-element` remains refused. The view owns its
light theme and transient notice without reading or writing Studio's theme key.
The preview remains script-free, network-denied and limited to admitted sources.

The intentional HTML failure is `cem.html.event_handler_rejected`, line 1,
column 39, UTF-8 byte offset 38. Removing the inline `onclick` attribute repairs
the same saved source. Import/build refusal prevents publishing that invalid
HTML. Shared declarative event bindings provide the working interaction instead.

## Explicit Inspect presentation through shared workers

The first browser-worker run exposed an independent transport gap: native CLI
honored `inspect --format json`, while shared invocation lowering omitted the
choice and operation preparation always requested the CEM-ML writer. A native
lower/prepare regression reproduced the lost field before the repair.

Portable Inspect now carries an optional typed `format`. Missing legacy fields
and explicit `cem` retain CEM-ML presentation. Explicit `json` bypasses the CEM-ML
presentation writer and publishes pretty JSON with the correct output identity;
stdout has the native CLI's final newline and file output retains its native
bytes. Unsupported format names are rejected. The typed result envelope and
native source remain unchanged, and the existing mixed-quote writer refusal is
preserved. This is a shared native command contract, with no Studio formatting
or parser workaround.

## Projected slice-event ownership

The deployed live view exposed an existing shared ownership gap: a genuine click
on the action projected into `cem-card` wrote to the receiving card instead of
the view that authored the action. A minimal shared ownership story reproduced
the failure before the runtime repair.

Slice bindings now trace renderer-issued identity witnesses through the original
inert payload to find their author. Public attributes cannot claim an owner.
Only slice-event binding uses this logical route; physical form, widget and
delegate ownership remains unchanged. Projected native and component actions,
named/default slots and child/parent rerenders share the same rule. Callbacks
retain weak owner references and recheck routing before writing. Initial render
events preserve their existing pre-publication path through an exact internal
event identity, without accepting a forged event-type bypass.

This repair belongs in `cem-elements`. Studio and the card example retain their
declarative bindings, with no application event handler or component workaround.

## Verification

The selected native suites pass **24 tests**: ten shared Inspect tests and 14
CLI cases across the new project and the existing Login examples.

```sh
cargo test -p cem-ml inspect_ --lib
cargo test -p cem-ml-cli --test html_cem_card --test amsterdam_samples \
  --test login_template --test login_graph
```

The shared Inspect cohort covers lower/wire/prepare/engine execution, old and
explicit formats, unsupported names, output identity and CLI-compatible
publication bytes. The CLI cohort covers all eight card configurations, complete
canonical template text, exact nine-value semantics and HTML output, native policy
repair/refusal, independent copies and original template ranges after Unicode
edits. Existing Login semantic, schema repair, rendering and graph cases pass.

Fresh native CLI, shared command-service, rendering and owned runtime packages
build successfully, including the backward-compatible optional protocol typing.
Native Feature Tour verification passes all 32 schema examples and the authored
project configurations. The checked inventory now has 56 workflow mappings.
Output verification passes 218 declared assets and 317 files; deterministic
builds match SHA-256
`07672e8073de1c4dc263cd14422568d69390f0a2a96d8a72b0752836bd92c128`.

All **25 Feature Tour browser tests pass** against the rebuilt packages in
61.40 seconds, including all eight card commands and the exact HTML/semantic
contracts. The existing **nine preview projection browser tests also pass**.

The shared runtime's focused ownership, slot, surface and deferred-body cohort
passes **24 browser cases**. Projected clicks, initialization before publication,
movement and contained-body grant revocation each have a verified failing case
before their repair. The final ownership/surface cohort passes all 15 cases after
the revocation adjustment and test fixture guards. Physical widget/form ownership
is unchanged. Shared runtime typecheck uses the exact native-generated CLI schema
input; the known standalone preparation gap remains a separate TODO. The temporary
generated input is removed after checking. Authored Studio lint/typecheck and
whitespace checks pass. Shared runtime lint has zero errors and 13 existing
non-null-assertion warnings; the new story has no warnings.

The dedicated `@epa-wg/cem-studio:verify:html-cem-card` gate passes with all 25
prerequisites. All 15 evidence flags in
`dist/reports/cem-studio/html-cem-card.json` are true: catalog/read-only originals,
semantic equivalence, native policy repair, exact static entry, shared live cards,
genuine interaction, unsaved-source guard, saved data/view changes, declaration
snapshot authority limits, nested declaration refusal, theme isolation, native
provenance, independent copies and offline reopening/live reruns/builds. The
shared controls are exercised through genuine input without clearing user data.

The rebuilt packaged Node command-service worker also passes canonical XML
inspection through `dist/bin.js inspect ... --show ast --format json`. Its
export preserves the complete original embedded template text and the CLI
stdout newline; it does not pass through the CEM-ML string writer.

## Preserved policies and next work

Acceptance uses isolated browser storage and temporary project copies. Existing
saved projects and drafts are not cleared, and editing copies leaves original
sample sources and intentional failures unchanged. Generated screenshots, Vitest
attachments, binaries and diagnostic reports remain outside Git.

Entered passwords and one-time codes still block recovery until explicitly
cleared. Automatic whole-Studio recovery remains disabled. No component/theme
tokens, preview grants or application UI implementation changes are introduced.

Continue with SVG/MathML inspection and preview, CSS/SCSS/selector workflows,
then CEM-QL/XPath data workflows in the active TODO.
