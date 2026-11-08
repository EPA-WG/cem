# Native CLI lowering and SSR exports — 2026-11-08

Base: `f59d11d92a4d3ed0ed283c688f6bf2c6db812fc0`.

The packaged immutable `ssr-page.cemt` exposes `ssr:render-page` through the
existing CEM-native transform adapter. Native CLI direct/config lowering uses
the same command engine and transactional publisher. Source paths become
absolute escaped file URIs before HTML import; page/base identities remain
independent of physical output placement.

The builtin consumes explicit page URL, consumer versions and depth, instance,
render, resource, byte, wall and CPU limits. It coordinates nested inline
declarations, inline scoped CSS and native payload source leases. Scoped URL
resolution grants no transport; every pending external ticket is refused.
No browser/Node SSR implementation, document JSON handoff, source reparsing or
generic public “ready” setter was introduced.

The private native output retains `ReadyInlinePage`, its context and the
original operation/CPU ledger. The shared engine publication hook rechecks
operation freshness and rejects colored output or document rewrites. The HTML
and explicitly named JSON-report encoders validate readiness before/after
encoding and share cumulative export-byte accounting. A late graph-export
failure leaves every destination unpublished. A report is diagnostic metadata;
the generic SSR envelope and report both retain `hydrationReady: false`.

The shared retained DOM writer projection preserves namespace bindings, ordinary
HTML/SVG names, doctype, raw text, semantic values and native source references.
HTML uses the existing DOM-to-HTML formatter/writer contract with color disabled.
The projection is not an SSR-specific serializer. Original parser AST traversal
remains inside the shared CEM-ML projection layer.

Completed fixtures:

- [x] Immutable builtin selection, explicit controls, nested readiness, invalid
      versions/routes/limits and unsupported export identities.
- [x] Fresh/repeated encodings, cumulative byte exhaustion, late cancellation,
      foreign controls, generic-envelope refusal and rewrite/color rejection.
- [x] Direct/config CLI outputs and transactional refusal across multiple files.
- [x] HTML namespace/raw-text/doctype/source-owner projection and host URI escaping.
- [x] Inline CSS and inert native CEM payload source leases without transport.
- [x] Chromium parsing of CLI-produced HTML: standards mode, correct nested
      content, two inert `cem-island:context-root` islands and zero requests.

Verification counts and final package fingerprints are recorded in the paired
[machine receipt](ssr-cli-exports-2026-11-08.json).

Limits: native CPU clocks were exercised on Linux. Acquisition/parser CPU before
the bridge is not included in its receipt; wall time retains the original
operation deadline. External-resource CLI admission, broader source/fan-out
lowering and browser hydration/adoption remain future work. No presentation
deployment or frozen-material update is part of this change.
