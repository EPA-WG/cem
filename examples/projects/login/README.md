# Login project

Open any resource or configuration in the shared Studio workspace. Edit and save
`data/login.cem`, then use **Validate login**, **Parse login**, or **Inspect login**.
**Convert HTML to CEM-ML** uses `data/login.html` as its input.

## Intentional validation failure and repair

`data/login-invalid.cem` is syntax-valid but omits `@method=post` from the form.
**Validate form method** loads the schema package and schema in `schemas/`.
Run **Validate persisted revision**, then select `demo.login.method_required` in
Diagnostics to locate the form. Restore `@method=post` beside
`@cem:form="sign-in"`, save, and validate again. An unsaved repair does not change
persisted validation. Remove the attribute and save to reproduce the failure.

The invalid source produces one error and three existing unresolved-reference
warnings. The repair removes the error; the three warnings remain. This teaching
schema requires a method attribute. It is not a complete HTML schema.

This project belongs to the Amsterdam 2026 catalog collection. It uses the same
resources, configurations and persistence as every other Studio project.

## HTML/CEM semantic comparison

Run **Login semantics from HTML** and **Login semantics from CEM-ML**. Both use
`queries/semantics.xpath` and should produce the same 21 named string values in
`expected/semantics.json`. These are ordinary query configurations with shared
resources; no special example screen or comparison renderer is involved.

The contract checks the screen/form/action annotations, heading and its accessible
reference, form method and action, both named input types/autocomplete/required
attributes, label associations, submit type/text, and input count. Edit and save
the heading or form method in either source, rerun its configuration, and observe
the changed field. Unsaved edits do not alter a persisted query result. Restore
and save the source to recover the baseline contract.

The query selects the XHTML `main` subtree explicitly. It compares CEM namespace
annotations with HTML's literal `data-cem-*` mirrors; import does not silently
rename those attributes. It also maps unqualified HTML attributes to CEM's
default-HTML-namespace attributes; the native expanded names remain distinct.
HTML document/head/body wrappers, doctype, indentation,
quote style and attribute order are outside this comparison. This proves the
named Login contract, not byte equality or universal HTML/CEM interchangeability.
The XML slide projection is a separate input and is not substituted for HTML.

Both inputs are processed as retained typed CEM trees. HTML uses its registered
native parser/recovery and validation policy, retaining decoded values and
original source ownership/ranges. JSON is the explicitly requested serialization
of the scalar query result; it is not an intermediate document or AST handoff.

## Equivalent XPath and CEM-QL queries

The four query configurations share the same 21-field semantic contract. The two
**(CEM-QL)** configurations use `queries/semantics.cemql`, an executable query module with
local element, attribute, annotation, label and field helpers. Run them beside
the original XPath configurations: both languages produce the same 21 values
from either source, including after saved edits. Boolean `required` is tested
by attribute presence, so a present empty value differs from an absent attribute.
Renaming a namespace prefix does not change the contract.

CEM-QL configurations explicitly request `--ready-names`. This consumes ready
literal namespace metadata over the original native source without evaluating
pending namespace values or resolving source references. Ordinary CEM-QL source
queries keep their authored-name view. The module preserves expanded namespaces
and makes the same HTML-mirror/default-attribute mappings as the XPath query.

The configurations request a bounded `queueSize=1024` navigation budget. Studio's
trusted worker host supplies that ceiling; request data cannot raise a stricter
host limit. Global defaults remain unchanged. The common browser runtime links
CEM-QL adapters into the same WASM binary and retains the native source owners.

Open `queries/semantics.cemql` through Resources, edit it, **Save and validate**,
then select a CEM-QL configuration and **Run configuration**. For example, rename
`local:field("inputs", seq:count(inputs))` to use `"input-count"`; only that
result field changes. Restore and save the original to recover the baseline.
The query host's `input` binding is recognized during validation; misspelled
variables still fail. Query resources and source edits remain project-local.

Generic source Parse/Inspect for CEM-QL and standalone-expression formatting
remain separate native adapter work. This project uses the supported executable
module identity, native module formatting, validation and query execution.

