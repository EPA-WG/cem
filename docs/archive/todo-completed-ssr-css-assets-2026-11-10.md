# CSS asset identities for SSR browser adoption

Base commit `c28aa83525ca68fcd47b543092722eaf3268002f`.

- [x] Add `verify-css-assets=true` to the packaged SSR template. Read supported
  image and font URLs through explicit CLI authority under the original
  operation. Share exact asset reads across stylesheet occurrences.
- [x] Bind URL, media type, size and digest to each stylesheet receipt. Replay
  the native CSS resource plan and verify all fresh responses before adoption.
  The browser uses its existing cross-origin grants for these checks.
- [x] Verify budget accounting and atomic CLI refusal. Verify in-place browser
  adoption and unchanged server DOM when assets are missing, changed, denied
  or served with the wrong media type.

Native CLI, export, replay and stripped-profile suites passed. Chromium
covered 102 cases, including seven CSS asset cases. The managed stylesheet
profile continues to use public asset URLs and leaves `@font-face` to page CSS.
