# `@epa-wg/custom-element` Package Baseline

This captures the Phase 3.6 pre-import baseline for `@epa-wg/custom-element`.
It should be read with
[`custom-element-migration-scope.md`](custom-element-migration-scope.md).

## Published Package Baseline

The workspace currently consumes the installed npm package:

- package: `@epa-wg/custom-element`
- version: `0.0.39`
- location inspected: `node_modules/@epa-wg/custom-element/`
- package type: ESM (`"type": "module"`)
- license: `Apache-2.0`
- npm identity fields to preserve: `name`, `repository`, `bugs`, `homepage`,
  `funding`, `author`, `types`, `web-types`, and public browser entrypoint names.

`package.json` fields that are part of the public shape:

- `"browser": "custom-element.js"`
- `"module": "custom-element.js"`
- `"types": "./custom-element.d.ts"`
- `"web-types": ["./ide/web-types-dce.json", "./ide/web-types-xsl.json"]`
- `"exports"`:
  - `"."` -> `./index.js`
  - `"./package.json"` -> `./package.json`
  - `"./CustomElement"` -> `./custom-element.js`
- `"files": ["*"]`, so the published artifact currently includes demos, docs,
  IDE metadata, scripts, and loose browser modules.

The root workspace dependency currently pins `@epa-wg/custom-element` to `0.0.39`.

## History Source Baseline

The history source is the local Git checkout:

- path: `/home/suns/aWork/custom-element/`
- remote: `git@github.com:EPA-WG/custom-element.git`
- branch/commit inspected: `main` at `0282a74`
- package version in checkout: `0.0.37`
- commit count: 273
- release tags present: through `0.0.37`

Do not treat the local checkout as the full published `0.0.39` baseline. Use it for
history import, then reconcile with the installed package contents before replacing
the workspace dependency.

## Entrypoints And Side Effects

Public browser files shipped by `0.0.39`:

| File | Public behavior |
| --- | --- |
| `custom-element.js` | Exports `CustomElement` and helpers; registers `custom-element`; dynamically registers produced tags. |
| `index.js` | Default-exports `CustomElement`; re-exports `custom-element.js`, `http-request.js`, `local-storage.js`, and `location-element.js`. |
| `custom-element.d.ts` | Type declarations for `CustomElement` and core helpers. |
| `http-request.js` | Exports `HttpRequestElement`; registers `http-request` on import. |
| `local-storage.js` | Exports local-storage helper functions and `LocalStorageElement`; registers `local-storage` on import. |
| `location-element.js` | Exports `LocationElement`; registers `location-element` on import. |
| `module-url.js` | Exports `ModuleUrl`; registers `module-url` on import, but is not re-exported from `index.js`. |

Importing these browser modules has global side effects through
`window.customElements.define(...)`. The migrated package must preserve this behavior
or clearly document any next-major break.

Observed custom element registrations:

- `custom-element`
- `http-request`
- `local-storage`
- `location-element`
- `module-url`
- produced custom element tags declared by `<custom-element tag="...">`

## `0.0.37` Checkout vs Installed `0.0.39`

The installed package differs from the local history checkout in ways that must be
preserved during import:

- `package.json` version is `0.0.39` instead of `0.0.37`.
- `custom-element.js` is larger in `0.0.39` and includes two browser fixes:
  - XSLT sanitation transforms into `document.implementation.createHTMLDocument('')`
    instead of the live document, avoiding eager resource loading from intermediate
    fragments.
  - `src` loading parses `responseText` with `DOMParser` and MIME detection instead
    of relying on `XMLHttpRequest.responseXML`.
- Companion module byte sizes match between the local checkout and installed package
  for `http-request.js`, `local-storage.js`, `location-element.js`, and
  `module-url.js`.
- The installed package includes extra demo/editor scratch files not present in the
  local checkout, including `.claude/`, `.idea/`, `.vs/`, and
  `demo/{a.html,b.html,s.xml,s.xslt,s1.xml,ss.html}`. These should be reviewed before
  being preserved in the workspace package.
- The local checkout contains `package-lock.json`; the installed package does not.

## Workspace References To Rewire

Current hard references to the external dependency:

- root `package.json`
  - dependency: `"@epa-wg/custom-element": "0.0.39"`
  - web-types:
    - `./node_modules/@epa-wg/custom-element/ide/web-types-dce.json`
    - `./node_modules/@epa-wg/custom-element/ide/web-types-xsl.json`
- `packages/cem-theme/project.json`
  - `build:html` inputs include:
    - `{workspaceRoot}/node_modules/@epa-wg/custom-element/custom-element.js`
    - `{workspaceRoot}/node_modules/@epa-wg/custom-element/http-request.js`
- `packages/cem-theme/src/lib/theme-editor/theme-editor.html`
  - imports both `custom-element.js` and `http-request.js` from `node_modules`.
- `packages/cem-theme/src/lib/css-generators/*.html`
  - import `custom-element.js` and `http-request.js` from `node_modules`.
  - also import local generated `cem-http-request.js` from generator output.
- `packages/cem-theme/docs/html-compile.md`
  - documents the current vendor copy layout and `node_modules` import paths.
- Several design docs and inventories reference `@epa-wg/custom-element` as the
  migration target or parity source; these are conceptual references, not package
  wiring.

The consumer-rewire phase must keep browser-served paths stable where practical, or
update the HTML compiler/vendor-copy docs and fixtures together.

## Baseline Acceptance Notes

Before code import begins:

- compare `0.0.39` installed files against the local history checkout and keep the
  browser fixes listed above;
- decide whether editor scratch files from the installed package belong in the new
  workspace package;
- preserve import side effects for shipped browser modules;
- preserve or deliberately replace the current `web-types` package metadata;
- keep `module-url.js` visible as a shipped browser file even though it is not part of
  `index.js` exports today.
