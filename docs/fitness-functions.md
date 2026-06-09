# Evolutionary fitness functions — scope

Status: Scope · Owner: [`todo.md`](todo.md) OQ-2 · BRD:
[`content-type-switch.md`](content-type-switch.md) §6.6 (BR-FF-1/2/3)

The BRD requires the evolutionary characteristics to be guarded by objective, automatable
fitness functions, run as **CI-blocking gates on every change**, with **fitness-function-driven
development** (every governed-contract change adds/extends its guard). OQ-2 fixed an eight-item
catalog; six reuse existing gates (`cem_ml_cli:validate-fixtures` / `e2e`, `cem-elements:verify`,
AC-P-V-2..V-8). This doc scopes the **two net-new** checks: **FF-5** (deprecated-form removal
scan) and **FF-6** (SemVer-presence lint).

Both follow the existing verifier style — small Node ESM scripts that assert and exit non-zero,
modeled on [`packages/custom-element/scripts/verify-package-baseline.mjs`](../packages/custom-element/scripts/verify-package-baseline.mjs)
(`assertIncludes` / `assertNotIncludes`) and wired like
[`tools/scripts/verify-cem-elements-substrate.mjs`](../tools/scripts/verify-cem-elements-substrate.mjs)
(run via an Nx target). Each is driven by a small **JSON registry** so FFDD means *editing a
registry entry*, not writing new scan code.

---

## FF-5 — Deprecated-form removal scan

**Status: Implemented** ✓ — `tools/fitness/deprecated-forms.json` (registry),
`tools/fitness/lib.mjs` (shared walk/match helpers), `tools/scripts/ff-deprecated-form-scan.mjs`
(scanner), Nx target `@epa-wg/cem:fitness-removal-scan`, wired into the CI gate
(`.github/workflows/ci.yml` "Run fitness-function gates", always-run). Green; the forbidden-form
fail path is verified.

**Guards:** BR-EV-7 (the contract/removal phase gate of parallel-change) and BR-CO-3 (legacy
must be inventoried). Also a permanent regression guard: a removed form must not reappear.

**Precedent:** `verify-package-baseline.mjs:47-49` already hard-asserts the adapter contains no
`XSLTProcessor` / `createXsltFromDom` / `class DceElement`. FF-5 generalizes that one-off into a
registry-driven workspace scan.

**Registry** — `tools/fitness/deprecated-forms.json`, one entry per form:

```jsonc
{
  "id": "custom-element-v0",
  "pattern": "lang=\"custom-element-v0\"",   // string or /regex/
  "status": "deprecated",                     // "deprecated" | "forbidden"
  "deprecatedSince": "1.4.0",
  "removeAt": "2.0.0",
  "replacement": "type=\"text/cem-ml\"",
  "allowlist": ["packages/cem-elements/src/lib/projection.ts", "docs/**", "**/*.spec.ts"]
}
```

Seed entries: `custom-element-v0`, `cem-ml-v0`, `browser-xslt-1.0` (the `XSLTProcessor` /
`createXsltFromDom` / `DceElement` patterns), and a `xsl-1.0-execution` marker.

**Scan scope:** glob `packages/**` + `examples/**` source (`.ts`, `.js`, `.html`, `.cem`),
excluding `node_modules`, `dist`, and per-entry `allowlist` (the form's own implementation
module, deprecation-test fixtures, and the migration docs are always allowlisted).

**Pass/fail (deterministic):**
- `forbidden` form with ≥1 non-allowlisted hit → **fail** (exit 1), list `file:line`. (regression
  guard / "removed form must not reappear".)
- `deprecated` form past its window — `removeAt` MAJOR ≤ current engine MAJOR and count > 0 →
  **fail**. (the BR-EV-7 removal gate: can't remove while consumers remain, and can't keep it
  past the deadline.)
- `deprecated` form within window → **pass**, print the usage **inventory** (count + locations)
  so removal is data-driven.

**External-window assertion (BR-EV-7 second half):** the scan can't see external consumers, so
for any form it asserts the *published* deprecation metadata exists: `deprecatedSince` MINOR is
at least one MINOR before the `removeAt` MAJOR, and the form is named in the changelog/migration
doc. Metadata-presence only — no network.

**Output:** human-readable table + `--json` report `{ id, status, count, hits[], removeAt }` for
CI artifacts.

