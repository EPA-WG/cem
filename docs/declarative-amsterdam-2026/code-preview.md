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

## Current native HTML output limitation

This binary's direct CEM output remains plain text with `--cemt-color-profile html`
and `--output-color-type html` or `html-css-vars`, despite its report recording
the HTML colorer stage. The preview therefore uses native CLI formatting followed
by the existing browser CEM HTML colorer, with `cem-demo-element` in source-only
mode. It does not use an ANSI-to-HTML converter or manually assign token roles.

The CLI also reports three `cem.ast.unresolved_reference` warnings for
`login-title`, `email`, and `password`. Its exit status is zero; the browser
source highlighter reports no diagnostics. The generator saves the CLI report
and diagnostics alongside the preview. This verifies source presentation,
not the full tutorial's validation or rendering behavior.