## Generate the Login view

New version-2.5.0 copies have thirteen configurations. **Render Login from HTML**
and **Render Login from CEM-ML** apply the shared `templates/view.cemt` resource
to the saved source. Both produce the complete HTML in `expected/view.html`:
an accessible heading and form, associated labels, autocomplete hints, required
controls and the template-owned instruction. Results uses the existing shared
sandboxed document preview and provides exact HTML for copy/download. A live
light-DOM Studio view is tracked separately.

Edit a heading, label or form method, **Save and validate**, then rerun its render
configuration. Unsaved changes do not replace the persisted input. Open
`templates/view.cemt` through Resources and change **Use your email and
password.** to **Use your Amsterdam account.**, save and validate it, then rerun
either render configuration. Only the instruction changes. Reload restores the
exact saved template; restore the original instruction to recover the baseline.
New copies have independent source and template revisions.

This is an explicitly authored Login projection. It selects the `main` element
by local name, then children in the same source namespace and attributes that
are unqualified or share their owner's source namespace. It keeps the native
source-oriented view and does not expand or guess namespace bindings. Renamed
producer prefixes therefore keep working. Core annotations, HTML `data-cem-*`
mirrors, document wrappers and source declarations are outside this generated
view. Required controls use attribute presence; absent attributes are omitted.
The shared native writer escapes source text and attribute values. The template
does not execute source references or supply an authentication service.

The configurations use the existing bounded `queueSize=1024` worker policy and
native CEMT adapter. No application UI JavaScript or additional renderer is
introduced. Existing project copies keep their saved contents; create a fresh
Login copy to receive these resources and configurations.

## Transformation graph and source provenance

**Render Login graph** opens `graphs/view.cem` in the same workspace. It imports
saved `data/login.cem`, applies the public `main` template from
`templates/view.cemt`, and exports `generated/login.html`. Its HTML matches
`expected/view.html`. Generated output stays a run artifact; it does not silently
create a source resource.

Use **Graph configuration** to choose **page (export)** and its **out** attribute.
Set it to `../generated/presentation.html`. The native graph editor stages a
validated draft; Undo/Redo and Reload use the usual workspace behavior.
**Run configuration** uses the saved graph. **Save and validate**, then rerun to
change the output destination. Saved source and template edits also affect this
configuration; relative dependencies resolve inside each project copy.

In **Results → Provenance**, rows connect generated UTF-8 byte ranges to an
explicit source URI and byte range. Select an instruction row to open
`templates/view.cemt` and select its authored text. Resource routing uses stable
resolver addresses even after a display-path rename. Unsaved drafts use the
normal Save/Discard/Cancel guard. Saving changes the project revision, so rerun
before following an older result. Missing resources, stale revisions and invalid
or split UTF-8 ranges cannot select an unrelated source span.

Native source IDs alone are local to their owners and do not identify project
resources. The shared engine carries explicit URI boundaries through template
compilation, portable development artifacts and binary source maps. Constructed
or computed output points to its authored template instruction/expression; it
does not claim per-character data lineage. Copied native input nodes retain their
own source URI/ranges. Production template artifacts still strip source maps.

### Login live view

Open **Login live view** in an ordinary Studio project copy. Its configuration parses
`views/login.cem`, then the shared `cem-media-preview` renders that saved template
in a script-free frame using `cem-element` light DOM. The configuration's explicit
`live-preview` tag opts a template resource into this presentation. `preview-component:<tag>` tags
select its shared controls for declaration and CSS preflight. The view reads
`data/login.html` through the native resource importer and uses shared text fields,
action and alert controls. The HTML/CEM comparison and graph remain independently
runnable configurations.

Edit the local action label, **Save and validate**, then **Run configuration**.
Change the HTML source's first `Sign in` heading, save, and run the live view again.
Unsaved drafts never replace persisted execution inputs. Reload works offline;
a fresh project copy keeps the original view and data. Fields and the local action
show transient interaction only: credentials are neither submitted nor saved.
The example owns its light theme; it does not read or write Studio's theme key.
