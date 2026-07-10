# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in [`wishlist.md`](wishlist.md). Completed implementation
history belongs in git history and the feature-specific docs linked below.

## Immediate Tasks

- [ ] Complete the schema-package folder frame for
      `packages/cem_ml/schema-packages`: every `{package-id}/vN/` folder must be
      discoverable from `package.cem` with a `.cem` schema source, example
      references, CEMT formatter artifacts, and CEMT colorizer artifacts.
  - [ ] Extend the schema-package manifest and validators so package examples
        are declared with source path, content type, schema URL, expected
        pass/fail result, and expected diagnostics.
  - [ ] Require example loading to resolve the declared content type plus schema
        URL and validate the source bytes against that schema; filename
        extension inference is only a fallback hint.
  - [ ] Require baseline formatter profiles for each package:
        `compact` as default, `pretty`, and `tabular`; implement them as CEMT
        transforms that preserve source-map ranges.
  - [ ] Require baseline colorizer profiles for each package: `terminal`,
        `html`, and `md`; implement them as CEMT transforms over the formatted
        CEM tree with source-map range preservation.
  - [ ] Add package-folder validation that checks `package.cem`, `schema/`,
        `examples/`, `formatters/`, and `colorizers/` completeness for every
        built-in package.

## Schema Package Frame Scope

Each supported package below is complete only when the generic folder frame in
Immediate Tasks is satisfied for that package: `package.cem`, `.cem` schema,
explicit example content-type/schema references, `compact`/`pretty`/`tabular`
CEMT formatters, `terminal`/`html`/`md` CEMT colorizers, and package-folder
validation coverage.

- [ ] `cem-ml/v1` (`application/cem`; aliases: `text/cem-ml`, `text/cem`,
      `application/cem+xml`).
- [ ] `schema/v1` (`application/vnd.cem.schema+cem`).
- [ ] `schema-package/v1` (`application/vnd.cem.schema-package+cem`).
- [ ] `cem-native-template/v1` (`application/vnd.cem.template+cem`; CEM source
      aliases).
- [ ] `cem-transform/v1` (`application/vnd.cem.transform+cem`, `.cemt`).
- [ ] `cem-ql/v1` (`application/vnd.cem.query+cem-ql`, `text/cem-ql`, query
      artifact aliases).
- [ ] `json/v1` (`application/json`, `text/json`).
- [ ] `json-schema/v1` (`application/schema+json`).
- [ ] `cem-dom-projection/v1` (`application/vnd.cem.dom+cem-bin`,
      `application/vnd.cem.dom+json`).
- [ ] `cem-ast-projection/v1` (`application/vnd.cem.ast+cem-bin`,
      `application/vnd.cem.ast+json`).
- [ ] `cem-events-projection/v1` (`application/vnd.cem.events+cem-bin`,
      `application/vnd.cem.events+json`).
- [ ] `yaml/v1` (`application/yaml`, YAML aliases).
- [ ] `csv/v1` (`text/csv`).
- [ ] `markdown/v1` (`text/markdown`).
- [ ] `xml/v1` (`application/xml`, XML aliases).
- [ ] `relax-ng/v1` (`application/relax-ng+xml`,
      `application/relax-ng-compact-syntax`).
- [ ] `xhtml/v1` (`application/xhtml+xml`).
- [ ] `svg/v1` (`image/svg+xml`).
- [ ] `mathml/v1` (`application/mathml+xml`, MathML aliases).
- [ ] `xslt/v1` (`application/xslt+xml`, XSLT aliases).
- [ ] `html/v1` (`text/html`).
- [ ] `css/v1` (`text/css`).

- [ ] Expand example coverage from representative constraint-kind coverage to
      finer diagnostic coverage, starting with schema-package source
      read/invalid cases and artifact source/parse/function-missing cases.

# [] believes schema + registry
stop for sync up with author
## Current Verification Commands

- `yarn nx run @epa-wg/cem-theme:verify:phase13`
- `yarn nx run cem-elements:verify`
- `yarn nx run @epa-wg/cem-components:verify`
- `yarn nx run cem-elements:verify-edge-ssr`
- `yarn nx run @epa-wg/custom-element:verify`

## Externally Gated

These are intentionally not active in the current workspace because the required native toolchains are unavailable.
Keep the existing offline platform artifact validation as the release gate until supported native CI exists.

- Swift/Xcode compile gate for `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`.
- Kotlin/Compose Gradle compile gate for `packages/cem-theme/dist/lib/token-platforms/android/`.
