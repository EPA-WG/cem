# Browser verification of SSR stylesheet imports

Base commit `bd46c883ce4b7e9009461828d9b0c72a58b5ea6c`.

- [x] Publish stylesheet receipts from the retained declaration and instance
  closures. Preserve the public URL, MIME profile, byte length and digest.
- [x] Read bounded same-origin responses and replay the imports through the
  native CSS loader. Resolve each import against its owning URL and compare
  the resulting scoped style text and installed CSS rules before adoption.
- [x] Verify declaration, nested and instance imports in Chromium. Matching
  resources retain the existing nodes in both runtime modes. Refusal preserves
  the style nodes, rendered range and hydration marker.

Native SSR and stripped-profile suites passed. Chromium covered 73 cases,
including 11 import cases, with zero off-origin requests.
