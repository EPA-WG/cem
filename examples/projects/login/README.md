# Login project

Use this ordinary project to compare HTML/CEM form semantics, author native
queries and templates, and preview a shared-control Login view. The local form
does not authenticate an account or contact an identity provider.

Open any resource or configuration in the shared Studio workspace. Edit and save
`data/login.cem`, then use **Validate login**, **Parse login**, or **Inspect login**.
**Convert HTML to CEM-ML** uses `data/login.html` as its input.

## Resource identities

The declared identities in [example.json](example.json) select these contracts.
Display paths do not substitute for a resource's schema identity.

| Saved resources | Content type | Schema identity |
| --- | --- | --- |
| `data/login.cem`, `views/login.cem` | `application/cem` | `https://cem.dev/ns/cem-ml/1` |
| `data/login.html`, `expected/view.html` | `text/html` | `https://cem.dev/ns/data/html/1` |
| `data/login-invalid.cem` | `application/cem` | `urn:demo:login` |
| `schemas/login-schema.cem` | `application/cem` | `https://cem.dev/ns/schema/1` |
| `schemas/login-schema-package.cem` | `application/vnd.cem.schema-package+cem` | `https://cem.dev/ns/schema-package/1` |
| `queries/semantics.xpath` | `application/vnd.cem.xpath` | `https://cem.dev/ns/query/xpath/1` |
| `queries/semantics.cemql` | `application/vnd.cem.query+cem-ql` | `https://cem.dev/ns/query/cem-ql/1` |
| `queries/summary.cem-ql` | `application/vnd.cem.query-expression+cem-ql` | `https://cem.dev/ns/query/cem-ql/1#expression` |
| `templates/view.cemt` | `application/vnd.cem.template+cem` | `https://cem.dev/ns/template/cem-native/1` |
| `graphs/view.cem` | `application/cem` | `https://cem.dev/ns/cli/transform-config/1` |
| `expected/semantics.json`, `expected/expression.json` | `application/json` | `https://cem.dev/ns/data/json/1` |

The invalid Login resource exercises the project-owned instance schema. The
generic CEM-ML source contract and that instance rule are separate checks. The
live view's current generic identity has the source-operation limit described
below; these identities do not promise every operation for every resource.

## Configuration index

These are stable configuration IDs in the same project. Their Studio labels and
workflow details appear below; generated configuration resources are ordinary
saved project resources.

| Configuration ID | Studio label | Expected result |
| --- | --- | --- |
| `read-edit` | Parse login | Native CEM-ML AST presentation |
| `validate` | Validate login | Zero errors and three unresolved-reference warnings |
| `inspect` | Inspect login | Native structural CEM-ML presentation |
| `format-pretty` | Format Login (pretty) | Expanded plain CEM-ML source |
| `format-tabular` | Format Login (tabular) | Tabular plain CEM-ML source |
| `source-colors` | Export Login source colors (HTML) | Escaped source-color HTML |
| `html-conversion` | Convert HTML to CEM-ML | Native CEM-ML source export |
| `validation-repair` | Validate form method | `demo.login.method_required`, then repair to zero errors |
| `semantics-html` | Login semantics from HTML | The 21-field `expected/semantics.json` contract |
| `semantics-cem` | Login semantics from CEM-ML | The same 21-field contract |
| `semantics-html-cemql` | Login semantics from HTML (CEM-QL) | The same 21-field contract |
| `semantics-cem-cemql` | Login semantics from CEM-ML (CEM-QL) | The same 21-field contract |
| `render-html` | Render Login from HTML | HTML matching `expected/view.html` |
| `render-cem` | Render Login from CEM-ML | The same expected HTML |
| `render-graph` | Render Login graph | `generated/login.html`, matching the expected HTML |
| `live-view` | Login live view | Shared-control light-DOM view, with the source-identity limit below |
| `expression-html` | Evaluate Login expression (HTML) | `title=Sign in` and `required=2` |
| `expression-cem` | Evaluate Login expression (CEM-ML) | The same `expected/expression.json` contract |
| `expression-format-pretty` | Format Login expression (pretty) | Saved standalone expression source |
| `expression-format-tabular` | Format Login expression (tabular) | The same authored expression layout |

