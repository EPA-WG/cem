# CEM-ML and CEM Markdown SSR page sources

Base commit `610267252be374d1f3658cb70bc7fd36b6f8dbf4`.

- [x] Admit an explicitly identified CEM-ML page and an opted-in
  `.cem.md` page in direct and config CLI transforms. Ordinary Markdown
  and CEM source handling keep their existing entry points.
- [x] Use the native CEM and GFM parsers and the shared HTML event builder.
  Keep the authored input hash and byte count in the readiness proof.
  The direct and graph HTML sidecars retain authored output spans, including
  the Markdown file and its embedded CEM-ML fence.
- [x] Charge parsing and typed page conversion to the original operation.
  Refuse unsafe markup and enforce structural limits before the recursive
  projection. Keep earlier HTML and sidecars when page admission fails.

The CEM-ML page profile requires an HTML root with `head` and `body`.
The CEM Markdown profile generates the document envelope and lowers
explicitly labelled `cem-ml` fences. Unlabelled code stays literal.
Standalone stylesheets in projected pages remain outside this admission
profile. Component-owned styles use the existing SSR stylesheet path.
The ordinary CEM namespace policy still applies. SVG uses retained namespace
bindings. The built-in page profile supplies no CEM MathML schema.

The native checks passed 11 CLI SSR cases, 12 adapter export cases, 39 core
SSR cases and two ordinary login graph regressions. The CLI-to-Chromium
suite passed 50 cases, including both new source profiles with original
node adoption and zero external requests.

The existing Markdown-site suite passed three of four tests. Its version
assertion expects `1.1.0` and the authored example already declares
`1.1.1` at the base commit.
