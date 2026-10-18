# Code preview for slides

The first preview uses [the existing login document](../../examples/cem-ml/login.cem).
The native CLI applies the **tabular** formatter. The same CEM HTML source
colorer and theme used by the syntax-coloring demo then produce the browser
preview. Studio is not required.

Generated files live in `dist/declarative-amsterdam-2026/code-preview/`:

- [Standalone HTML](../../dist/declarative-amsterdam-2026/code-preview/login-preview.html)
- [1920 × 1080 preview](../../dist/declarative-amsterdam-2026/code-preview/login-preview.png)
- [Code-only image](../../dist/declarative-amsterdam-2026/code-preview/login-code.png)
- [CLI tabular source](../../dist/declarative-amsterdam-2026/code-preview/login.tabular.cem)
- [Browser verification](../../dist/declarative-amsterdam-2026/code-preview/preview-evidence.json)

These are generated, ignored outputs. Recreate them from the repository root:

```sh
node docs/declarative-amsterdam-2026/generate-code-preview.mjs
```

The generator uses the existing native binary at
`dist/target/cem_ml_cli/debug/cem-ml`, built theme and demo packages, and installed
Playwright Chromium. It copies the rendered source and its original styles into
the standalone HTML, which needs no script, WASM download, or server. A local
server can also open it for browser review:

```sh
python3 -m http.server 8766 --bind 127.0.0.1 \
  --directory dist/declarative-amsterdam-2026/code-preview
```

Open `http://127.0.0.1:8766/login-preview.html`.

Verification on 2026-10-11: the browser text matches the CLI output exactly,
the inserted markup matches the CEM WASM highlighter after HTML normalization,
61 tokens have semantic markup, and four sampled token roles resolve to
distinct colors. There is no horizontal or page clipping at 1920 × 1080 and
no live form insertion. Sasha approved the formatting and colors on
2026-10-11 for use in the Google Slides presentation.

The CLI reorders attributes, normalizes quoting, and compacts closing braces
through its formatter. The preview preserves that output without manual
realignment. The original source remains unchanged.

## Native HTML source output

The original preview used native CLI formatting followed by the shared browser
CEM HTML colorer in source-only mode because native HTML presentation returned
plain text. The native typed CEM-tree writer now emits escaped HTML source colors
for `html`, `html-css-vars`, `html-classes` and `html-inline-style`, preserving
pretty/tabular layout and source-frame chains. Login 2.7.0 exposes this as
**Export Login source colors (HTML)**. Its downloaded HTML carries the shared
palette; Studio's sandboxed preview continues to block styles and scripts.

The approved slide artifact and its generator remain unchanged. Replacing that
artifact is a separate presentation choice; the native export is now available
without a browser highlighter or ANSI-to-HTML conversion.

The CLI also reports three `cem.ast.unresolved_reference` warnings for
`login-title`, `email`, and `password`. Its exit status is zero; the browser
source highlighter reports no diagnostics. The generator saves the CLI report
and diagnostics alongside the preview. This verifies source presentation,
not the full tutorial's validation or rendering behavior.
