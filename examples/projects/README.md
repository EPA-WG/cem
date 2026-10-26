# Studio example projects

Every directory containing `example.json` defines an ordinary portable Studio
project. The generator discovers directories deterministically; adding an example
does not require a Studio route or renderer.

The [content-type and schema inventory](../../docs/studio-content-types.md) maps
all registered package identities and selectors to existing validation resources,
functional configurations and planned projects. Consult its operation limits
before adding a sample: source validation, instance validation and execution
are separate capabilities.
The [catalog coverage audit](../../docs/studio-catalog-coverage.md) separates
declared primary inputs, packaged dependencies and expected reference artifacts
from native, worker and deployed verification. A registered alias or available
browser command does not establish per-format execution support.

- `id`, `name` and `version` identify the catalog template. Optional `collections`
  mark presentation or subject membership, such as `amsterdam-2026`.
- `sources` declare stable resource IDs, repository source paths, project paths,
  content types, schemas and roles. Include README resources for instructions.
- `configurations` declare names, kinds, source/dependency resource IDs and native
  authored command arguments. `$source` and `$<dependency-id>` are replaced with
  project-relative resource addresses during generation. Expected results and
  repair metadata support native fixtures; they do not drive a tutorial UI.
  Optional `entryArtifact` sets a named build's project-relative preview output;
  it is copied into the portable configuration, independently of input resources.

Every project includes `README.md` as an ordinary resource. Its instructions
name each owned configuration ID and every non-documentation content type/schema
identity in exact inline code, with typical uses, expected artifacts, original
source-location boundaries, repairs where applicable and current native limits.
Group identities by source/reference role; listing a golden output does not
make it an admitted input. Preserve credential privacy: entered passwords or
one-time codes block recovery until cleared, and automatic whole-Studio recovery
remains disabled. Documentation changes never overwrite existing saved copies.

Direct source-validation arguments generate a native run configuration. An
explicit `validate --config $source` instead keeps the versioned command envelope
and executes the authored run-configuration resource with its declared inputs.
Editing that resource can repair validation policy, such as CSV's header mode;
validating the configuration's own source is a separate operation.
Native invocation resolves its relative input and schema-package addresses from
the project working directory, preserving declared addresses and effective
identities in the plan. Resource loading remains limited to admitted resources.

[collections.json](collections.json) defines discovery folders with stable `id`,
`name`, `description` and optional `parentId`. The **Content types and schemas**
folder contains Data, Documents and UI, Queries and transformations, and Schema
authoring. A project may belong to several folders while retaining one archive,
project identity, resource set and set of configurations. Amsterdam membership
stays independent from these subject categories. Declared empty folders remain
visible so a collection can grow without placeholder projects.

Opening a project from a folder uses `?collection=<folder-id>&sample=<project-id>`;
file/configuration links carry that context. Existing `?sample=<project-id>`
bookmarks continue to work and highlight the first membership in the tree. The
legacy `?collection=examples` folder remains available. Context must be a direct
project membership. Catalog loading checks duplicate identities, missing parents,
cycles and memberships before fetching any archive; catalogs without collection
definitions retain their legacy implicit flat folders.

Generated bundles use the native version-1 project contract (`entries` and
`runConfigResourceId` are compatibility wire fields). Studio verifies archive
hashes and imports editable copies through the same repository as user-created
and imported projects. Copies have independent project identities and preserve
resource IDs and reference paths. Archive updates never overwrite saved copies.

The schema-package example project is generated from package manifests using
the same archive/catalog contract. Run the known Studio build and verification
Nx targets after changing a definition, source or expected result; maintain
`docs/todo.md` fixture actions before adding tests.
Its instructions come from [schema-examples.md](schema-examples.md), shared with
the legacy Feature Tour seed. [starter.md](starter.md) documents the editable
starter, which has no persisted configurations and keeps `source` as its default
document. Their READMEs travel in the same archives and offline cache as other
resources; no dedicated documentation route is required.

After generating bundles, run
`node packages/cem-studio/scripts/audit-catalog.mjs --write` to refresh the coverage
document and review it. Run
`yarn nx run @epa-wg/cem-studio:verify:catalog-audit` for the deterministic gate
and negative-reference tests. Native command grammar and data parsing stay in
the shared native engine; the documentation audit reads named control metadata.

Run `yarn nx run @epa-wg/cem-studio:verify:catalog-acceptance` for the complete
catalog gate. It checks actual native/worker configuration receipts, project
navigation and independent clones, original/saved/offline bytes, builds and
previews. The fast resource-presentation preflight runs before the browser census.
Project profiles keep complete assertions and existing timeouts; cross-project
state, draft and preview checks remain in the normal navigation suite.

The evidence distinguishes intentional diagnostic cases, native source limits,
Inspector output versus its diagnostic companion, and binary admission refusals.
Native query owner identities are not portable addresses: compare original
source facts, retain exact per-run exports, and never restore an owner from JSON.
See the [acceptance evidence](../../docs/archive/todo-completed-full-catalog-acceptance-2026-10-25.md)
for the checked scope and remaining native follow-ups.

The [Markdown site](markdown-site/README.md) demonstrates a folder-based build
with four Markdown documents, shared CEM-ML layouts/partials, explicit site data
and static assets. Its validation, body-preview and complete-site configurations
use the same portable project format and Studio workspace as Login and Supplier.

The [JSON and YAML report](json-yaml-report/README.md) combines a fictional API
payload with YAML report settings. Its ordinary configurations inspect imported
data, validate source syntax, format both languages, filter records through native
CEM-QL and build HTML/JSON/YAML artifacts. Malformed sources have explicit repair
steps. Source validation is separate from validating application instances against
an authored schema; no general JSON Schema instance validator is implied.

The [CSV inventory](csv-inventory/README.md) covers spreadsheet-style imports,
explicit header policy, quoted fields, native queries and an HTML/JSON inventory
report. Its configurations preserve native header/row-width warnings and hard
quoting errors, with repairs in ordinary source or configuration resources.
Compact CSV formatting preserves interchange values; tabular formatting is a
review presentation. Inventory number conversion is an explicit template rule.

The [Supplier pipeline](supplier-pipeline/README.md) also exposes native XML
validation, expanded-name XPath selection and a mismatched-tag repair without
copying its catalog or reconciliation graph. The [XHTML document](xhtml-document/README.md)
adds XML-serialized document inspection, namespace-aware outlines, source
validation/repair and template-driven HTML output. XHTML uses its own lifecycle
identity and preserves foreign namespaces; validation covers the documented
XML/XHTML source contract rather than complete HTML content models.
