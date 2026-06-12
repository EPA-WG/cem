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

Next migration steps:

- reconcile the imported snapshot with a history-preserving graft when the branch is
  ready for a dedicated import commit;
- scaffold workspace/Nx package metadata;
- replace the legacy implementation with a `<custom-element>` adapter over
  `packages/cem-elements`;
- keep or explicitly retire legacy browser entrypoints during the next-major package
  plan.
