# Import Notes

This directory is the Phase 3.6 import target for the published
`@epa-wg/custom-element` package.

Initial source snapshot:

- copied from `node_modules/@epa-wg/custom-element/`
- package version: `0.0.39`
- copied before adapter/scaffold work so the workspace starts from the currently
  consumed published package behavior
- editor/cache directories from the installed artifact (`.claude/`, `.idea/`, `.vs/`)
  were intentionally omitted

History source:

- local checkout: `/home/suns/aWork/custom-element/`
- remote: `git@github.com:EPA-WG/custom-element.git`
- inspected commit: `0282a74`
- package version in checkout: `0.0.37`
- release tags present through `0.0.37`

The local checkout remains the history source. The installed `0.0.39` package remains
the behavior baseline because it contains browser fixes not present in the local
`0.0.37` checkout. See
[`../../docs/custom-element-migration-scope.md`](../../docs/custom-element-migration-scope.md)
and
[`../../docs/custom-element-package-baseline.md`](../../docs/custom-element-package-baseline.md).

History import completed 2026-08-19:

- tree-neutral three-parent join: `dfe142be5dadd7d84010b3954c9532ff2f85ddcb`;
- rewritten source `main`: `32e1246f01ebc7e46303a1c52fea49d390a7e9f9`;
- rewritten source `develop` / npm `0.0.39` lineage:
  `c99632f04d4115c6a2eeec992fbd3003bc822d3d`;
- all 282 source commits are reachable, with 32 release tags namespaced as
  `custom-element-v*` plus two permanent source-tip tags;
- the join tree is byte-identical to its monorepo first parent, preserving all
  existing adapter work;
- `test-fixtures/history-provenance.json` and the uncached Nx `verify-history`
  target enforce the imported topology.

Reference-corpus mapping completed 2026-08-19:

- `test-fixtures/external-reference-corpus.json` pins the excluded distribution
  repository's `develop` tree and all 18 contributing blob IDs;
- its 87 Storybook exports, one import-map harness, and three helper unit cases
  are individually mapped to accepted, package-adapter, or rejected-bridge
  evidence;
- the cached Nx `verify-reference-corpus` target enforces the immutable 88+3
  inventory without an external checkout or network access.

See
[`../../docs/custom-element-history-import-plan.md`](../../docs/custom-element-history-import-plan.md)
for the accepted procedure and package boundary.

Remaining migration steps:

- complete the public-adapter parity matrix and clean packed-archive gate;
- complete the next-major export/type gates;
- keep or explicitly retire the legacy browser bridge from fixture evidence.
