# Studio content-type catalog folders, 2026-10-24

- [x] Author the Content types and schemas hierarchy in
  [collection metadata](../../examples/projects/collections.json): Data,
  Documents and UI, Queries and transformations, and Schema authoring. Emit
  parent relationships/descriptions into the existing version-1 sample index.
  Preserve Amsterdam and the legacy `collection=examples` discovery folder.
- [x] Place the existing Supplier, Login, Markdown site and Schema examples
  projects in applicable categories. Reuse one archive/provider per project;
  preserve project/resource/configuration IDs, archive bytes and hashes. Carry
  project descriptions through the generated catalog into shared folder listings.
- [x] Build declared nested/empty folders in the navigation domain model and
  render them through existing shared recursive trees and folder listings. Every
  membership gets a unique `collection=<id>&sample=<id>` occurrence. Preserve
  explicit context through files, configurations and output routes; keep bare
  sample/file URLs unchanged and reveal their first discovery membership.
- [x] Reject false memberships and conflicting saved-project/view contexts before
  loading projects. Preserve selection through collapse and normal publications.
  Share pending archive loads by project ID, retry failed loads and copy cached
  bundles before creating independent saved clones.
- [x] Validate duplicate identities, missing parents, cycles and malformed
  memberships before lazy archive loading. Snapshot/freeze accepted metadata.
  Preserve opaque collection IDs, including spaces, slashes and Unicode, without
  trimming/normalizing identity; reject empty/control-containing IDs. Legacy
  catalogs without collection definitions retain implicit flat folders.
- [x] Add hierarchy/metadata/navigation fixtures, including 21 malformed cases,
  nested empty groups, unique occurrence keys, legacy/contextual URLs, collapse,
  lazy/shared archive loading, independent clones, unchanged originals and
  retained drafts. Extend deployed acceptance with offline category reload and
  existing shared read-only source views. Include authored collection definitions
  in the Feature Tour verifier's cache inputs.
- [x] Document discovery/authoring and the ordinary project contract. Keep UI in
  shared declarative components; no new UI JavaScript capability, component
  styles, sample screens or native parser are introduced. Entered credentials
  still block recovery, and automatic whole-Studio recovery remains disabled.

## Verification

The final provider suite passes all six browser cases, including opaque-ID
compatibility and the 21 malformed metadata cases. Native Feature Tour
verification passes all 32 manifest-selected source validations, ordinary project
bundle/configuration checks, Supplier/site builds, artifacts and provenance.
Generator drift, Studio typecheck, authored source/script lint and whitespace
checks pass. The final graph deployment verifies 218 declared assets and 313 files.

The deployed navigation verifier covers category listings/descriptions,
contextual/legacy selection, collapse, read-only source, Create/Clone/Delete,
unchanged originals, two independent saved copies, retained drafts, history,
reload and offline category/source navigation. It retains theme, responsive
320/390/900/1440px, forced-color, Supplier and Markdown site acceptance. Generated
screenshots remain under `/tmp`; the actual browser session, projects and drafts
are not cleared.

The final workspace/navigation/recovery cohort passes all 143 browser cases
across three files, including all eight recovery/credential-policy cases. It
retains the original recovery budgets/deadlines and completes in 735.8 seconds.
The final exact-source deployed navigation replay also passes, including the
ordinary Markdown site's editing, intentional failure/repair, independent copy,
source provenance and offline rebuild. Reports remain generated verification
outputs rather than committed source.

## Next work

Implement the JSON/YAML payload/configuration/report projects, followed by the CSV
inventory/header/quoting failure-and-repair project. Extend/reuse Supplier for
XML/XHTML workflows. Keep source validation distinct from instance validation
and execution as specified by [the audited inventory](../studio-content-types.md).
