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

New version-2.2.0 copies have nine configurations. The two **(CEM-QL)**
configurations use `queries/semantics.cemql`, an executable query module with
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
