# `@epa-wg/custom-element` Phase 3.6 Inventory

Status: accepted inventory evidence as of 2026-08-19. No external checkout was
modified while collecting it.

This inventory refreshes the June snapshot in
[`custom-element-package-baseline.md`](custom-element-package-baseline.md) against
the current external refs, npm artifact, test repository, and monorepo state. It
deliberately stops before choosing a Git import topology or changing
`packages/custom-element/`.

## Executive Findings

1. The npm release of record is `@epa-wg/custom-element@0.0.39`, built from the
   external repository's `develop` tip `3ce6f57`. The external `main` branch is
   still `0.0.37` at `0282a74`; the branches have diverged by 4/9 commits.
2. The source package has no meaningful local test gate. Its `test` script exits
   successfully after pointing to the separate `custom-element-dist` repository,
   whose `develop` branch contains 88 exported browser stories and three real
   unit cases.
3. The published archive has 76 entries and leaks editor/cache metadata because
   its `files` field is `['*']`. A migrated release must be built from an explicit
   clean package root and verified as an archive, not merely inspected in place.
4. `packages/custom-element/` already exists in this monorepo. It was copied as a
   `0.0.39` source snapshot in commit `338f9cd3` and later converted into a
   `CemElementRuntime` adapter with Nx targets. The external Git objects, branches,
   and tags were not imported, so the roadmap's history-preservation requirement
   remains unsatisfied.
5. The in-repo package is not a neutral import target: it contains valuable
   adapter, fixture, consumer-rewire, and release work. Its source manifest says
   `0.1.0`, its changelog records `0.1.1`, and npm still ends at `0.0.39`. The next
   task must decide how to join the external history to this existing line without
   discarding either history.

## Evidence Sources

| Surface | Read-only source | State inspected |
| --- | --- | --- |
| Package history | `/home/suns/aWork/custom-element/` | clean `main`, local and remote refs, tags, full graph |
| Published package | npm registry and `npm pack --dry-run --ignore-scripts` | `@epa-wg/custom-element@0.0.39` |
| Behavioral reference | `/home/suns/aWork/custom-element-dist/` | clean `main`, `origin/develop`, stories, tests, build/release files |
| Accepted substrate | `packages/cem-elements/` | resolved Nx targets, legacy/material inventories, browser/demo/Edge fixtures |
| Existing destination | `packages/custom-element/` and its Git log | current adapter, Nx/package metadata, fixtures, prior migration docs |

Remote source refs were also checked with `git ls-remote`; no fetch, checkout,
reset, tag creation, or external file write was performed.

## Source Repository And History Shape

The authoritative source repository is
`git@github.com:EPA-WG/custom-element.git`.

| Item | Recorded state |
| --- | --- |
| Checkout | clean `main`, tracking `origin/main`, 0 ahead / 0 behind |
| `main` | `0282a74fb2c79223f7f216627c7d0e997272ed99`, package/tag `0.0.37` |
| `develop` | `3ce6f57c49e5f1e840c2eb3bdad0740838cad4f2`, package `0.0.39` |
| Divergence | `main...origin/develop` = 4 main-only / 9 develop-only commits |
| Graph | 282 commits, one root `e75c240156fb4585e1361016819407d24227a5ab`, not shallow |
| Tags | 32 local tags; normal releases are incomplete and include the anomalous tag `0.05` |
| Published identity | npm `0.0.39` has `gitHead=3ce6f57...`, exactly the external `develop` tip |

The tag set stops at `0.0.37` and omits several npm-published versions, including
`0.0.38` and `0.0.39`. Generic tags such as `0.0.37` cannot be copied directly
into the monorepo tag namespace without a collision policy.

### Existing Monorepo History

The current monorepo package began as a copied npm snapshot, not a Git history
import:

- `338f9cd3` added the `0.0.39` files below `packages/custom-element/`;
- later commits added Nx ownership, the thin adapter, fixtures, theme rewiring,
  CEM-ML/CEM-QL legacy conversion, and release metadata;
- `git log --follow packages/custom-element/custom-element.js` terminates at the
  snapshot commit;
- the external root, `main`, and `develop` commit objects are absent from the
  monorepo object database;
- no external branches or namespaced custom-element release tags exist here.

The earlier [`packages/custom-element/IMPORT.md`](../packages/custom-element/IMPORT.md)
correctly says a history-preserving graft was still pending. The graft must now
preserve both the external history and the subsequent monorepo adapter history.

## Published Package Contract

The `0.0.39` manifest establishes this browser-oriented shape:

