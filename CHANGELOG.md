## 0.1.0 (2026-06-12)

First release shipping the CEM-ML / CEM-QL engine and the `<cem-element>` substrate.
(Curated from `docs/release-readiness-0.1.0.md`; the conventional-commits auto-changelog does not
represent this release — see §8.)

### 🚀 Features

- **CEM-ML engine (`cem_ml`)** — layered tokenizer → event normalizer → schema machine → AST: `@doc`
  version negotiation, namespace rebinding, schema scoping, Layer-5 handoff, unknown-namespace
  disposition (AC-P-6.7), and XSLT region dispatch (AC-P-6.8).
- **CEM-QL engine (`cem_ql`)** — compile-once / render-many WASM boundary, `/datadom` selection,
  `cem:for-each` / `cem:if` / `cem:choose`, and an XPath-parity stdlib (`str:*`, `seq:*`, `num:*`).
- **`<cem-element>` substrate (`@epa-wg/cem-elements`)** — runtime slices A–E, C2 WASM lowering,
  serializable projection boundary, edge render-state, and SSR hydration with BR-VC-9 disposition.
- **`@epa-wg/cem-components`** — base primitives authored entirely against `<cem-element>`.
- **Legacy HTML+XSLT backward-compat** — legacy `<custom-element>` HTML+XSLT templates are transpiled
  to canonical CEM-ML by the single CEM-owned engine (`cem_ml::legacy_custom_element`, shared by
  browser / CLI / SSR / tests) and rendered on the cem_ql engine — no browser XSLT processor — so
  legacy demos render identically to their migrated CEM-ML twins.
- **`cem-theme`** — all 10 CSS generators converted off the browser XSLT 1.0 runtime to
  `type="cem-ml; version=0.0"` (Option B); DTCG / Figma / native token-platform export pipeline.
- **Governance** — fitness functions FF-1..FF-8 all CI-blocking and active, the FF-gate map, and
  governed-contract SemVer axes.

### 💥 Breaking changes

- The browser-native XSLT 1.0 `XSLTProcessor` engine is **retired** from `@epa-wg/custom-element`;
  legacy HTML+XSLT runs via DOM→CEM-ML conversion on the substrate (Tier 1/2; standalone XSLT
  stylesheets are a deferred Tier-3 handoff).
- `cem-theme` CSS generators no longer use the browser XSLT runtime — the generated
  `dist/lib/css/*.css` is unchanged, so CSS consumers are unaffected; only forked/embedded generator
  HTML must migrate.
- Versioned governed contracts apply a run-mode disposition on ingest (BR-VC-9): a data/security
  payload whose schema MINOR is ahead of the consuming build is rejected in application runs.
- Prefer package export subpaths over deep `dist/` imports.

See [`docs/release-readiness-0.1.0.md`](docs/release-readiness-0.1.0.md) for the full breaking-change
list, bridge-window support matrix, and rollback plan.

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.14 (2026-05-04)

### 🚀 Features

