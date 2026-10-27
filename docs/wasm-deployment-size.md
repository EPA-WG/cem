# Demo deployment size analysis

The complete demo is assembled in `~/cem-bin`, including Studio. Current
deployment policy and optimizer verification are documented below; the separate
historical tables record the smaller gallery-only build from 2026-09-13.
Sizes are MiB (1,048,576 bytes). Release builds, metadata removal, optimization
and sharing the CEM-QL gallery runtime are implemented in `yarn copy:demo`.

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
They then run `wasm-opt -Oz` from the pinned development dependency `binaryen@132.0.0` through
the shared [optimizer](../tools/scripts/wasm-opt.mjs). Install it with the
workspace's normal `yarn install --immutable`; no optimizer download occurs
during the build. Optimization precedes factories, package integrity records,
vendored runtime copies and native Studio graph publication. Identical browser
and Node CEM-ML input binaries share one optimization; different inputs are
optimized independently with their own matching glue.

The optimizer validates both binaries and compares their complete import/export
name/kind descriptors. It refuses invalid output, descriptor changes, size
increases and output at or above 26,214,400 bytes, replacing the input only
after all checks succeed. No closed-world assumptions, feature-expansion flags,
relaxed floating-point semantics or Cargo feature removal are used.
Measurement reports live outside the deployment in
`dist/reports/wasm-optimization/`. Nx hashes the optimizer helper and pinned
Binaryen dependency and restores the corresponding reports with its outputs.

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
Publication also preflights every new file, retained regular file and the
generated inventory against a strict **below 25 MiB** limit. Count or size
overflow stops before copying or retiring files; an oversized unrelated or
modified obsolete file is preserved and blocks publication. `verify:demo`
checks all deployed sizes and hashes and expects the four separately paired
WASM assets. This is a raw-byte limit, independent of HTTP compression.

## Complete-runtime measurements (2026-10-26)

Measured from the current post-bindgen release binaries with all default
capabilities enabled, using pinned Binaryen 132.0.0:

| Runtime (two deployed copies each) | Before, bytes | `-Os`, bytes | Selected `-Oz`, bytes | Selected MiB |
| --- | ---: | ---: | ---: | ---: |
| CEM-QL | 26,630,206 | 23,667,268 | **23,416,451** | **22.33** |
| CEM-ML common-query bridge | 27,408,583 | 24,339,027 | **24,066,033** | **22.95** |

`-Oz` saves 12.07% and 12.20% respectively against the complete inputs, with
2.67 MiB and 2.05 MiB of raw-file headroom. Both modes retain all 169 CEM-QL
exports, 170 CEM-ML exports and 28 imports per module, with equal capability
and protocol manifests and the real native outputs checked below. Export counts
include the memory and wasm-bindgen support exports.

The rebuilt deployment has **898 files, 114,301,242 bytes (109.01 MiB)**.
Its largest file is 24,066,033 bytes. All four deployed WASM files match the
measured `-Oz` hashes: the gallery and Studio each keep their own CEM-QL and
CEM-ML binary/loader pairs. No additional chunks or optional schema packages
are needed to meet either hosting bound.

On this eight-core WSL host, the portable Node optimizer took approximately
423/518 seconds for CEM-QL/CEM-ML `-Os` and 586/592 seconds for `-Oz` (some
measurements overlapped); peak resident memory was approximately 0.82–0.87 GiB.
These are build-tool measurements, not browser latency benchmarks. `-Oz` is
selected for additional deployment headroom; Nx caches the completed build
outputs. The bounded optimizer process has a 30-minute deadline for slower
build hosts, independent of runtime recovery deadlines.

The broader CEM-QL checks exposed an existing XML reload-bundle byte-roundtrip
failure at `tests/reference-transport-wasm.mjs:105`, reproduced with the unchanged
unoptimized binary. The [2026-10-27 native serialization correction](archive/todo-completed-xml-reload-roundtrip-2026-10-27.md)
repairs it without changing the strict assertion or the capability profile.

### Current output after the XML reload correction (2026-10-27)

| Runtime (two deployed copies each) | Post-bindgen input, bytes | `-Oz`, bytes | MiB |
| --- | ---: | ---: | ---: |
| CEM-QL | 26,753,663 | **23,481,853** | **22.39** |
| CEM-ML common-query bridge | 27,533,204 | **24,131,701** | **23.01** |

