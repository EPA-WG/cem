# Demo file budget and owned cleanup — completed 2026-10-26

- [x] Omit 122 duplicate/obsolete built component HTML pages from the demo
  assembly. Retain all 120 canonical playground pages, component XHTML,
  styles, bundle and catalog. Standalone package build outputs remain intact.
- [x] Rebase the copied theme generator's three runtime imports to the primary
  `cem-elements` library and omit its 123-file vendored tree. Retain the
  common-query CEM-ML bridge and matching CEM-QL loaders, generation factories
  and binaries; Studio keeps its complete native graph deployment.
- [x] Add `demo-deployment.json` with deterministic sorted managed paths, byte
  sizes and SHA-256 hashes. Publish the inventory by atomic rename after
  verifying copied bytes and cleanup. The inventory counts as a deployed file.
- [x] Retire stale files only when their bytes match previous ownership
  records. The first migration recognizes known earlier source copies and
  deployment adapters by hash. Modified obsolete files and unrelated files
  remain untouched and count toward the file limit. Matching current outputs
  continue to be updated by the build.
- [x] Refuse invalid/escaping inventory paths, duplicate records, malformed
  inventories, symlinked managed destinations and updates exceeding 999 total
  files before writing or deleting deployment outputs. Recheck obsolete hashes
  immediately before removal. Do not recursively purge the destination.
- [x] Six deployment fixtures and four hosting fixtures pass: deterministic
  binary inventories, legacy migration, later stale cleanup, modified/unrelated
  preservation, preflight overflow, malformed/escaping metadata, symlinks,
  redirects, MIME/HEAD behavior and Studio-only headers.
- [x] The real migration removes exactly 245 unchanged obsolete files and
  adds the inventory: **1,142 − 122 − 123 + 1 = 898**. The deployment is
  121.51 MiB with its four matching WASM files. A repeated copy removes zero
  files and produces an identical inventory. A clean temporary deployment
  also produces an identical inventory and is removed after verification.
- [x] `yarn copy:demo` builds six projects and 16 dependency tasks. The deployed
  verifier checks every inventory hash and file count, all 333 unchanged Studio
  files, paired native gallery assets, shared legacy initialization, separate
  generation memories and real native rendering/highlighting.
- [x] Capture all ten theme generators before migration and compare their
  complete generated CSS after rebasing: every hash remains unchanged. All
  canonical playground pages return HTTP 200 and the live tree gallery renders.
- [x] Studio's real demo link, subpath URL selection, independent saved SCSS
  project clone, exact native HTML/CSS preview, offline rebuild and isolated
  service-worker scope pass. Browser verification uses temporary storage and
  does not clear existing user projects or drafts.
- [x] Entered-credential recovery refusal passes on worker and fallback
  runtimes. Automatic whole-Studio recovery remains disabled. No UI behavior,
  token definitions, native parser or credential policy is changed.

The demo remains available at `http://localhost:8081/`, with Studio under
`/studio/`. Source, release artifacts and user data are preserved; the reduction
applies only to the consumer demo deployment. Generated binaries, screenshots
and Vitest attachments remain outside Git.
