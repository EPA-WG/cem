# Retained CLI schema typecheck inputs — 2026-11-05

- [x] Retain/restore the exact native-generated CLI TypeScript input through the
  CLI build/cache contract and prepare it for standalone runtime typechecking.
- [x] Verify cold builds, exact cache restoration, native version refusal and
  forced project-reference declaration checking without manual schema setup.

The standalone `cem-elements:typecheck` target reproduced TS2307 at
`cem-ml-cli-npm/src/command.ts`: `./generated/command-schema.js` was missing.
The CLI build created the corresponding TypeScript input, compiled it, then
deleted its directory in `finally`. TypeScript's transitive references through
`cem-demo-element` still required that source. The new cold-build regression
also fails against the previous builder because the generated directory is gone.

The [CLI builder](../../packages/cem-ml-cli-npm/scripts/build.mjs) now retains
`src/generated/command-schema.ts`, regenerating the owned directory from the
validated native JSON on each actual build. Nx declares both that directory and
`dist/` as outputs. Generated source is ignored by Git and excluded from authored
build inputs, preventing output/hash feedback. It remains outside the package's
published source list. The existing native emitter and its dependency-output
hashing remain authoritative; no hand-maintained flag table is introduced.

The [runtime typecheck target](../../packages/cem-elements/project.json) now
depends explicitly on the CLI build as well as rendering WASM. A normal clean
invocation prepares all required inputs before `tsc --build --emitDeclarationOnly`.
The [CLI verification target](../../packages/cem-ml-cli-npm/project.json) includes
the new uncached, serial `verify:typecheck-input` integration gate.

The [verifier](../../packages/cem-ml-cli-npm/scripts/verify-typecheck-input.mjs)
uses a private Nx artifact cache **and** metadata database. It requires an actual
cold build, checks the compiled schema against native JSON, deletes both output
roots, and requires a cache hit that restores identical source/JS hashes. An
uncached invalid-version case exercises the unchanged version guard; the original
native JSON is restored in `finally`. Leaf invocations deliberately skip producer
execution; normal builds continue to hash the native emitter's declared outputs.

Forced consumer compilation uses TypeScript's real solution builder with the
runtime's actual project references and declaration-only/force options. Emitted
declarations/build-info stay in an in-memory host; assertions require both CLI
and runtime declarations to be generated. This keeps other packages' processed
publication artifacts intact. CLI source, emitted JS and build-info hashes must
remain unchanged across that check.

## Verification

- The normal configured `cem-elements:typecheck --args=--force` invocation passes
  after deleting the generated input, including all four prerequisites. Existing
  native/WASM prerequisites were rebuilt during this check.
- Deleting that input again and repeating the normal invocation restores the
  exact source hash with **5/5 tasks served from cache**. No manual schema
  preparation is used.
- Full normal CLI `verify` passes: all **61 Node tests**, Chromium worker-pool
  and dedicated-worker command checks, the new cold/cache/version/consumer gate,
  publication integrity and strict browser/Node declaration checks. The graph
  completes eight tasks, four from cache. Original startup deadlines,
  cancellation, physical-memory separation and invalid-bootstrap refusals pass.
- The runtime package was repackaged after the physical declaration check. All
  **373** runtime artifact hashes remain unchanged across the complete CLI
  verification, confirming the in-memory consumer check preserves those files.
- CLI lint passes with zero errors and its existing unused `copyFileSync`
  warning. The frozen presentation archive retains its exact size and SHA-256.

The [README](../../packages/cem-ml-cli-npm/README.md) documents input ownership and
the normal verification commands. Runtime semantics, UI, tokens, recovery and
credential policy are unchanged; this is build/typecheck preparation work.

Commands from the repository root, on Linux x64 / Node 24.16.0 with
`NX_DAEMON=false`:

```sh
yarn nx run cem-elements:typecheck --args=--force
yarn nx run @epa-wg/cem-ml-cli:verify:typecheck-input --excludeTaskDependencies
yarn nx run @epa-wg/cem-ml-cli:verify
yarn nx run @epa-wg/cem-ml-cli:lint --excludeTaskDependencies --skip-nx-cache
```

The focused integration command used prepared dependencies; the full configured
typecheck and CLI verification also passed with their normal dependency graphs.
The next active item is concurrent release-verifier scheduling with its original
deadlines and assertions.
