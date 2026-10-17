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
