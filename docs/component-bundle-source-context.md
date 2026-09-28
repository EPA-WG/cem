# Release bundle source context

Status: recommended approach approved and implemented, 2026-09-28.

The accepted [component development pattern](component-development-pattern.md)
requires a generated combined XHTML document and verification of relative
dependencies before release. The native transform graph supports collecting
canonical source artifacts; the unresolved part is the resource context of a
copied template when loaded through `components.xhtml#tag`.

## Original characterization

The original browser fixture
`packages/cem-elements/src/lib/bundle-source-context.stories.ts` loads identical
CEM-ML template text from an individual XHTML declaration and a combined XHTML
document. Its `cem-module-url` requests `./icon.svg`.

| Loading source | Resolved dependency |
| --- | --- |
| `/src/components/card/card.xhtml#card` | `/src/components/card/icon.svg` |
| `/dist/components.xhtml#card` | `/dist/icon.svg` |

The combined fixture retains
`xml:base="../src/components/card/card.xhtml"` on the enclosing declaration.
Before the fix, the runtime used the fetched bundle URL as its resource base.
The original test characterized that behavior through the URL slice and rendered
link. The updated test now requires source-base parity.

The source loader selects the fragment template and supplies the fetched
document's resource base to compilation. Metadata on the enclosing declaration
is not inherited by that fragment-loading path. For example, callers currently
supply `capability="choice-select"` explicitly; merely copying the declaration
wrapper does not change the loader contract.

## Decision (approved)

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

The user approved option 1. The runtime now applies the selected template's
`xml:base` ancestor chain through the shared native URL boundary. The updated
browser fixture expects the original dependency location in both loading forms,
and covers invalid bases, redirects, separate fragment bases and CSS imports.
The historical behavior table above records the issue before this change.

## Delivery

The native CEM AST graph in `packages/cem-components/build/` generates
`dist/components.xhtml` from canonical declarations. The package exports it and
ships the source definitions, build inputs and bundle playground. Generated
containers retain relative `xml:base` metadata. Individual and bundled template
text and declaration metadata match.

Verification passes: three native URL tests, the native CLI bundle fixture,
77 browser stories, and source/isolated-package playground journeys. Browser
checks include relative CSS imports, redirects, two fragment bases in one cached
document, invalid metadata, unique IDs, stylesheet ownership and duplicate
registration rejection. Existing fragment capability declarations remain explicit.
