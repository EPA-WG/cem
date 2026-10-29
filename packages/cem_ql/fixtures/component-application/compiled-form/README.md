# Compiled shared form

This build fixture loads an ordinary form, card, text and alert through generated
declarations and pinned native template binaries. It demonstrates source-free
component loading and editable form controls; it is not a storefront or a Studio
example project.

- [cem-compiled-form-app.xhtml](cem-compiled-form-app/cem-compiled-form-app.xhtml)
  composes five canonical shared components without application JavaScript.
- The native `compile_declaration_closure` example discovers their static
  dependencies and generates metadata, binaries, a dependency inventory and
  `declarations.html`. Include that markup in a document with the normal shared
  runtime installed. Relative asset URLs use that document's resource base.
- Run `node packages/cem_ql/tests/build-declaration-fixtures.mjs` from the repository
  root, then the `compiled-declaration-loader.stories.ts` browser tests.

Generated files remain under ignored `dist/artifacts/cem_ql/compiled-form`.
Native CSS parsing remains required; source compilers are still present in the
full WASM. No credentials, API transport, or app-local behaviors are added.
