# CEM schema authoring

This ordinary Studio project defines a small note contract, packages it as a
project-owned schema and applies it to a separate CEM instance. Open it from
**Content types and schemas → Schema authoring**. Clone the read-only example
before editing; all operations use the shared source editor and command controls.

## Authored identity reference

These are resource identities, including expected artifacts. The configurations
below determine their supported uses. An identity alone does not grant an
operation.

| Resource role | Content type | Schema |
| --- | --- | --- |
| CEM schema source | `application/cem` | `https://cem.dev/ns/schema/1` |
| Note domain instances | `application/cem` | `urn:example:note` |
| CEM schema packages | `application/vnd.cem.schema-package+cem` | `https://cem.dev/ns/schema-package/1` |

## Resources and configurations

| Configuration | Input | Purpose |
| --- | --- | --- |
| `validate-schema` | `schemas/note.cem` | Validate native schema declarations, attribute types, the required-id field contract and diagnostic binding. |
| `inspect-schema` | `schemas/note.cem` | Inspect the imported native CEM source structure and original ranges. |
| `format-schema` | `schemas/note.cem` | Run native tabular formatting and compare `expected/formatted-schema.cem`. |
| `validate-package` | `schemas/package.cem` | Validate the local schema-source declaration, namespace claim and content-type metadata. |
| `validate-instance` | `data/note.cem` | Load the owned package and apply the note domain contract. |
| `repair-schema` | `failures/missing-namespace.cem` | Restore the required namespace declaration. |
| `repair-instance` | `failures/missing-id.cem` | Add the id required by the loaded note schema. |

The schema source uses `https://cem.dev/ns/schema/1`; its package uses
`https://cem.dev/ns/schema-package/1`. The domain identity is
`urn:example:note`. A package namespace claim associates that domain with its
owned metadata; the schema URI and namespace claim are distinct declarations.
The package resolves `note.cem` relative to its own location. No remote dependency,
network resolver or replacement authority is enabled by this project.

Instances are explicitly CEM syntax (`application/cem`) with the domain schema.
The package's custom content-type declaration is metadata; it does not install a
new parser or grant an executable adapter for arbitrary custom media types.
Schema-source validation, package-source validation and instance validation are
three different operations. A valid schema source alone says nothing about an
instance. The required-id rule is a supported native field contract, not an
application JavaScript check or a claim of complete schema-language support.

## Failure, repair and saved edits

`repair-schema` reports `cem.schema_model.missing_required_attribute` at the
original buffer start. Restore `@namespace=urn:example:note` on the schema root.
The resulting bytes match `schemas/note.cem`.

`repair-instance` loads both project-owned schema resources and reports
`example.note.id_required`. Add `@id=welcome` on the note root; the repaired bytes
match `data/note.cem`. Its location is the native contract's original-buffer
position, not an invented range for the absent attribute.

To test schema edits, change the diagnostic message in `schemas/note.cem` from
`A note must declare its id` to `Saved schema requires a note id`, save it, and
rerun a missing-id instance. The new native diagnostic message proves the saved
schema revision is used. Restore the id and validate again. Changing the text
`Café notes` in a valid instance independently leaves schema-source validation
unchanged. Clone again to edit without changing the first saved project.

No application artifact or live Login view is created. Source projections retain
native URI/ranges as presentation facts; common Inspect currently publishes no
owned artifact-source references to Studio's Provenance selector. Actual native
diagnostics remain separate source-selection inputs. Formatting creates text
and does not claim arbitrary instance character lineage. Saved resources and
native operations are available offline
after the ordinary Studio cache is installed. Acceptance uses isolated browser
storage and temporary copies; existing user projects and drafts are preserved.
Entered passwords and one-time codes continue to block recovery until cleared.
Automatic whole-Studio recovery remains disabled.
