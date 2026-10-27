# Studio in the demo site — completed 2026-10-26

- [x] Add `@epa-wg/cem-studio` to `build:demo` and link **CEM Studio** from the
  assembled demo home page. Copy the complete native graph output to `/studio/`,
  including samples, module maps, cache inventory, service worker and its own
  matching WASM assets. Preserve its emitted bytes and declarative UI.
- [x] Carry Studio's canonical security headers into the static `_headers`
  subtree and local preview. Keep the service worker scoped to `/studio/`.
  Add `yarn serve:demo [destination]`, defaulting to `~/cem-bin` on port 8081.
- [x] Add four Node hosting fixtures: directory redirects with URL selections,
  MIME/HEAD behavior, Studio-only headers, bounded worker scope and refusal of
  encoded traversal, external symlinks, malformed paths and unsupported methods.
- [x] Repair the copy script's obsolete CEM-ML/CEM-QL interchangeability check.
  The current CEM-ML bridge adds native common-query startup registration, so
  preserve its own loader/factory/binary pair. Continue sharing matching CEM-QL
  gallery artifacts and route vendor factory imports to the canonical factory
  code/binary, retaining a separate heap for each generation. No JavaScript
  native-runtime compatibility stub or Studio-specific UI behavior was added.
- [x] Run `yarn copy:demo`: six projects and 16 dependency tasks build
  successfully; 1,134 files plus eight generated adapters/navigation files are
  assembled. Studio retains its independent graph deployment.
- [x] Add and run `yarn verify:demo`: all 333 Studio files and paired gallery
  loader/factory/WASM files match build outputs byte-for-byte. Every demo home
  link returns HTTP 200. Legacy CEM-QL adapters share initialization, factory
  memories remain distinct, and native CEM-ML rendering/highlighting pass.
- [x] Follow the real Studio link in an isolated browser context. Verify
  subpath URL selection, an independent saved SCSS project clone, exact native
  HTML/CSS build output, styled preview and offline rebuild. The scoped worker
  controls Studio and leaves the demo home page uncontrolled.
- [x] Start the demo preview at `http://localhost:8081/` and queue its browser
  tab in the app.
  Existing saved projects/drafts and the original Studio origin remain intact;
  verification uses temporary browser storage. Entered-credential recovery
  refusal and disabled automatic whole-Studio recovery are unchanged.

The README documents build, copy, serve and verification commands. The WASM size
document distinguishes its historical gallery-only measurements from current
common-query/Studio deployments. The initial integration copy reported
125.15 MiB and 1,142 files; the subsequent
[inventory-based reduction](todo-completed-demo-file-budget-2026-10-26.md)
keeps the complete Studio deployment while reducing the site to 898 files.
Its four WASM files are reported individually. No binaries, screenshots or
Vitest attachments are added to Git.