## Format the saved source

**Format Login (pretty)** and **Format Login (tabular)** both read saved
`data/login.cem` through native Convert with the exact CEM-ML identity. Pretty
uses expanded closing lines; tabular compacts adjacent closing braces. The two
layouts retain the same 21-field semantic contract used by the comparison queries.
Results provides plain CEM-ML for copy/download, without ANSI escapes. Formatting
produces a run result and never overwrites the source resource.

Change the first `Sign in` heading to `Bienvenue 🍒`. Run before saving to see the
persisted version; then **Save and validate** and rerun either format configuration
to see the change. Restore and save the original heading to recover the baseline.
Saved sources and commands work offline and remain independent in each copy.

**Export Login source colors (HTML)** uses the native tabular writer with
`--output-color-type html-css-vars`. Copy/download contains escaped CEM-ML inside
`<pre>` and role-tagged spans, with HTML output identity and inherited
`--cem-color-*` properties plus the shared palette fallbacks. Open the exported
HTML to see source colors; it contains source text, not an active Login form.
Studio's existing sandboxed HTML preview blocks styles and scripts, so colors
are not displayed there. The plain configurations still request `none`.
The approved external slide preview remains unchanged.

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

Open `queries/semantics.cemql` in Structure, edit it, **Save and validate**,
then select a CEM-QL configuration and **Run configuration**. For example, rename
`local:field("inputs", seq:count(inputs))` to use `"input-count"`; only that
result field changes. Restore and save the original to recover the baseline.
The query host's `input` binding is recognized during validation; misspelled
variables still fail. Query resources and source edits remain project-local.

With `queries/semantics.cemql` selected, use **Parse persisted revision** for the
native AST or token events. **Inspect persisted revision** offers summary, AST,
events, diagnostics, source offsets and tree views. These commands inspect saved
source without running it or binding query inputs. The AST retains declaration
and expression structure, source URI and UTF-8 byte ranges; events include trivia.
Results are tabular CEM-ML. Unsaved edits do not change persisted inspection.

The native adapter also accepts the standalone-expression content type/schema,
without adding a module wrapper. Invalid syntax remains inspectable and mismatched
identities produce diagnostics. Parse keeps recoverable diagnostics under its
default policy; use `--fail-level validate` in a configuration when errors should
produce a nonzero exit. Native expression formatting is available through an
ordinary Convert configuration: use
`application/vnd.cem.query-expression+cem-ql` with
`https://cem.dev/ns/query/cem-ql/1#expression` for both input and output. It retains
comments and Unicode, supports the existing formatter/line-ending options, and
rejects invalid source without inventing a module. Module formatting, validation
and execution remain available.

## Standalone expression evaluation and formatting

Login 2.8.0 has twenty ordinary configurations. Open `queries/summary.cem-ql`
in Structure, or select **Format Login expression (pretty)** or
**Format Login expression (tabular)** to edit it directly. This is one standalone
CEM-QL expression with the explicit expression content type/schema, not a module.
Its comment, local bindings and Unicode remain in the source.

**Evaluate Login expression (HTML)** and **Evaluate Login expression (CEM-ML)**
read their saved Login input and the same saved expression dependency. Both return
`title=Sign in` and `required=2`, matching `expected/expression.json`. The expression
selects HTML elements by expanded namespace and counts required attributes by
presence. The configurations request `--ready-names` and the existing bounded
`queueSize=1024` worker policy. They inspect the example's native source tree;
they do not validate credentials or submit the form.

Change `"title="` in the expression to `"titre 🍒="`. Run a format configuration
before saving: the result still contains the persisted `"title="`. **Save and
validate**, then run either evaluation configuration to see `titre 🍒=Sign in`.
Editing and saving the input heading changes its value independently. Restore
and save the expression to recover the baseline. Each project copy owns its
source, expression and command revisions, including after offline reload.

