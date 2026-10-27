# Demo deployment size analysis

Measured on 2026-09-13 using the broader demo assembled in `~/cem-bin`.
Sizes below are MiB (1,048,576 bytes). Release builds, metadata removal, and
sharing the CEM-QL gallery runtime are implemented in `yarn copy:demo`.

## Release build commands

```bash
yarn build:demo                       # build all demo dependencies
yarn copy:demo                        # build, then copy to ~/cem-bin
yarn copy:demo /tmp/cem-site           # optional destination
yarn serve:demo                       # serve ~/cem-bin at localhost:8081
yarn nx run cem_ql:build:wasm          # CEM-QL browser release artifacts
yarn nx run @epa-wg/cem-ml:build       # CEM-ML browser and Node release artifacts
```

Both WASM builders use Cargo `--release`, the workspace's existing release LTO,
and wasm-bindgen's `--remove-name-section --remove-producers-section`.
`copy:demo` builds the dependencies through Nx and verifies that both packaged
CEM-QL loader/binary pairs match the canonical release output before copying.
Theme cache inputs include dependent runtime outputs so cached builds cannot
silently restore an older debug runtime.

These commands build locally. They do not publish packages or upload the site.

Current demo assembly has a 999-file limit. It omits 122 duplicate component
HTML pages and replaces the 123-file theme-vendored runtime with imports from
the primary runtime. Canonical playgrounds, component declarations/styles,
token documentation and Studio's complete 333-file deployment remain present.
`demo-deployment.json` records managed file hashes; unchanged obsolete outputs
are retired while modified obsolete files and unrelated files are preserved and counted.
The first migration uses hash-proven previous outputs, never a recursive purge.
The reduced deployment contains 898 files, including the inventory.

The demo build now also includes CEM Studio at `/studio/`. Its complete native
graph deployment keeps its own runtime assets, cache inventory and scoped
service worker; gallery runtime adapters do not replace Studio's emitted
files. The historical measurements below describe the gallery-only deployment
and predate the separate common-query bridge and Studio assets. The copy command reports current
per-file WASM sizes and the complete deployment total.

## Measured sizes

| Artifact | Before | Release without name/producer sections | Shared release deployment |
| --- | ---: | ---: | ---: |
| CEM-QL WASM, each copy | 35.66 (2 copies) | 13.92 (2 copies) | 13.92 (1 copy) |
| Separate CEM-ML WASM | 12.68 | 11.76 | Not deployed |
| Entire deployed directory | 92.25 | 47.85 | **22.09** |

The original CEM-QL build used Cargo's debug profile; CEM-ML already used release.
The original CEM-QL file contained 9.58 MiB of function-name metadata. Removing
that metadata alone leaves 26.08 MiB. Release optimization is therefore a
substantial additional saving, beyond metadata removal.

