# Studio generated WASM glue lint scope — 2026-10-28

- [x] Reproduce the failure and verify generated staging ownership.
- [x] Exclude only the optional-profile staging directory and preserve authored
  source, script and root-configuration lint coverage/rules.
- [x] Verify lint enumeration, diagnostic probes, cached/uncached lint, shared
  declarative components and isolated credential-policy recovery.

## Cause and scope

The unchanged package-wide lint target reproduced 238 errors and five warnings
in `generated/schema-assets/cem_ql.js` and `cem_ql_factory.js`. The optional
deployment's `scripts/build-schema-assets-profile.mjs` generates those bindings
from the native WASM loader and shared owned-factory generator. This build-owned
staging directory is separate from `dist/` and the existing `dist-schema-assets`
exclusions; it was already ignored by Git but remained visible to ESLint.

The workspace's global ESLint ignore now names only
`packages/cem-studio/generated/schema-assets/**`. It does not exclude all
`generated/` directories or binding-like filenames. The package-wide lint
command and shared authored-code rules remain unchanged. The shared ESLint
config is already an explicit input of Studio's cached lint target.

## Verification

- Uncached Studio lint passes with zero errors/warnings; ordinary and repeated
  cached Nx runs also pass.
- ESLint's actual package enumeration contains all 86 tracked authored JS/TS
  files, with no generated schema-asset bindings and no diagnostics.
- Before/after rule hashes match for source, build script and root Vitest
  configuration. In-memory diagnostic probes at all three paths still produce
  a `no-debugger` error and a `@typescript-eslint/no-unused-vars` warning.
- Both generated bindings and future modules inside their staging directory are
  ignored. Binding-like names under authored source/scripts and a sibling
  `generated/authoring-helper.js` remain eligible for lint.
- The shared declarative architecture gate passes for all 59 components.
- `git diff --check` passes. Existing Nx configuration-age and environment-color
  notices are unrelated; no authored lint rules were weakened.

Only lint configuration and documentation change. Shared UI/theme components,
native/WASM implementations and deployment bytes remain unchanged. No runtime
rebuild is needed. Generated screenshots and Vitest attachments remain excluded.
Entered Login passwords/one-time codes must still block recovery until explicitly
cleared; automatic recovery remains disabled. The 4 MiB group budget, 32,768-unit
witness bound and allocation/resume deadlines remain unchanged.

Final isolated complete Login recovery passes both worker/fallback cases, with
three recoveries, twelve edits and two credential refusals per runtime. Entered
credentials still block recovery until explicitly cleared. All original budgets
and deadlines remain intact.
