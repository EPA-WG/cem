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

## CEM Elements Runtime

- [ ] **Dynamic internal `<textarea>` merge and hydration handling.** Deferred out of the immediate release queue.
      Implement and cross-browser validate the hidden child-node merge model plus explicit `.value` projection, including
      SSR loader conversion from a loader-friendly `<xsl:element name="textarea">`-style or equivalent CEM-ML placeholder
      form.
