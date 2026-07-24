# Wishlist

This file tracks future capability ideas that are not part of the immediate release queue. Active execution tasks live
in [`todo.md`](todo.md).

## CEM-ML Runtime

- [ ] **Engine XSLT 3.0/4.0 execution behind G-NVDL-FULL (AC-P-6.9).** The architecture keeps XSLT as a
      capability-gated peer language behind explicit dispatch, not the primary authoring/rendering model or a
      browser-native dependency. Building the XSLT 3/4 engine remains out of scope for the current release.
- [ ] **Web-service schema validation.** Extend CEM-ML's generic schema engine to support service-description schemas
      such as OpenAPI/Swagger and GraphQL. Compose service validation with the existing URL-level validation—including
      URL parameter encoding—so web services can participate in a unified web-application validation chain.
- [ ] **`*.cemt.md` authored transform-doc format with HTML output.** Add a Markdown-adjacent CEMT documentation format
      that can embed CEMT modules/examples and transform them into HTML documentation or previews through the CEM-ML
      pipeline.
- [ ] **Advanced import fallback and substitution policies.** Extend the resolver policy model beyond explicit
      one-step substitution to cover ordered fallback lists, offline mirrors, semver/range module replacement,
      dev/prod import maps, stale-cache use when remote imports are unavailable, and trust-tier downgrade/upgrade
      diagnostics. These policies must preserve requested and resolved identity in reports and artifact/cache stamps.

## CEM-QL Language

- [ ] **User-defined overloads.** Allow user-authored declarations such as functions with the same exported name only
      after CEM-QL has a typed signature model that can distinguish arity and parameter/return types deterministically.
      The design must define overload-set encoding in package artifacts, import/export collision rules, ambiguous-call
      diagnostics, and formatter/HTML/example coverage before relaxing duplicate-declaration errors.

## CEM Elements Runtime

- [ ] **Dynamic internal `<textarea>` merge and hydration handling.** Deferred out of the immediate release queue.
      Implement and cross-browser validate the hidden child-node merge model plus explicit `.value` projection, including
      SSR loader conversion from a loader-friendly `<xsl:element name="textarea">`-style or equivalent CEM-ML placeholder
      form.
- [ ] assure `$document` in scope
- [ ] named scope
