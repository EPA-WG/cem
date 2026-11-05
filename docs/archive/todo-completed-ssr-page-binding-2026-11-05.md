# SSR routing foundation: `{page}` — 2026-11-05

The shared native graph lowerer now supplies the normalized `{page}` filename
binding proposed by Phase A of the whole-page SSR design. It strips one
nonempty registered page suffix (`.cem.md`, `.html`, `.cem`, ASCII
case-insensitive) and preserves every legacy binding. The binding is available
to template paths, parameters and output destinations through the existing graph
lowering path used by filesystem, manifest, Node and browser hosts.

| Filename | `{page}` | Existing `{stem}` |
| --- | --- | --- |
| `guide.cem.md` | `guide` | `guide.cem` |
| `Guide.CEM.MD` | `Guide` | `Guide.CEM` |
| `release.cem.cem.md` | `release.cem` | `release.cem.cem` |
| `guide.html` | `guide` | `guide` |
| `guide.cem` | `guide` | `guide` |
| `guide.md` | `guide.md` | `guide` |
| `.html` | `.html` | `.html` |

Unknown suffixes remain intact. No repeated extension stripping, URI rewriting,
content-type/schema inference or CEM-enabled Markdown activation is added.
Bindings remain strings; URL escaping and page-URL admission belong to explicit
route preflight. The implementation adds one private native filename helper and
one map entry; there is no host-specific JavaScript routing implementation.

Source URI, declared content type/schema, old `{file}`/`{stem}`/`{ext}` values,
stable import ordering and output write authority retain their original owners.
The existing unknown-binding refusal and duplicate-destination preflight stay
strict. Two formats mapping to the same page name cannot overwrite each other.

Native/provider fixtures cover twelve naming vectors, exact old bindings,
filesystem/file-URI/manifest request parity, template and parameter expansion,
source identities and runtime collision preflight. Executable native CLI and
Node/WASM fixtures render the same ordinary native CEMT graph into three output
routes and compare exact HTML bytes across different physical output roots.
The browser worker fixture verifies three actually committed graph exports.
The graph fixtures declare `text/cem-ml` explicitly, including filenames ending
in `.html`/`.md`; they exercise routing independently of source-format inference.

Negative executable fixtures preserve prior published files. Native CLI usage
refusals retain exit 2 and their exact diagnostic; the Node host's current
lowering-refusal exit is 7. Runtime destination collisions retain exit 1. These
existing host classifications are preserved, with exact native error codes.

The native library tests and two executable CLI tests fail before the new map
entry because `{page}` is unknown. The old packaged Node/WASM runtime likewise
passes 61 existing tests and fails only the two new routing tests. Validation
results for the completed increment are recorded in the adjacent receipt.

- [x] Native graph-request cohort: **19/19** pass, including the three new
      routing cases; all three also pass with `--no-default-features` requested.
- [x] Native executable routing tests: **2/2** pass; existing template module
      conformance **8/8** and CLI transform-config tests **34/34** pass.
- [x] Optimized default WASM rebuilt: **24,898,879 bytes**, retaining the existing
      25 MiB limit, 658 integrity records, command schema v1 and native interface.
- [x] Low-level runtime suite: **30/30**, plus complete native XSLT contracts.
- [x] CLI package rebuilt with its current runtime-integrity pin; the cached
      wrapper build was bypassed once to refresh that embedded pin.
- [x] Node service/executable suite: **63/63** pass, including the two new tests,
      legacy-stem collision control and prior-output preservation.
- [x] Full browser command-service verifier passes, including the new three-route
      committed-output case and original startup/cancellation/ownership checks.
- [x] Both npm lint targets pass: zero errors; CLI retains its existing unused
      `copyFileSync` warning in the build script.
- [x] Frozen delivery remains 17,321,262 bytes, SHA-256
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

The [receipt](ssr-page-binding-2026-11-05.json) retains the validation scope,
exact published browser outputs, runtime hashes and frozen delivery identity.
Local command logs are `/tmp/cem-page-binding-*`. Native and Node checks used
their Nx targets, with prepared dependencies for focused runs. Core semantics
were established in native tests before the downstream WASM/browser rebuild;
this is a focused routing acceptance, not a full workspace verification.

The full SSR compiler remains open. Next are distinct source/page/base/output
route preflight and the retained native SSR artifact contract, followed by the
shared page lifecycle and an inline-declaration hydration slice. This increment
does not provide an SSR command, execute page scripts, discover DCE instances,
write islands, implement the `.cem.md` source adapter or claim adoption readiness.
It introduces no UI behavior, CSS or theme-token changes. The accepted
presentation archive retains its original bytes and SHA-256; no public Site
deployment is changed.
