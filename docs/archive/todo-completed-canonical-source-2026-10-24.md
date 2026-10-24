# Completed canonical source viewer repair — 2026-10-24

- [x] Repair extensionless Canonical source URLs in component property pages and galleries after the folder-level import-map migration; use explicit XHTML paths with the shared resolver.
- [x] Add import-verifier checks for explicit canonical document paths and loaded viewer content in source and packaged pages; rerun the nine-gallery compact Studio gate.

Updated 76 source-viewer specifiers to name the document, for example
`@epa-wg/cem-components/components/cem-text/cem-text.xhtml`. Existing folder
import maps and `cem-module-url` resolution remain in place. Source viewers
load the complete document; component declarations retain their template
fragments. No UI behavior or application JavaScript was introduced.

The demo-import verifier now rejects component document paths without the
explicit XHTML filename, checks mapped files exist, waits for the matching
source viewer to become ready, and compares its source with the HTTP response.
It also checks a corresponding document link. Matching uses public source URLs
rather than assuming every viewer shares the same legend wording.

Validation:

- The new endpoint assertion failed against the original `cem-text.html` URL.
- All 170 canonical source endpoints in authored and built pages resolve to
  declaration documents after the playground build.
- Four source/built `cem-text` property/gallery pages passed the browser import,
  live page-theme, canonical viewer and source-link checks.
- Four source/built menu-item and theme-switch property pages passed those same
  checks, including their distinct source-viewer legends.
- The complete compact Studio verifier passed all nine affected galleries,
  accordion/data-view property behavior and the canonical bundle from both
  source and isolated installed package archives.
- Scoped ESLint and whitespace checks passed. Browser contexts were isolated;
  saved projects and drafts were not modified.

A larger combined demo-import run also exposed existing tree-gallery null
`querySelector` page errors and an icon-button selected-page-theme wait failure.
These independent failures remain in the active TODO; no full-inventory
`verify-demo-imports` pass is claimed. The earlier custom-legend viewer waits
were corrected in the verifier and the menu-item/theme-switch rerun passed.

The independent gallery errors and stalled theme update were subsequently
[diagnosed and repaired](todo-completed-gallery-imports-2026-10-24.md), with a
passing targeted 14-page source/built import cohort.
