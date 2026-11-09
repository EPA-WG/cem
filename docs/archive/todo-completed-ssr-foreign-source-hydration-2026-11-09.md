# Browser adoption of foreign markup and inert CEM source

Base commit `75feef62968959c4523495e0bacbdb7b108aeaa7`.

- [x] Compare captured SVG and MathML against the native projected range using
  expanded element and attribute names. Reuse the passive foreign-content
  validator for the range and island payload. Reject altered names, attribute
  values and resource-bearing foreign content.
- [x] Admit a canonical CEM source envelope through the shared bounded source
  importer. Require the exact fresh DOM capture owner and operation. Verify
  runtime source maps, unknown authored coordinates, lowered bounds, agreement
  with a nested participant's parent and refusal of malformed or transported
  source.
- [x] Verify CLI-emitted HTML in Chromium. Compatible adoption retains the SVG,
  MathML and source envelope nodes. XML and XLink attributes keep their expanded
  names on subsequent renders. Changed or active foreign content, malformed or
  incompatible source and edits during admission freeze the instance. Valid
  edits after adoption invalidate the instance and source expressions stay inert.

The WASM bridge consumes the live DOM capture synchronously and keeps its
reservation through admission. Existing declaration stylesheet values can still
use typed transport. The browser watches the separate inert source fragment
during proof and after connection.

Native verification passed 10 browser-resume checks, 126 stripped SSR checks
and 17 component-runtime checks. Both packages passed 39 of 39 CLI hydration
cases, including all eight foreign and source cases, with zero external
requests. The optimized package also passed 27 processing-boundary tests,
10 Edge SSR stories, WASM consumer regressions, Clippy and browser lint.
The source-admission suite passed 314 native checks and 30 browser cases.

The optimized core and packaged WASM match at 24,293,376 bytes with 33 imports
and 178 exports. SHA-256 is
`d1e3b58450146fefa8439e805b18c82a31fb0ac469efd74b577b95e98f26d686`.
