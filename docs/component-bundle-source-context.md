# Release bundle source context

Status: decision needed, 2026-09-28.

The accepted [component development pattern](component-development-pattern.md)
requires a generated combined XHTML document and verification of relative
dependencies before release. The native transform graph supports collecting
canonical source artifacts; the unresolved part is the resource context of a
copied template when loaded through `components.xhtml#tag`.

## Verified behavior

The browser fixture
`packages/cem-elements/src/lib/bundle-source-context.stories.ts` loads identical
CEM-ML template text from an individual XHTML declaration and a combined XHTML
document. Its `cem-module-url` requests `./icon.svg`.

| Loading source | Resolved dependency |
| --- | --- |
| `/src/components/card/card.xhtml#card` | `/src/components/card/icon.svg` |
| `/dist/components.xhtml#card` | `/dist/icon.svg` |

The combined fixture retains
`xml:base="../src/components/card/card.xhtml"` on the enclosing declaration.
The runtime still uses the fetched bundle URL as its resource base. The test
asserts both the URL slice and rendered link and passes under the current
contract. It is a characterization of existing behavior, not bundle acceptance.

The source loader selects the fragment template and supplies the fetched
document's resource base to compilation. Metadata on the enclosing declaration
is not inherited by that fragment-loading path. For example, callers currently
supply `capability="choice-select"` explicitly; merely copying the declaration
wrapper does not change the loader contract.

## Decision

1. **Preserve the original source context (recommended).** Add a shared runtime
   contract for retained source-base metadata, emitted by the native AST bundle
   transform. Both loading forms then resolve relative resources beside the
   original definition. Define the metadata carrier and relative resolution,
   validate it at the native loading boundary, and include it in declaration,
   resource, stylesheet and registration identities. Keep caller capability
   declarations explicit under the existing fragment contract.
2. **Relocate dependencies at build time.** Keep the fetched bundle as the runtime
   base. The native AST transform must rewrite supported relative dependencies
   or deploy their assets beside the bundle. Define handling of dynamic resource
   expressions and fail unsupported relocation cases rather than silently
   changing their meaning.

Preserving source context fits the requirement that individual and bundled
loading behave alike without rewriting dynamic CEM-ML expressions. This is a
shared loading-contract change, so execution stops here under the user's
instruction to stop for open decisions. No release bundle or runtime behavior
change has been made.

## Next after the decision

- Add native tests for the chosen source-context or relocation contract, then
  implement the shared mechanism.
- Generate the XHTML bundle through the native CEM AST graph, retaining unique
  template IDs, canonical declarations and source/playground distribution.
- Verify individual and fragment loading, relative dependencies, retained style
  ownership and duplicate-registration diagnostics from source and packages.

Verification: `yarn nx run cem-elements:test
packages/cem-elements/src/lib/bundle-source-context.stories.ts` passes (one story).