The shared native serializer now orders passive namespace/schema lookup maps,
with runtime hash lookup and authored declaration order unchanged. The existing
XML roundtrip fixture passes against both the new unoptimized and optimized
WASM builds. Current deployment: **898 files, 114,563,382 bytes (109.26 MiB)**;
all four WASM files remain below 25 MiB with all exports and schemas enabled.
The earlier comparison table remains the measured 2026-10-26 baseline.

## Reproducing optimizer comparisons

Keep isolated copies of the post-bindgen binaries before running the optimized
builders. For a later comparison, regenerate an unoptimized post-bindgen copy
from the retained release Rust artifact with the builders' pinned wasm-bindgen
version:

```bash
wasm-bindgen dist/target/cem_ql/wasm32-unknown-unknown/release/cem_ql.wasm --target web --out-dir /tmp/cem-ql-measure --out-name cem_ql --remove-name-section --remove-producers-section
cp /tmp/cem-ql-measure/cem_ql_bg.wasm /tmp/cem-ql-input.wasm
```

Measure both modes with the pinned CLI, then compare each candidate with its
original binary and matching web glue:

```bash
node node_modules/binaryen/bin/wasm-opt /tmp/cem-ql-input.wasm -Os -o /tmp/cem-ql-Os.wasm
node node_modules/binaryen/bin/wasm-opt /tmp/cem-ql-input.wasm -Oz -o /tmp/cem-ql-Oz.wasm
node tools/scripts/verify-wasm-optimization.mjs packages/cem_ql/dist/wasm/cem_ql.js /tmp/cem-ql-input.wasm /tmp/cem-ql-Os.wasm /tmp/cem-ql-Oz.wasm
```

Repeat for CEM-ML with the retained Rust artifact
`dist/target/cem_ml_npm/wasm32-unknown-unknown/release/cem_ml_transform_cem_ql.wasm`
and matching `packages/cem-ml-npm/dist/wasm/browser/cem_ml.js`.
The comparison initializes separate native instances and requires exact equality
of capability and worker-protocol manifests, imports/exports, query results,
template output, static HTML rendering and source highlighting. Keep the full
embedded schemas and default debug-control capability profile; size reduction
must not come from disabling features or sharing independent native memories.

```bash
node --test tools/scripts/wasm-opt.test.mjs tools/scripts/demo-deployment.test.mjs tools/scripts/serve-demo-site.test.mjs
yarn copy:demo
yarn verify:demo
```

The demo build now also includes CEM Studio at `/studio/`. Its complete native
graph deployment keeps its own runtime assets, cache inventory and scoped
service worker; gallery runtime adapters do not replace Studio's emitted
files. The historical measurements below describe the gallery-only deployment
and predate the separate common-query bridge and Studio assets. The copy command reports current
per-file WASM sizes and the complete deployment total.

## Historical gallery sizes (2026-09-13)

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

That historical generated browser CEM-ML module had 42 public function exports.
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

A second historical cached copy into a custom destination reproduced all 259 files
byte-for-byte and preserves an unrelated destination file.

## Further reductions under the complete-capability contract

The post-bindgen Binaryen measurement and guarded build integration above are
implemented. Additional reductions should start with compiler profiles rather
than removing capabilities:

1. **Compare size-oriented compiler profiles if more headroom is needed.** The release build uses
   the standard optimization level with LTO. Measure `opt-level = "s"` and
   `"z"`, and `codegen-units = 1`, against size and real demo execution time.
   Neither size setting is guaranteed to win. Use a separate WASM profile so
   native CLI performance policy remains independent. See
   [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html).
2. **Profile retained code before changing delivery.** Any future lazy module
   or asset split needs complete native ownership, integrity, offline delivery
   and capability coverage, as well as the 999-file bound. Existing optional
   schema profiles are separate experiments; they are not needed for this
   deployment's size limit.

The default release keeps all exports and `debug-control`. Disabling them or
substituting the bare CEM-QL module for the CEM-ML startup bridge changes the
capability contract and is not an accepted size reduction for this deployment.

## Historical transfer sizes and other assets

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