**Wiring:** `tools/scripts/ff-deprecated-form-scan.mjs` → Nx target
`@epa-wg/cem:fitness-removal-scan`, to be composed into the CI gate; runs on every change. The
three `verify-package-baseline.mjs` XSLT asserts are **retained** — they uniquely guard the built
`dist/custom-element.js` artifact, which FF-5's source scan excludes; FF-5 adds the workspace
source-wide coverage (and shares the same forbidden patterns via the registry).

**Effort:** Small (~½ day). Glob + per-entry match + the existing assert/exit pattern.

---

## FF-6 — SemVer-presence lint

**Status: Implemented** ✓ — `tools/fitness/governed-contracts.json` (registry),
`tools/scripts/ff-semver-presence.mjs` (scanner, reuses `lib.mjs`), Nx target
`@epa-wg/cem:fitness-semver-presence`, CI-wired alongside FF-5. Green: all nine `required`
contracts resolve real SemVers — `data-snapshot`, `token-outputs`, `patch-transport`, and
`edge-render-state` landed at 1.0.0 and promoted from `pending-version`; **no gaps remain.** The
fail path is verified.

**Guards:** BR-VC-5 (every axis carries an independent SemVer line) and BR-EV-6 (governed
contracts are exactly the enumerated boundary contracts). Catches the two known un-versioned
gaps and blocks any *new* governed contract from shipping unversioned.

**Registry** — `tools/fitness/governed-contracts.json`, one entry per governed contract (the §5
list), each naming where its version is declared:

```jsonc
{
  "id": "data-snapshot",
  "status": "pending-version",                // "required" | "pending-version"
  "locator": { "file": "packages/cem-elements/src/lib/cem-elements.ts",
               "match": "interface DataIslandSnapshot", "field": "version" },
  "tracks": "todo.md What's-left #3"
}
```

Seed: `template-authoring`, `namespace-dispatch`, `patch-transport` (`renderEngineVersion`),
`artifact-cache` (`cemMlVersion`/`cemQlVersion`), `cli-io`, `edge-render-state` (`RenderRevision`)
as `required` (they already declare a version — positive cases to lock in, e.g. `cemQlVersion` in
[`cem-ql-render.ts`](../packages/cem-elements/src/lib/internal/runtime-support/cem-ql-render.ts));
and the two gaps `data-snapshot` (`DataIslandSnapshot`, [`cem-elements.ts:77`](../packages/cem-elements/src/lib/cem-elements.ts))
and `token-outputs` ([`export-tokens.mjs`](../packages/cem-theme/scripts/export-tokens.mjs)) as
`pending-version`.

**Check logic:** for each contract, resolve `locator` and assert the version field/const exists
and its value is a valid SemVer 2.0 string. Use targeted regex against the located file (a
`version:` field near the matched symbol, an exported `const xVersion`, or a JSON `$version` /
`metadata.version`) — no full TS AST parse, matching the repo's pragmatic verifier style.

**Pass/fail:**
- `required` contract missing or with a non-SemVer version → **fail** (exit 1).
- `pending-version` contract → **report** (non-blocking) with its `tracks` pointer, so the gap is
  visible but CI stays green until that work item lands; flipping it to `required` is the
  acceptance test for the "add a SemVer axis" task.
- All `required` present + valid → **pass**.

This makes FF-6 both a guard *and* the driver for what's-left #3: closing the snapshot/token gap
= adding the version field + flipping its registry status to `required`.

**Output:** report `{ id, status, versionFound, valid, tracks }`.

**Wiring:** `tools/scripts/ff-semver-presence.mjs` → Nx target `fitness:semver-presence`, composed
into the CI gate.

**Effort:** Small–medium (~1 day). Locator resolution per contract is the only real work; keep it
regex-based.

---

## FF-1..FF-4, FF-7, FF-8 — gate map (reuse existing gates)

**Status: framework + ready FFs implemented** ✓ — the six non-net-new FFs are wired as named,
CI-blocking gates through a registry + composing runner that mirrors the FF-5/FF-6 style:
`tools/fitness/fitness-gates.json` (registry) + `tools/scripts/ff-gate-run.mjs` (runner) + Nx target
`@epa-wg/cem:fitness-gate-map`, composed into the CI gate alongside FF-5/FF-6.

Unlike FF-5/FF-6 (standalone scanners), these FFs **reuse existing gates** — so the runner does not
re-run those heavy gates; it verifies the **FF→gate→CI mapping is intact** (a drift guard), while the
backing gates do the actual behavioral enforcement when they run in CI. Per FF the registry records
`backing` (the Nx target[s] that enforce it), `guards` (BR/AC refs), and either `evidence`
(fixtures/scripts that must exist, for `active`) or `tracks` (for `tracked`).