| Field | Published value |
| --- | --- |
| Name/version/type | `@epa-wg/custom-element`, `0.0.39`, ESM |
| Browser/module | `custom-element.js` |
| Types | `custom-element.d.ts` |
| Root export | `.` -> `index.js` |
| Subpath exports | `./CustomElement` -> `custom-element.js`; `./package.json` |
| IDE metadata | `ide/web-types-dce.json`, `ide/web-types-xsl.json` |
| License | Apache-2.0 |
| Runtime dependencies | none |

`index.js` default-exports `CustomElement` and re-exports
`custom-element.js`, `http-request.js`, `local-storage.js`, and
`location-element.js`. `module-url.js` is a shipped, directly importable browser
file but is intentionally absent from the root barrel and export map. Importing
the browser files registers `custom-element`, `http-request`, `local-storage`,
`location-element`, or `module-url` as a global side effect.

There are two additional compatibility traps:

- `custom-element.js` exposes a broad helper surface, including DOM/XML helpers,
  merge/identity helpers, slice helpers, and XSLT/XPath implementation symbols.
  The declaration file documents a different, narrower helper set. The actual JS
  and declared TypeScript surfaces are already out of sync.
- the current monorepo adapter preserves only a subset of those runtime helper
  exports while adding adapter APIs such as `getCustomElementRuntime()` and
  settlement helpers. Whether the next-major intentionally breaks, aliases, or
  restores the old helper surface belongs to the package-boundary decision.

### Packed Artifact

`npm pack @epa-wg/custom-element@0.0.39 --dry-run --json --ignore-scripts`
reported:

- 76 entries;
- 222,856 packed bytes;
- 1,084,410 unpacked bytes.

Because `files` is `['*']`, the artifact includes source, demos, docs and IDE
metadata together with private/build metadata: `.claude`, `.idea`, `.vs`,
`.vscode`, `.github`, `.gitignore`, and `.editorconfig`. In particular, the
published `.vs` tree contains workspace databases and indexes. These bytes are a
published accident, not compatibility surface.

The existing monorepo build already stages a curated `dist/`, but its verifier
checks directory presence rather than the final `npm pack` inventory. Archive
cleanliness and clean-consumer imports therefore remain unproven.

## Build, Test, And Release Topology

### Source Package

- There is no compile/build script; the package ships loose browser JavaScript.
- `typings` invokes an unpinned `npx` TypeScript declaration build.
- `start` installs `@web/dev-server` on demand.
- `test` is a successful no-op that refers consumers to `custom-element-dist`.
- The only workflow runs on GitHub Release creation, uses Node 16 and
  `actions/checkout@v3`/`setup-node@v3`, runs the no-op test, then publishes to
  GitHub Packages. It is not evidence for npmjs publication or runtime behavior.

### Behavioral Reference Repository

`git@github.com:EPA-WG/custom-element-dist.git` is a separate, clean repository:

| Item | Recorded state |
| --- | --- |
| `main` | `9887eec720704ec33e5c37e73a07a7437b2ed0f1` |
| `develop` | `49d20ab3d1faf9659eb57493eb81abc148a61ec4` |
| Extra remote ref | `origin/sb-upgrade` at `67f5393826c04f2c59ecfb3a31fff52fa1392baa` |
| Divergence | 3 main-only / 8 develop-only commits |
| Graph | 469 commits, 12 tags, not shallow |
| npm `0.0.39` | `gitHead=63ef0cfd...`, an ancestor of the `develop` version-bump tip |

Its Yarn/Vite/Vitest/Playwright/Storybook build copies the installed source
package into `src/custom-element`, runs browser tests, builds Vite bundles, and
builds Storybook. The prepublish script removes and regenerates `dist`,
`coverage`, and `storybook-static`; the package then includes all three plus a
broad `files` surface. Its GitHub workflow deploys `src` over SFTP on pushes and
manual dispatch, but does not provide a normal CI verification gate.

The current `develop` story surface exports 88 browser cases:

| Category | Cases | Category | Cases |
| --- | ---: | --- | ---: |
| attributes | 9 | scoped CSS | 3 |
| DOM merge | 5 | external templates | 12 |
| form/validity | 5 | HTTP request | 5 |
| local storage | 4 | location | 2 |
| module URL | 4 | set URL | 2 |
| slice events | 9 | slots | 7 |
| version selection | 1 | XSLT conditionals | 15 |
| XSLT `for-each` | 3 | XSLT `if` regression | 1 |
| import-map frame | 1 | **Total** | **88** |

`src/custom-element.test.ts` adds three real helper cases (`deepEqual`,
`xml2dom`/`xmlString`, and `obj2node`). `src/sum.test.ts` is generated placeholder
coverage and is not migration behavior.

## Runtime Ownership Map

The next-major package should keep a facade, not a second engine:

