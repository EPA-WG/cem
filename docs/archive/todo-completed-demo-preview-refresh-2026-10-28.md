# Local demo Studio refresh

The local demo host previously reused `/studio/assets/` URLs across copies.
An installed cache-first worker could retain an older shell and runtime despite
`cache-control: no-store` on the HTTP server. The standalone Studio preview had
revision-based asset URLs; the assembled demo host did not.

- [x] Derive a deterministic local Studio revision from the existing deployment
  inventory's Studio entries. New copies that change those entries get new URLs.
- [x] Point the demo home link and canonical Studio redirect at
  `/studio/__cem_preview__/<revision>/`, retaining selection query parameters.
  Serve the graph-emitted HTML with that address as its base. Emitted files,
  inventory hashes and production deployment markup remain unchanged.
- [x] Keep aliases inside Studio, preserve security/MIME headers, refuse
  traversal and symlink escapes, and retain the worker's default revision scope.
- [x] Add Node fixtures for revision changes, stable repeated reads, redirects,
  selection parameters, native worker scope, alias assets and path refusal.
- [x] Reproduce stale shell/bootstrap caches in a disposable deployment/profile.
  Open the new home link, retain an actual saved project and unsaved source
  draft, and verify selected-resource identity and offline reload.
- [x] Verify the current README link, shared field caption and direct CSS output
  rows, restart the local host on port 8081, and open its current URL through
  Windows' default browser. The app-panel open request was also queued.

All six hosting tests, the new stale-cache browser regression and the complete
898-file demo verification pass. This includes unchanged ten-theme CSS output,
120 playgrounds, the live tree gallery, native builds, scoped offline operation
and unchanged emitted Studio assets. Changed hosting/verifier modules pass lint
and whitespace checks.

This change is local hosting/URL transport. It adds no application-local UI
behavior and clears no browser storage. Entered-credential recovery refusal,
checkpoint budgets and disabled automatic whole-Studio recovery remain unchanged.
The Computer Use connector failed before window discovery with the WSL
`sandboxCwd is not a local file URI` error; browser-state verification used an
isolated Playwright profile instead. No user window or saved draft was inspected
or modified by that verification.
