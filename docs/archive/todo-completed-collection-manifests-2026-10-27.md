# Shared collection manifest publication — 2026-10-27

- [x] Reproduce the CLI/common manifest mismatch with failing native fixtures before changing publication.
- [x] Share producer-owned manifest and sidecar projections and the established JSON/YAML/XML output writers in `cem-ml`.
- [x] Preserve explicit writer bytes, application JSON, empty collections, child destinations, source maps/spans and retained native owners.
- [x] Verify rebuilt Node/browser WASM, Studio worker builds, saved/offline delivery, credential refusal and deployment limits.

## Contract

`graph_output::collection` owns the stable native collection-control manifest.
It includes `kind`, `mode`, `count`, `bindings` and the established item fields,
including destination and complete child primary values. The runtime-only
collection producer role gates this projection; a JSON object with the same
field names remains ordinary application data. The role is not serialized or
reconstructed from JSON.

`graph_artifact_bytes` gives explicit native writer bytes first priority, even
when empty. Derived collection manifests use the shared structured output
writers moved from CLI, preserving its JSON/YAML/XML bytes and format selection.
Ordinary abstract JSON keeps its existing complete-value encoding. These are
explicit output/control boundaries, never AST input projections or replacement
document owners. Native collection exporter admission is unchanged.

Item source maps and output spans stay out of the published manifest. Common
command responses retain their existing complete inline metadata, typed maps
and native owners. CLI uses the shared sidecar projections, retaining adjacent
file names, child destinations, top-level destination deduplication, console
coloring and transactional writes. Common hosts publish only requested graph
exports; no implicit sidecar or child writes are introduced.

No Studio UI, declarative component, JavaScript UI capability, command protocol
or serialized artifact shape changes. Entered credentials still block recovery;
automatic whole-Studio recovery stays disabled and the checkpoint budget stays
4 MiB. Tests use isolated browser contexts and preserve the user's projects and
drafts.

## Verification

The native command fixture and CLI byte fixture fail before the shared writer
and pass after it. An authored graph against the old WASM also reproduces the
unwanted inline item metadata in publication. The new WASM boundary tests use
the native parser/graph processor and named publication callbacks, never a
JavaScript document parser or runtime AST handoff.

Focused native coverage passes **42 checks**: 15 command execution, seven graph
publication, one encoded-child owner/provenance, four typed collection binding
and 15 folder-build cases. Both authored CLI collection integration cases pass,
including per-item destinations, provenance sidecars and top-level ownership.
The direct/staged CLI test covers the same manifest in JSON, YAML and XML; its
JSON bytes also match the independent golden fixture exactly.

The optimized CEM-ML package passes all **12** Node/browser-binding tests,
including authored native collection graphs, exact published bytes and hashes,
one requested output and retained inline item origins. Package verification
passes for the browser/Node ABI, **657** integrity records and all **32** schema
packages. All **12** CEM-QL WASM boundary scripts pass, including the unchanged
XML byte-roundtrip, independent owners, checkpoint, memory, compilation/reuse
and stylesheet tests.

Studio's **five selected worker tests** pass: the new collection publication
case, CEM and XSLT entry-artifact projects, CEM-QL parameter-map output and the
Supplier pipeline. These exercise real command-worker transport, source edits,
native output/provenance and independent copies. The original credential-refusal
story passes. This is focused acceptance, not a rerun of the complete Studio
source cohort.

Assembled demo acceptance verifies all owned hashes, complete **333-file Studio**
delivery, runtime pairs and independent factory memories, link/subpath navigation,
an independent saved copy, native HTML/CSS preview, offline rebuilds and worker
scope. All **10** theme generators retain the frozen CSS hashes; **120** canonical
playground pages and the live tree gallery pass.

Deployment is **898 files, 114,696,684 bytes (109.38 MiB)**. All four WASMs remain
below 25 MiB, retaining 169/170 exports, 28 imports, the full default capability
profile and pinned Binaryen 132.0.0 `-Oz`:

| Runtime, two deployed copies each | Bytes | MiB | SHA-256 |
| --- | ---: | ---: | --- |
| CEM-QL | 23,515,341 | 22.43 | `6dc9938481aad96c547999508afb1b80998eb153c058c25e4dcd5a71aee8a514` |
| CEM-ML bridge | 24,164,864 | 23.05 | `ee983b5f849e90da6b920496d0e56667af9debfab674ad1430134a77ef4e4422` |

Generated binaries, optimization reports, screenshots and Vitest attachments are
excluded from the commit. User projects and drafts are preserved.

The broader native graph conformance check also finds the existing module-asset
binding defect: a secondary template's `{$datadom.attributes.kind}` renders
empty instead of `document`. The pre-change release CLI reproduces the same
output on the identical authored fixture. Its assertion remains unchanged and
the repair is tracked separately in [todo.md](../todo.md). This task does not
claim that broader suite is green.

The complete CLI unit suite passes **547/548** cases. Its formatter/colorizer
profile case expects plain CEM but receives an HTML `pre.cem-output` wrapper.
Restoring the unchanged HEAD CLI dispatch and running that case in isolation
reproduces the same mismatch; the task's source is then restored byte-for-byte.
The assertion is unchanged and a separate TODO records the console/profile
policy investigation. The earlier aggregate-only Studio fallback module-fetch
follow-up also remains open. Focused acceptance does not stand in for either
complete failing cohort.