With the expression selected, **Parse persisted revision** and **Inspect persisted
revision** show its native structure and source locations without evaluating it.
The two format configurations also do not evaluate or overwrite the expression.
They preserve comments, Unicode and expression identity and provide plain source
for copy/download. CEM-QL's current pretty and tabular profiles preserve the
expression's authored layout; this sample does not promise distinct layouts.
Invalid syntax is reported; repair and save before rerunning.

## Generate the Login view

**Render Login from HTML**
and **Render Login from CEM-ML** apply the shared `templates/view.cemt` resource
to the saved source. Both produce the complete HTML in `expected/view.html`:
an accessible heading and form, associated labels, autocomplete hints, required
controls and the template-owned instruction. Results uses the existing shared
sandboxed document preview and provides exact HTML for copy/download. The separate **Login live view** configuration provides interactive light-DOM rendering.

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

The expected HTML pins the native writer's exact output bytes, including its
three final spaces. The former fixture's final newline was stale; the native
HTML, CEM-ML and graph configurations produce the same exact artifact.

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

`views/login.cem` declares the registered
`application/vnd.cem.element-template+cem` source type and
`https://cem.dev/ns/template/cem-element/1` schema. The **Login live view**
configuration uses these same selectors, so native Parse and Validate recognize
runtime `datadom`, slice and declared-variable bindings without evaluating the
live view. Both operations retain the 15 advisory
`cem.schema.unknown_html_attribute` warnings; no hard binding errors are waived.
The source editor uses the shared native CEM-ML colorer for this media type.
The CLI and Studio's existing validation host still differ for an intentionally
undeclared expression variable: CLI Validate rejects it, while the worker
currently defers that error. This separate [semantic-validation follow-up](../../../docs/todo.md)
does not make worker success proof of strict binding conformance. Malformed CEM
source is rejected on both paths.

`data/login.cem` remains generic CEM-ML data, and `templates/view.cemt` remains a
native build template. These are separate source contracts. Project 2.8.1 keeps
all resource/configuration IDs, paths and authored template bytes. Newly cloned
projects use the corrected selectors. Existing saved copies, commands and
drafts retain their own identities; they are not migrated automatically. A saved
copy can adopt both selectors explicitly. Generic CEM Parse still reports its
original hard binding diagnostics for this template; a zero default Parse exit
alone does not establish source validity. The
[shared card project](../html-cem-card/README.md) uses the same element-template
contract.

Open **Login live view** in an ordinary Studio project copy. Its configuration parses
`views/login.cem`, then the shared `cem-media-preview` renders that saved template
in a script-free frame using `cem-element` light DOM. The configuration's explicit
`live-preview` tag opts a template resource into this presentation. `preview-component:<tag>` tags
load only those individual shared declarations for declaration and CSS preflight,
while retaining the complete shared theme. List transitive component dependencies
explicitly for dependent assets; live preview still refuses template-emitted
`cem-element` declarations rather than granting new registration authority. The view reads
`data/login.html` through the native resource importer and uses shared text fields,
action and alert controls. The HTML/CEM comparison and graph remain independently
runnable configurations.

Edit the local action label, **Save and validate**, then **Run configuration**.
Change the HTML source's first `Sign in` heading, save, and run the live view again.
Unsaved drafts never replace persisted execution inputs. Reload works offline;
a fresh project copy keeps the original view and data. Fields and the local action
show transient interaction only: credentials are neither submitted nor saved.
The example owns its light theme; it does not read or write Studio's theme key.

Entered passwords and one-time codes block recovery until the user explicitly
clears them. They are not exported in checkpoints or project bundles. Automatic
whole-Studio recovery remains disabled. Shared declarative controls own the
interaction; the project adds no JavaScript behavior or credential persistence.

Parse/Inspect displays native coordinates in its structural presentation, but
currently publishes no owned artifact-source references to Studio's Provenance
panel. The graph provenance described above is a separate native output contract;
printed source IDs and byte ranges must not be turned into links by the example.
