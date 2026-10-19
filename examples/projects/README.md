# Studio example projects

Every directory containing `example.json` defines an ordinary portable Studio
project. The generator discovers directories deterministically; adding an example
does not require a Studio route or renderer.

- `id`, `name` and `version` identify the catalog template. Optional `collections`
  mark presentation or subject membership, such as `amsterdam-2026`.
- `sources` declare stable resource IDs, repository source paths, project paths,
  content types, schemas and roles. Include README resources for instructions.
- `configurations` declare names, kinds, source/dependency resource IDs and native
  authored command arguments. `$source` and `$<dependency-id>` are replaced with
  project-relative resource addresses during generation. Expected results and
  repair metadata support native fixtures; they do not drive a tutorial UI.

Generated bundles use the native version-1 project contract (`entries` and
`runConfigResourceId` are compatibility wire fields). Studio verifies archive
hashes and imports editable copies through the same repository as user-created
and imported projects. Copies have independent project identities and preserve
resource IDs and reference paths. Archive updates never overwrite saved copies.

The schema-package example project is generated from package manifests using
the same archive/catalog contract. Run the known Studio build and verification
Nx targets after changing a definition, source or expected result; maintain
`docs/todo.md` fixture actions before adding tests.

The [Markdown site](markdown-site/README.md) demonstrates a folder-based build
with four Markdown documents, shared CEM-ML layouts/partials, explicit site data
and static assets. Its validation, body-preview and complete-site configurations
use the same portable project format and Studio workspace as Login and Supplier.