Pass/fail (deterministic):
- `active` FF → **fail** unless every backing target is *defined*, at least one backing target is
  *invoked in `.github/workflows/ci.yml`*, and every `evidence` path exists.
- `tracked` FF → **report** non-blocking with its `tracks` pointer (its AC-P-V dispatch fixtures are
  not authored yet); the backing target must still be defined.

Current state (matches `todo.md` OQ-2):
- **active:** FF-1 backward-render (`cem_ml_cli:validate-fixtures`, `cem-elements:verify`; evidence
  `examples/cem-ml/fixture-manifest.json`), FF-2 negotiation determinism (`cem_ml_cli:e2e`,
  `cem_ml:test`; evidence `examples/cem-ml/version-negotiation/core-major-forgiving.cem` +
  `packages/cem_ml/tests/version_negotiation_fixtures.rs`), FF-3 isolation
  (`cem_ml_cli:e2e`/`validate-fixtures`; evidence `schema-scoping/sibling-isolation.cem`),
  FF-8 source-map continuity (`cem_ml_cli:validate-fixtures`; evidence
  `namespace-rebinding/default-html-svg-html.cem`), FF-4 mode-disposition (`cem-elements:test:unit`,
  `cem-elements:test`; evidence `disposition.ts` + `disposition.spec.ts` +
  `projection.disposition.spec.ts`), plus FF-5 and FF-6. CI invokes `cem_ml_cli:validate-fixtures` +
  `cem_ml_cli:e2e` in the fitness-gate step and runs `cem_ml:test` / `cem-elements:test{,:unit}` via
  `nx affected -t test test:unit`, so FF-1/2/3/4/8 are genuinely enforced.
  - **FF-4 scope (BR-VC-9 / AC-P-6.7):** the run-mode disposition over unknown OPTIONAL features per
    governed contract — `RunMode` (application / build-SSR / development) × per-contract class
    (presentation: templates/tokens; data/security: snapshot-datadom / edge-render-state / privacy)
    → reject / degrade, with the BR-VC-8 must-understand override. Decision-core
    `packages/cem-elements/src/lib/disposition.ts`; applied at both data/security ingest seams —
    snapshot hydration (`adoptServerRenderedInstance`) and edge-render-state
    (`readEdgeRenderStateContents`) — via a configurable `runMode` (default `application`). It does
    **not** cover the parser-side AC-P-V-6 verification (below).
- **tracked (deferred — the underlying [B]-tier capability is not built yet, not just the fixture):**
  FF-7 XSLT capability-gating (AC-P-V-4/V-7) needs AC-P-6.8 XSLT region dispatch — the engine has no
  `xsl:` handling and XSLT is absent from the Layer-5 handoff content types. Flipping it to `active` =
  building the capability, then authoring the AC-P-V fixture(s) + integration test and pointing
  `evidence` at them — the FFDD acceptance test for that work, exactly as FF-2/FF-4 here (and as
  FF-6's `pending-version`→`required` flip drove the SemVer-axis task).
- **separate follow-up (not an FF gate):** the parser-side **AC-P-V-6** unresolved-namespace
  disposition — a `cem_ml` region whose namespace resolves to no metadata/schema/rule yielding
  reject/allow/ignore per scope policy + run mode (AC-P-6.7's parser subject). FF-4 covers the
  BR-VC-9 contract disposition that AC-P-6.7 references as the default selector; this parser verifier
  is tracked in `todo.md` as cem_ml work.

## Shared infrastructure & sequencing

- New `tools/fitness/` holds the two JSON registries; new `tools/scripts/ff-*.mjs` hold the two
  scanners; both reuse a shared `assert/report` helper extracted from `verify-package-baseline.mjs`.
- Wire both as Nx targets and add them to the workspace CI gate so they run on every change
  (OQ-2: CI-blocking). A small `fitness` project (or root target) composing
  `fitness:removal-scan` + `fitness:semver-presence` is the natural home.
- **FFDD (BR-FF-3) becomes concrete:** introducing a deprecated form or a new governed contract
  means adding a registry row — the guard grows with the contracts, by config not code.
- Suggested order: FF-5 first (smallest, has a precedent, immediate regression value), then FF-6
  (which also tees up the snapshot/token SemVer-axis work as its `pending-version` → `required`
  acceptance test).
