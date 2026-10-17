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