| Legacy responsibility | Accepted owner after adoption | Package compatibility responsibility |
| --- | --- | --- |
| `<custom-element>` definition and old declaration normalization | `packages/custom-element` facade calling `CemElementRuntime` | keep the public tag, import side effect, `tag`/`src`/`hidden`, and omitted-tag window if accepted |
| Produced-tag registration and collisions | scoped declaration registry plus document-global registration in `cem-elements` | translate public declarations; do not create a second produced-element registry |
| Inline/external template loading | CEM runtime source resolver, host loader, module-map and scope policy | preserve legacy URL/fragment authoring through host-controlled resolution |
| HTML+XSLT parsing and XPath lowering | `cem_ml` legacy adapter -> canonical CEM-ML -> `cem_ql` | normalize only the explicitly accepted bridge language; never restore browser `XSLTProcessor` |
| Live XML `/datadom` | inert `DataIslandSnapshot` and CEM-QL record projection | preserve observable values where migration fixtures require them, not a live XML DOM |
| Attribute defaults/select/exposure | declaration compiler, snapshot attributes and CEM-QL | decide old helper/select spellings at the bridge boundary |
| Slices, multi-events, form data and validity | `CemElementRuntime` event/data state | pass old attributes into the one substrate state machine |
| Slots and payload | substrate payload capture and render-plan projection | preserve author-facing slot/payload semantics |
| Scoped styles | substrate light-DOM scope rewriting | prove public adapter containment and prevent legacy page-global leakage |
| DOM identity merge | revisioned patch frames and main-thread reconciliation | prove identity/focus/selection through the public adapter |
| HTTP/storage/location/module resources inside templates | host-policy resource controls and focused substrate primitives | keep directly imported companion modules only as an intentional browser compatibility surface |
| Standalone companion element imports | `packages/custom-element` browser shims | preserve filenames, exports and registration side effects unless the next-major contract explicitly breaks them |
| Edge/SSR processing | clone-safe substrate host envelopes and export policy | aggregate the package gate with the isolated Edge/SSR lane; do not let the facade bypass policy |

## Accepted Evidence And Remaining Migration Evidence

The current substrate already owns substantial behavior: 12 file-backed
legacy/CEM pairs, 8 material/CEM pairs, 15 executable demo pages, 114 browser
cases, 133 default unit cases, and the isolated 6-browser/16-unit Edge/SSR lane.
The existing `packages/custom-element` smoke fixture also covers source and built
entrypoints, registration side effects, omitted-tag inline rendering, inline and
external `src`, substrate data islands, and the four companion elements.

Those are reusable inputs, but they do not close the refreshed adoption work:

1. **History-provenance migration gate.** After import, verify that the accepted
   rewritten source refs and namespaced tags exist, the recorded external tips are
   reachable, and the current package tree remains connected to both the external
   and monorepo adapter histories.
2. **External reference-corpus manifest.** Record all 88 browser cases and three
   real unit cases by stable source/category, mapping each to an existing accepted
   gate, a package-adapter case, or an explicit bridge rejection. Fail when a
   reference case silently disappears or is left unmapped.
3. **Public adapter parity matrix.** Extend the existing source/dist browser smoke
   fixture through the current substrate to cover multi-event and multi-slice
   updates, checkbox/radio coercion, form/custom validity, scoped-style
   containment, and DOM identity/focus across rerender. This proves delegation at
   the public `<custom-element>` boundary rather than only in `cem-elements`.
4. **Packed archive clean-consumer gate.** Pack the actual release root, assert the
   intentional entry list and private/generated exclusions, verify root and
   subpath JS/type contracts, and load the package from a clean temporary browser
   consumer.

These four fixtures are now explicit Phase 3.6 checklist items. Existing legacy,
material, demo, companion, and Edge/SSR fixtures should be reused rather than
duplicated.

## Decision Boundary For The Next Item

No source should be copied and no history should be rewritten until these choices
are accepted together:

- retain only the npm-producing `develop` lineage, or retain both divergent source
  branches and all release-tag history;
- namespace the 32 legacy tags, document missing npm-version tags, and choose
  whether to create namespaced aliases for `0.0.38`/`0.0.39`;
- join the imported, path-prefixed history to the existing snapshot/adaptation
  history, or replace/replay the current package directory;
- retain the separate `custom-element-dist` Git history, or retain only a
  provenance-locked reference manifest and selected fixture bytes.

Recommended direction for the decision: path-filter the external source history
under `packages/custom-element/`, retain both `main` and `develop`, namespace its
32 real tags, and merge that graph with the existing monorepo line while keeping
the current adapter commits. Treat `3ce6f57` as the published-content baseline.
Do not merge the 469-commit distribution repository into the product history;
instead, preserve its pinned refs and 88+3 behavioral surface in the reference
manifest. The next task must still lock the exact commands, temporary refs,
rollback branch, tree-resolution rule, and post-import checks before executing
this recommendation.