The release CEM-QL artifact is 14,600,141 bytes; CEM-ML is 12,328,514 bytes.
The assembled site now has 259 files totaling 23,160,227 bytes. Sharing saves
53.8% against the previous release deployment; combined with the release build,
the total reduction from the original deployment is 76.1%.
Its single WASM file is below the current Cloudflare Pages 25 MiB asset limit. See
[Pages limits](https://developers.cloudflare.com/pages/platform/limits/).

## Why the two modules differ

[`cem-ql`](../packages/cem_ql/Cargo.toml) depends on
[`cem-ml`](../packages/cem_ml/Cargo.toml). Rust statically links the reachable
library code into the CEM-QL WASM module.

- CEM-ML owns parsing, validation, source highlighting, static HTML rendering,
  module resolution, and command/operation services.
- CEM-QL adds query compilation/evaluation and retained component-template
  compilation/rendering.

The current generated browser CEM-ML module has 42 public function exports.
The CEM-QL module exposes all 42, plus 11 query/template functions. In particular,
it already exposes `highlightSourceToHtmlV1` and `renderCemMlToHtmlV1`: the two
functions used by the demo viewer's
[`cem-ml-runtime.ts`](../packages/cem-demo-element/src/cem-ml-runtime.ts).

That historical deployment did not require a separate CEM-ML binary. The
current common-query bridge adds native startup registration through
`initialize_common_query_runtime`; the bare CEM-QL build does not export that
initializer. Current assembly therefore preserves the CEM-ML loader/binary pair
and shares only matching CEM-QL artifacts. It does not replace native startup
registration with a JavaScript compatibility stub.

## Implemented deployment sharing

The [copy script](../tools/scripts/copy-demo-site.mjs) assembles one canonical
CEM-QL browser loader/binary pair under `packages/cem_ql/dist/wasm/` and routes
the element, component and theme-generator imports through
`runtime.js` in that directory. Package-local bundles remain available for
independent package distribution; deduplication happens during site assembly.

The generated JavaScript and its matching WASM are copied together. Replacing just the
CEM-ML binary while keeping its old JavaScript glue is not an ABI-safe substitution.
The primary and theme-vendored `cem_ql.js` files become small ES-module re-export
shims. Their factory modules re-export the canonical generation factory, whose
binary URL resolves next to the canonical WASM. Each factory call still creates
an independent memory. Their relative imports also work below a URL prefix.
Before writing, the script checks the packaged CEM-QL bytes against release
outputs, verifies the generation factory and rejects unexpected extra WASM
dependencies. The CEM-ML bridge and the complete Studio subtree keep their
matching native loaders/binaries.

Legacy CEM-QL consumers in a JavaScript realm use one initialization promise.
This avoids racing the asynchronous initializer and replacing the module's
retained-template state. Each worker still has its own runtime instance and
memory; sharing the URL reduces shipped/downloaded bytes, not worker isolation.
Observers and resolver registrations also become shared within that realm;
this needs an explicit contract for consumers beyond the demo viewer.

After copying the shared runtime and adapters, inventory-based cleanup retires
unchanged obsolete outputs, including superseded CEM-QL files. It checks
destination paths for symlinks before writes or removals and preserves modified
and unrelated files. Future renamed/deleted managed outputs are cleaned up on
the next copy without clearing the destination.

Historical gallery-only browser verification covered the NPM picker,
all eight component workflows, the demo viewer, syntax highlighting/error
example, and all ten theme generators (14 pages). The syntax-coloring page's
deliberately invalid example retains its expected error state. All eight
navigation targets returned HTTP 200. Separate checks confirmed that concurrent
initializers returned the same promise and instance, retained templates survived
calls through different adapters, and a module worker can initialize/render
using the shared URL. The runtime checks also run under `/preview/` to verify
relative module URLs. No removed WASM URL is requested. This is deployment
evidence, not a promise that every standalone CEM-ML consumer can be substituted
without further checks.

A second cached copy into a custom destination reproduces all 259 files
byte-for-byte and preserves an unrelated destination file.

## Further binary reductions, in order

1. **Compare size-oriented compiler profiles.** The current release build uses
   the standard optimization level with LTO. Measure `opt-level = "s"` and
   `"z"`, and `codegen-units = 1`, against size and real demo execution time.
   Neither size setting is guaranteed to win. Use a separate WASM profile so
   native CLI performance policy remains independent. See
   [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html).
2. **Measure wasm-opt after wasm-bindgen.** Compare `-Os` and `-Oz`, then rerun
   browser and worker checks. The tool was not installed for this analysis; no
   saving is claimed. See the
   [Binaryen optimizer](https://github.com/WebAssembly/binaryen).
3. **Make browser exports intentional.** The demo uses a fraction of the 53
   exported functions. Exported command-service, inspection, and other APIs can
   retain large reachable dependency graphs even if application JavaScript
   never calls them. Introduce an explicit deployment feature/export surface,
   then use a WASM size profiler to identify retained code. Keep the full
   authoring/command runtime available as a separate package or lazy-loaded tool.
4. **Evaluate disabling `debug-control`.** This optional Rust feature is still
   enabled in release builds. It controls debugger capabilities and exports;
   it is distinct from compiler debug metadata. `--no-default-features` changes
   the API/capability profile, so verify worker controls and manifests before
   adopting it. The existing CEM-ML `build:stripped`/`verify:stripped` targets
   describe that contract.

## Transfer size and other assets

| Release binary | Raw | Gzip level 9 | Brotli quality 6 |
| --- | ---: | ---: | ---: |
| CEM-QL | 13.92 | 3.87 | 2.88 |
| CEM-ML | 11.76 | 3.32 | 2.48 |

These are local compression measurements, not observed production response sizes.
HTTP compression reduces network transfer; it does not remove duplicate files
from the deployment. Cloudflare supports compression for `application/wasm`;
check actual `Content-Encoding` responses after deployment. See
[Cloudflare content compression](https://developers.cloudflare.com/speed/optimization/content/compression/).

Content-hashed shared runtime URLs can use long-lived immutable caching, while
the changing demo HTML continues to revalidate. A symlink or filesystem hard
link alone does not establish a common browser URL or portable static upload.

After WASM sharing, inspect the PNG illustrations (especially `sufler.png` and
`action-colors.png`), unused runtime fixtures, and unreferenced generated files.
Preserve readable illustrations and the full requested gallery. Pre-rendering
static documentation/source highlighting can defer runtime downloads, but the
interactive CEM components and live CSS generator demos still need their engine.