- token diagram created ([b95efa6](https://github.com/EPA-WG/cem/commit/b95efa6))

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))
- repo added into package.json on all sub-projects ([3d19a85](https://github.com/EPA-WG/cem/commit/3d19a85))
- repo `version` added into package.json on all sub-projects ([748bd84](https://github.com/EPA-WG/cem/commit/748bd84))
- Merge pull request #2 from EPA-WG/develop ([#2](https://github.com/EPA-WG/cem/issues/2))
- "version": "0.0.2" ([2362544](https://github.com/EPA-WG/cem/commit/2362544))
- yarn publish:prepare ([e206055](https://github.com/EPA-WG/cem/commit/e206055))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([4c217d7](https://github.com/EPA-WG/cem/commit/4c217d7))
- yarn publish:prepare as publish-prepare.sh ([5350f5d](https://github.com/EPA-WG/cem/commit/5350f5d))
- yarn publish:prepare fix if interrupted ([17a045e](https://github.com/EPA-WG/cem/commit/17a045e))
- bump version test ([3346347](https://github.com/EPA-WG/cem/commit/3346347))
- bump version test 4a ([851daa4](https://github.com/EPA-WG/cem/commit/851daa4))
- bump version test 4b ([76fc015](https://github.com/EPA-WG/cem/commit/76fc015))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([cfdc77f](https://github.com/EPA-WG/cem/commit/cfdc77f))
- restore workspace protocol before nx release ([db35b3f](https://github.com/EPA-WG/cem/commit/db35b3f))
- bump version 0.0.6 ([1c34012](https://github.com/EPA-WG/cem/commit/1c34012))
- cem-theme tokens/*.md added ([4de02de](https://github.com/EPA-WG/cem/commit/4de02de))
- cem-theme tokens/*.md compilation and utf8 validation ([80248bb](https://github.com/EPA-WG/cem/commit/80248bb))
- theme xhtml * title from MD * images copied ([9ef6a92](https://github.com/EPA-WG/cem/commit/9ef6a92))
- theme xhtml * copy css files * .md link points to .xhtml * prismjs colored CSS ([1559a17](https://github.com/EPA-WG/cem/commit/1559a17))
- theme xhtml * internal links recovered ([a85fa2a](https://github.com/EPA-WG/cem/commit/a85fa2a))
- `yarn publish:prepare patch` documented ([a7e9cd7](https://github.com/EPA-WG/cem/commit/a7e9cd7))
- update npm on CDN links ([a574d68](https://github.com/EPA-WG/cem/commit/a574d68))
- README.md updates ([df269b6](https://github.com/EPA-WG/cem/commit/df269b6))
- @epa-wg/cem-theme README.md updates ([2018847](https://github.com/EPA-WG/cem/commit/2018847))

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.13 (2026-05-03)

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))
- repo added into package.json on all sub-projects ([3d19a85](https://github.com/EPA-WG/cem/commit/3d19a85))
- repo `version` added into package.json on all sub-projects ([748bd84](https://github.com/EPA-WG/cem/commit/748bd84))
- Merge pull request #2 from EPA-WG/develop ([#2](https://github.com/EPA-WG/cem/issues/2))
- "version": "0.0.2" ([2362544](https://github.com/EPA-WG/cem/commit/2362544))
- yarn publish:prepare ([e206055](https://github.com/EPA-WG/cem/commit/e206055))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([4c217d7](https://github.com/EPA-WG/cem/commit/4c217d7))
- yarn publish:prepare as publish-prepare.sh ([5350f5d](https://github.com/EPA-WG/cem/commit/5350f5d))
- yarn publish:prepare fix if interrupted ([17a045e](https://github.com/EPA-WG/cem/commit/17a045e))
- bump version test ([3346347](https://github.com/EPA-WG/cem/commit/3346347))
- bump version test 4a ([851daa4](https://github.com/EPA-WG/cem/commit/851daa4))
- bump version test 4b ([76fc015](https://github.com/EPA-WG/cem/commit/76fc015))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([cfdc77f](https://github.com/EPA-WG/cem/commit/cfdc77f))
- restore workspace protocol before nx release ([db35b3f](https://github.com/EPA-WG/cem/commit/db35b3f))
- bump version 0.0.6 ([1c34012](https://github.com/EPA-WG/cem/commit/1c34012))
- cem-theme tokens/*.md added ([4de02de](https://github.com/EPA-WG/cem/commit/4de02de))
- cem-theme tokens/*.md compilation and utf8 validation ([80248bb](https://github.com/EPA-WG/cem/commit/80248bb))
- theme xhtml * title from MD * images copied ([9ef6a92](https://github.com/EPA-WG/cem/commit/9ef6a92))
- theme xhtml * copy css files * .md link points to .xhtml * prismjs colored CSS ([1559a17](https://github.com/EPA-WG/cem/commit/1559a17))
- theme xhtml * internal links recovered ([a85fa2a](https://github.com/EPA-WG/cem/commit/a85fa2a))
- `yarn publish:prepare patch` documented ([a7e9cd7](https://github.com/EPA-WG/cem/commit/a7e9cd7))
- update npm on CDN links ([a574d68](https://github.com/EPA-WG/cem/commit/a574d68))
- README.md updates ([df269b6](https://github.com/EPA-WG/cem/commit/df269b6))
- @epa-wg/cem-theme README.md updates ([2018847](https://github.com/EPA-WG/cem/commit/2018847))

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.12 (2026-05-02)

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))
- repo added into package.json on all sub-projects ([3d19a85](https://github.com/EPA-WG/cem/commit/3d19a85))
- repo `version` added into package.json on all sub-projects ([748bd84](https://github.com/EPA-WG/cem/commit/748bd84))
- Merge pull request #2 from EPA-WG/develop ([#2](https://github.com/EPA-WG/cem/issues/2))
- "version": "0.0.2" ([2362544](https://github.com/EPA-WG/cem/commit/2362544))
- yarn publish:prepare ([e206055](https://github.com/EPA-WG/cem/commit/e206055))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([4c217d7](https://github.com/EPA-WG/cem/commit/4c217d7))
- yarn publish:prepare as publish-prepare.sh ([5350f5d](https://github.com/EPA-WG/cem/commit/5350f5d))
- yarn publish:prepare fix if interrupted ([17a045e](https://github.com/EPA-WG/cem/commit/17a045e))
- bump version test ([3346347](https://github.com/EPA-WG/cem/commit/3346347))
- bump version test 4a ([851daa4](https://github.com/EPA-WG/cem/commit/851daa4))
- bump version test 4b ([76fc015](https://github.com/EPA-WG/cem/commit/76fc015))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([cfdc77f](https://github.com/EPA-WG/cem/commit/cfdc77f))
- restore workspace protocol before nx release ([db35b3f](https://github.com/EPA-WG/cem/commit/db35b3f))
- bump version 0.0.6 ([1c34012](https://github.com/EPA-WG/cem/commit/1c34012))
- cem-theme tokens/*.md added ([4de02de](https://github.com/EPA-WG/cem/commit/4de02de))
- cem-theme tokens/*.md compilation and utf8 validation ([80248bb](https://github.com/EPA-WG/cem/commit/80248bb))
- theme xhtml * title from MD * images copied ([9ef6a92](https://github.com/EPA-WG/cem/commit/9ef6a92))
- theme xhtml * copy css files * .md link points to .xhtml * prismjs colored CSS ([1559a17](https://github.com/EPA-WG/cem/commit/1559a17))
- theme xhtml * internal links recovered ([a85fa2a](https://github.com/EPA-WG/cem/commit/a85fa2a))
- `yarn publish:prepare patch` documented ([a7e9cd7](https://github.com/EPA-WG/cem/commit/a7e9cd7))

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.11 (2026-05-02)

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))
- repo added into package.json on all sub-projects ([3d19a85](https://github.com/EPA-WG/cem/commit/3d19a85))
- repo `version` added into package.json on all sub-projects ([748bd84](https://github.com/EPA-WG/cem/commit/748bd84))
- Merge pull request #2 from EPA-WG/develop ([#2](https://github.com/EPA-WG/cem/issues/2))
- "version": "0.0.2" ([2362544](https://github.com/EPA-WG/cem/commit/2362544))
- yarn publish:prepare ([e206055](https://github.com/EPA-WG/cem/commit/e206055))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([4c217d7](https://github.com/EPA-WG/cem/commit/4c217d7))
- yarn publish:prepare as publish-prepare.sh ([5350f5d](https://github.com/EPA-WG/cem/commit/5350f5d))
- yarn publish:prepare fix if interrupted ([17a045e](https://github.com/EPA-WG/cem/commit/17a045e))
- bump version test ([3346347](https://github.com/EPA-WG/cem/commit/3346347))
- bump version test 4a ([851daa4](https://github.com/EPA-WG/cem/commit/851daa4))
- bump version test 4b ([76fc015](https://github.com/EPA-WG/cem/commit/76fc015))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([cfdc77f](https://github.com/EPA-WG/cem/commit/cfdc77f))
- restore workspace protocol before nx release ([db35b3f](https://github.com/EPA-WG/cem/commit/db35b3f))
- bump version 0.0.6 ([1c34012](https://github.com/EPA-WG/cem/commit/1c34012))
- cem-theme tokens/*.md added ([4de02de](https://github.com/EPA-WG/cem/commit/4de02de))
- cem-theme tokens/*.md compilation and utf8 validation ([80248bb](https://github.com/EPA-WG/cem/commit/80248bb))
- theme xhtml * title from MD * images copied ([9ef6a92](https://github.com/EPA-WG/cem/commit/9ef6a92))
- theme xhtml * copy css files * .md link points to .xhtml * prismjs colored CSS ([1559a17](https://github.com/EPA-WG/cem/commit/1559a17))
- theme xhtml * internal links recovered ([a85fa2a](https://github.com/EPA-WG/cem/commit/a85fa2a))
- `yarn publish:prepare patch` documented ([a7e9cd7](https://github.com/EPA-WG/cem/commit/a7e9cd7))

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.10 (2026-05-01)

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))
- repo added into package.json on all sub-projects ([3d19a85](https://github.com/EPA-WG/cem/commit/3d19a85))
- repo `version` added into package.json on all sub-projects ([748bd84](https://github.com/EPA-WG/cem/commit/748bd84))
- Merge pull request #2 from EPA-WG/develop ([#2](https://github.com/EPA-WG/cem/issues/2))
- "version": "0.0.2" ([2362544](https://github.com/EPA-WG/cem/commit/2362544))
- yarn publish:prepare ([e206055](https://github.com/EPA-WG/cem/commit/e206055))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([4c217d7](https://github.com/EPA-WG/cem/commit/4c217d7))
- yarn publish:prepare as publish-prepare.sh ([5350f5d](https://github.com/EPA-WG/cem/commit/5350f5d))
- yarn publish:prepare fix if interrupted ([17a045e](https://github.com/EPA-WG/cem/commit/17a045e))
- bump version test ([3346347](https://github.com/EPA-WG/cem/commit/3346347))
- bump version test 4a ([851daa4](https://github.com/EPA-WG/cem/commit/851daa4))
- bump version test 4b ([76fc015](https://github.com/EPA-WG/cem/commit/76fc015))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([cfdc77f](https://github.com/EPA-WG/cem/commit/cfdc77f))
- restore workspace protocol before nx release ([db35b3f](https://github.com/EPA-WG/cem/commit/db35b3f))
- bump version 0.0.6 ([1c34012](https://github.com/EPA-WG/cem/commit/1c34012))
- cem-theme tokens/*.md added ([4de02de](https://github.com/EPA-WG/cem/commit/4de02de))
- cem-theme tokens/*.md compilation and utf8 validation ([80248bb](https://github.com/EPA-WG/cem/commit/80248bb))
- theme xhtml * title from MD * images copied ([9ef6a92](https://github.com/EPA-WG/cem/commit/9ef6a92))
- theme xhtml * copy css files * .md link points to .xhtml * prismjs colored CSS ([1559a17](https://github.com/EPA-WG/cem/commit/1559a17))
- theme xhtml * internal links recovered ([a85fa2a](https://github.com/EPA-WG/cem/commit/a85fa2a))
- `yarn publish:prepare patch` documented ([a7e9cd7](https://github.com/EPA-WG/cem/commit/a7e9cd7))

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.9 (2026-05-01)

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))
- repo added into package.json on all sub-projects ([3d19a85](https://github.com/EPA-WG/cem/commit/3d19a85))
- repo `version` added into package.json on all sub-projects ([748bd84](https://github.com/EPA-WG/cem/commit/748bd84))
- Merge pull request #2 from EPA-WG/develop ([#2](https://github.com/EPA-WG/cem/issues/2))
- "version": "0.0.2" ([2362544](https://github.com/EPA-WG/cem/commit/2362544))
- yarn publish:prepare ([e206055](https://github.com/EPA-WG/cem/commit/e206055))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([4c217d7](https://github.com/EPA-WG/cem/commit/4c217d7))
- yarn publish:prepare as publish-prepare.sh ([5350f5d](https://github.com/EPA-WG/cem/commit/5350f5d))
- yarn publish:prepare fix if interrupted ([17a045e](https://github.com/EPA-WG/cem/commit/17a045e))
- bump version test ([3346347](https://github.com/EPA-WG/cem/commit/3346347))
- bump version test 4a ([851daa4](https://github.com/EPA-WG/cem/commit/851daa4))
- bump version test 4b ([76fc015](https://github.com/EPA-WG/cem/commit/76fc015))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([cfdc77f](https://github.com/EPA-WG/cem/commit/cfdc77f))
- restore workspace protocol before nx release ([db35b3f](https://github.com/EPA-WG/cem/commit/db35b3f))
- bump version 0.0.6 ([1c34012](https://github.com/EPA-WG/cem/commit/1c34012))
- cem-theme tokens/*.md added ([4de02de](https://github.com/EPA-WG/cem/commit/4de02de))
- cem-theme tokens/*.md compilation and utf8 validation ([80248bb](https://github.com/EPA-WG/cem/commit/80248bb))
- theme xhtml * title from MD * images copied ([9ef6a92](https://github.com/EPA-WG/cem/commit/9ef6a92))
- theme xhtml * copy css files * .md link points to .xhtml * prismjs colored CSS ([1559a17](https://github.com/EPA-WG/cem/commit/1559a17))
- theme xhtml * internal links recovered ([a85fa2a](https://github.com/EPA-WG/cem/commit/a85fa2a))
- `yarn publish:prepare patch` documented ([a7e9cd7](https://github.com/EPA-WG/cem/commit/a7e9cd7))

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.8 (2026-04-30)

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))
- repo added into package.json on all sub-projects ([3d19a85](https://github.com/EPA-WG/cem/commit/3d19a85))
- repo `version` added into package.json on all sub-projects ([748bd84](https://github.com/EPA-WG/cem/commit/748bd84))
- Merge pull request #2 from EPA-WG/develop ([#2](https://github.com/EPA-WG/cem/issues/2))
- "version": "0.0.2" ([2362544](https://github.com/EPA-WG/cem/commit/2362544))
- yarn publish:prepare ([e206055](https://github.com/EPA-WG/cem/commit/e206055))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([4c217d7](https://github.com/EPA-WG/cem/commit/4c217d7))
- yarn publish:prepare as publish-prepare.sh ([5350f5d](https://github.com/EPA-WG/cem/commit/5350f5d))
- yarn publish:prepare fix if interrupted ([17a045e](https://github.com/EPA-WG/cem/commit/17a045e))
- bump version test ([3346347](https://github.com/EPA-WG/cem/commit/3346347))
- bump version test 4a ([851daa4](https://github.com/EPA-WG/cem/commit/851daa4))
- bump version test 4b ([76fc015](https://github.com/EPA-WG/cem/commit/76fc015))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([cfdc77f](https://github.com/EPA-WG/cem/commit/cfdc77f))
- restore workspace protocol before nx release ([db35b3f](https://github.com/EPA-WG/cem/commit/db35b3f))
- bump version 0.0.6 ([1c34012](https://github.com/EPA-WG/cem/commit/1c34012))
- cem-theme tokens/*.md added ([4de02de](https://github.com/EPA-WG/cem/commit/4de02de))
- cem-theme tokens/*.md compilation and utf8 validation ([80248bb](https://github.com/EPA-WG/cem/commit/80248bb))
- theme xhtml * title from MD * images copied ([9ef6a92](https://github.com/EPA-WG/cem/commit/9ef6a92))
- theme xhtml * copy css files * .md link points to .xhtml * prismjs colored CSS ([1559a17](https://github.com/EPA-WG/cem/commit/1559a17))
- theme xhtml * internal links recovered ([a85fa2a](https://github.com/EPA-WG/cem/commit/a85fa2a))

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.7 (2026-04-26)

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))
- repo added into package.json on all sub-projects ([3d19a85](https://github.com/EPA-WG/cem/commit/3d19a85))
- repo `version` added into package.json on all sub-projects ([748bd84](https://github.com/EPA-WG/cem/commit/748bd84))
- Merge pull request #2 from EPA-WG/develop ([#2](https://github.com/EPA-WG/cem/issues/2))
- "version": "0.0.2" ([2362544](https://github.com/EPA-WG/cem/commit/2362544))
- yarn publish:prepare ([e206055](https://github.com/EPA-WG/cem/commit/e206055))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([4c217d7](https://github.com/EPA-WG/cem/commit/4c217d7))
- yarn publish:prepare as publish-prepare.sh ([5350f5d](https://github.com/EPA-WG/cem/commit/5350f5d))
- yarn publish:prepare fix if interrupted ([17a045e](https://github.com/EPA-WG/cem/commit/17a045e))
- bump version test ([3346347](https://github.com/EPA-WG/cem/commit/3346347))
- bump version test 4a ([851daa4](https://github.com/EPA-WG/cem/commit/851daa4))
- bump version test 4b ([76fc015](https://github.com/EPA-WG/cem/commit/76fc015))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([cfdc77f](https://github.com/EPA-WG/cem/commit/cfdc77f))
- restore workspace protocol before nx release ([db35b3f](https://github.com/EPA-WG/cem/commit/db35b3f))
- bump version 0.0.6 ([1c34012](https://github.com/EPA-WG/cem/commit/1c34012))
- cem-theme tokens/*.md added ([4de02de](https://github.com/EPA-WG/cem/commit/4de02de))
- cem-theme tokens/*.md compilation and utf8 validation ([80248bb](https://github.com/EPA-WG/cem/commit/80248bb))
- theme xhtml * title from MD * images copied ([9ef6a92](https://github.com/EPA-WG/cem/commit/9ef6a92))
- theme xhtml * copy css files * .md link points to .xhtml * prismjs colored CSS ([1559a17](https://github.com/EPA-WG/cem/commit/1559a17))
- theme xhtml * internal links recovered ([a85fa2a](https://github.com/EPA-WG/cem/commit/a85fa2a))

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.6 (2026-01-04)

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))
- repo added into package.json on all sub-projects ([3d19a85](https://github.com/EPA-WG/cem/commit/3d19a85))
- repo `version` added into package.json on all sub-projects ([748bd84](https://github.com/EPA-WG/cem/commit/748bd84))
- Merge pull request #2 from EPA-WG/develop ([#2](https://github.com/EPA-WG/cem/issues/2))
- "version": "0.0.2" ([2362544](https://github.com/EPA-WG/cem/commit/2362544))
- yarn publish:prepare ([e206055](https://github.com/EPA-WG/cem/commit/e206055))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([4c217d7](https://github.com/EPA-WG/cem/commit/4c217d7))
- yarn publish:prepare as publish-prepare.sh ([5350f5d](https://github.com/EPA-WG/cem/commit/5350f5d))
- yarn publish:prepare fix if interrupted ([17a045e](https://github.com/EPA-WG/cem/commit/17a045e))
- bump version test ([3346347](https://github.com/EPA-WG/cem/commit/3346347))
- bump version test 4a ([851daa4](https://github.com/EPA-WG/cem/commit/851daa4))
- bump version test 4b ([76fc015](https://github.com/EPA-WG/cem/commit/76fc015))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([cfdc77f](https://github.com/EPA-WG/cem/commit/cfdc77f))
- restore workspace protocol before nx release ([db35b3f](https://github.com/EPA-WG/cem/commit/db35b3f))
- bump version 0.0.6 ([1c34012](https://github.com/EPA-WG/cem/commit/1c34012))

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.5 (2026-01-04)

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))
- repo added into package.json on all sub-projects ([3d19a85](https://github.com/EPA-WG/cem/commit/3d19a85))
- repo `version` added into package.json on all sub-projects ([748bd84](https://github.com/EPA-WG/cem/commit/748bd84))
- Merge pull request #2 from EPA-WG/develop ([#2](https://github.com/EPA-WG/cem/issues/2))
- "version": "0.0.2" ([2362544](https://github.com/EPA-WG/cem/commit/2362544))
- yarn publish:prepare ([e206055](https://github.com/EPA-WG/cem/commit/e206055))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([4c217d7](https://github.com/EPA-WG/cem/commit/4c217d7))
- yarn publish:prepare as publish-prepare.sh ([5350f5d](https://github.com/EPA-WG/cem/commit/5350f5d))
- yarn publish:prepare fix if interrupted ([17a045e](https://github.com/EPA-WG/cem/commit/17a045e))
- bump version test ([3346347](https://github.com/EPA-WG/cem/commit/3346347))
- bump version test 4a ([851daa4](https://github.com/EPA-WG/cem/commit/851daa4))
- bump version test 4b ([76fc015](https://github.com/EPA-WG/cem/commit/76fc015))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([cfdc77f](https://github.com/EPA-WG/cem/commit/cfdc77f))
- restore workspace protocol before nx release ([db35b3f](https://github.com/EPA-WG/cem/commit/db35b3f))

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.4 (2026-01-04)

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))
- repo added into package.json on all sub-projects ([3d19a85](https://github.com/EPA-WG/cem/commit/3d19a85))
- repo `version` added into package.json on all sub-projects ([748bd84](https://github.com/EPA-WG/cem/commit/748bd84))
- Merge pull request #2 from EPA-WG/develop ([#2](https://github.com/EPA-WG/cem/issues/2))
- "version": "0.0.2" ([2362544](https://github.com/EPA-WG/cem/commit/2362544))
- yarn publish:prepare ([e206055](https://github.com/EPA-WG/cem/commit/e206055))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([4c217d7](https://github.com/EPA-WG/cem/commit/4c217d7))
- yarn publish:prepare as publish-prepare.sh ([5350f5d](https://github.com/EPA-WG/cem/commit/5350f5d))
- yarn publish:prepare fix if interrupted ([17a045e](https://github.com/EPA-WG/cem/commit/17a045e))
- bump version test ([3346347](https://github.com/EPA-WG/cem/commit/3346347))
- bump version test 4a ([851daa4](https://github.com/EPA-WG/cem/commit/851daa4))

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.3 (2026-01-04)

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))
- repo added into package.json on all sub-projects ([3d19a85](https://github.com/EPA-WG/cem/commit/3d19a85))
- repo `version` added into package.json on all sub-projects ([748bd84](https://github.com/EPA-WG/cem/commit/748bd84))
- Merge pull request #2 from EPA-WG/develop ([#2](https://github.com/EPA-WG/cem/issues/2))
- "version": "0.0.2" ([2362544](https://github.com/EPA-WG/cem/commit/2362544))
- yarn publish:prepare ([e206055](https://github.com/EPA-WG/cem/commit/e206055))
- Cannot publish package "@epa-wg/cem-components" because it contains a local dependency protocol in its "dependencies", and your package manager is yarn. ([4c217d7](https://github.com/EPA-WG/cem/commit/4c217d7))

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.2 (2026-01-04)

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))
- repo added into package.json on all sub-projects ([3d19a85](https://github.com/EPA-WG/cem/commit/3d19a85))
- repo `version` added into package.json on all sub-projects ([748bd84](https://github.com/EPA-WG/cem/commit/748bd84))
- Merge pull request #2 from EPA-WG/develop ([#2](https://github.com/EPA-WG/cem/issues/2))
- "version": "0.0.2" ([2362544](https://github.com/EPA-WG/cem/commit/2362544))
- yarn publish:prepare ([e206055](https://github.com/EPA-WG/cem/commit/e206055))

### ❤️ Thank You

- Sasha Firsov @sashafirsov

## 0.0.1 (2026-01-04)

### 🩹 Fixes

- package.json description sync w/ github ([a3a9c6c](https://github.com/EPA-WG/cem/commit/a3a9c6c))
- patch test for publish ([ecf0489](https://github.com/EPA-WG/cem/commit/ecf0489))

### ❤️ Thank You

- Sasha Firsov @sashafirsov