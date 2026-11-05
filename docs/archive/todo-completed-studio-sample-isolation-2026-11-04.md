# Optional Studio sample generation — 2026-11-04

- [x] Isolate the optional sample graph from the strict default Feature Tour
  generated-file domain, retaining correct relative resource bases.
- [x] Add default → optional → default native-CLI verification and preserve
  rejection of unexpected default generated files.

The optional builder previously wrote
`generated/feature-tour/feature-tour.schema-assets.cem` next to the ordinary
generated inputs. Its build succeeded, but the subsequent default generator
check correctly refused the extra file. The new regression reproduces this
failure against the previous builder; the rejection fixture also observes that
leftover scratch file. Neither failure involves saved projects or drafts.

The [canonical generator](../../packages/cem-studio/scripts/generate-feature-tour.mjs)
now supports graph-only optional emission into
`generated/schema-assets/feature-tour.cem`. It uses the same sample inventory and
graph declarations as the default build. Input literals are resolved from the
ordinary sample domain to the optional graph's location; output literals resolve
into `dist/schema-assets/samples/`. Local archives, run-config globs, schema
package resources, fixtures and documentation retain their correct native bases.
The default graph bytes and exact generated-file checker are unchanged.

The [optional builder](../../packages/cem-studio/scripts/build-schema-assets-profile.mjs)
invokes this generator and the native CLI; it no longer edits a copy of the
default graph or writes into its directory. Nx's optional outputs and ignore
rules drop the obsolete scratch path. The generator and its inventory helper
are explicit optional build inputs. The new
`test:sample-profile-isolation` target depends on the prepared optional build,
runs serially and participates in `verify:schema-assets`.

## Verification

Both [regressions](../../packages/cem-studio/scripts/sample-profile-isolation.test.mjs)
pass in the final run (8.76 seconds):

- Emit default samples with the native CLI, run the actual optional profile
  builder, then check the default file set **before** regenerating anything.
  All 65 generated default files and the complete ordinary deployment retain
  their original hashes. All 107 emitted sample files match between profiles.
- Regenerate/re-emit the default profile and verify both deployments and default
  inputs remain unchanged. A separate injected extra file still fails with the
  original strict file-set diagnostic; removing only that fixture restores the
  successful check.

Studio package-wide lint passes. The final presentation archive still matches
its recorded size and SHA-256. This changes build configuration/generation only;
application UI, styles, theme ownership, native transformation semantics,
credential policy, recovery behavior and deadlines are unchanged. Existing
prepared native CLI and pinned optional WASM/pack artifacts were used; this is
not a fresh WASM rebuild or a new browser/public deployment acceptance.

Commands from the repository root, using Linux x64 / Node 24.16.0 with
`NX_DAEMON=false` and prepared dependencies:

```sh
yarn nx run @epa-wg/cem-studio:test:sample-profile-isolation --excludeTaskDependencies --skip-nx-cache
yarn nx run @epa-wg/cem-studio:lint --excludeTaskDependencies --skip-nx-cache
```

The [Studio README](../../packages/cem-studio/README.md) documents the normal
dependency-prepared target and regeneration of obsolete scratch inputs in older
checkouts. The next unfinished item is the intermittent StaticPreviewRefusals
fresh-worker allocation timeout; its original five-second deadline remains.
