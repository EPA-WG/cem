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
- [Recovery acceptance](../../../../../docs/cem-components-shopping-cart-profile.md#compiled-application-recovery-acceptance)
  uses `compiled-declaration-recovery.stories.ts` to replace real worker/fallback
  generations twice, preserve edits/forms/styles, refuse entered credentials and
  reject newer input or changed build pins. Clear credentials explicitly before
  recovery; authored/reset credential defaults must also be empty.

Generated files remain under ignored `dist/artifacts/cem_ql/compiled-form`.
Native CSS parsing remains required; source compilers are still present in the
full WASM. Build assets contain no credentials. Recovery tests use temporary
values in isolated browser realms; API transport and app-local behaviors are not
added.
