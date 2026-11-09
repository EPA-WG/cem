# Live slice and local form hydration

Base commit `3cd6afbcc57e6769a93b2e7fc80a7ca14b9d9e90`.

- [x] Export typed declaration-owned scalar defaults and verify them against a
  fresh native compile on adoption. Refuse unknown and changed slice values,
  private controls and form submission routes.
- [x] Verify CLI output in Chromium for retained input, checkbox and select
  nodes and range markers. Test focus and dirty edits before adoption,
  event-driven patches, stale capture, forged state and external form owners.
  Check that private or external values stay out of the live snapshot.
- [x] Verify that a named control shadowing a form DOM method refuses adoption
  before any range or island metadata changes.

The browser proves the native island and compares a fresh processing baseline
with the retained range before publishing node identities or binding events.
Current control values come from owned native form elements. Reading their
state emits no `formdata` event, and the adopted local form suppresses
submission. Both main-thread and worker processing retain the same controls
through later events.

The stateful profile covers compatible standalone inline participants,
declaration-owned string, boolean and null defaults, and ordinary native
controls. Styled or nested stateful ranges, browser-authored checkpoint state,
external actions, typed event expressions and custom validation require
separate admission. Re-export earlier CLI output to include canonical slice
defaults.

The native browser-resume suite passed 11 checks, the stripped SSR suite
passed 127 checks and the component-runtime suite passed 17. The optimized
CLI-to-Chromium suite passed 47 of 47 cases, including all eight form cases,
with zero external requests. The 27 processing-boundary tests, 10 Edge SSR
stories and WASM consumer checks passed.
The source-admission suite passed 314 native checks and 30 browser cases.
Clippy and browser lint passed.

The optimized core and packaged WASM match at 24,234,458 bytes with 33 imports
and 178 exports. SHA-256 is
`b7189aeb483eea22dee5969a46bc8fc8ef07218f968ac092256e656d541a72ad`.
