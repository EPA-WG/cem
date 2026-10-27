# Native module-asset attribute conformance — 2026-10-27

- [x] Reproduce the native CLI conformance failure before changing its template.
- [x] Preserve the exact original output assertion while using adopted native metadata navigation.
- [x] Add native adapter fixtures separating metadata, authored/missing attributes and implicit host fields, with original AST allocation, source spans and read-only repeated rendering.
- [x] Add shared imported/named-module fixtures with explicit parameters and secondary assets for CLI, Node/browser WASM and Studio worker acceptance.
- [x] Complete credential, package-integrity and assembled-demo verification and archive evidence for delivery.

## Cause and correction

The conformance fixture used `datadom.attributes.kind` to obtain the document's
metadata kind. That expression relied on the former record-shaped document
view's automatic host-field bindings. Native artifact ingress deliberately
retains a CEM document node; its metadata is available at `input.kind`, while
authored element attributes remain native attribute nodes. This contract was
already documented in the bridge README and the
[earlier CLI fixture migration](todo-snapshot-2026-10-07.md).

The original CLI case fails before correction: its secondary output is empty
where the assertion requires `document`. Changing the authored expression to
`input.kind` makes the exact existing HTML expectation pass. Production input
loading, renderer/adapter behavior and binding rules are unchanged. There is no
record fallback, synthetic source attribute, AST serialization handoff or
application/component JavaScript workaround.

## Coverage

The native adapter integration fixture checks three inputs: an absent `kind`
attribute, `@kind=asset` and `@kind=document`. Across repeated renders it verifies
document/element metadata, authored values, absent implicit host metadata,
unchanged AST allocation and attribute/source state, and output spans retaining
the original source ID. Source nodes are emitted through the native renderer.

The shared graph fixture has a primary CEM source, a secondary `stats` asset,
an imported public `card` template and explicit `kind` parameters. It keeps
the caller's `host`, the imported call's `card`, native metadata `document` and
the authored attribute separate. The missing attribute stays empty. Unicode
source values, exact output bytes, both exports, imported-module URI provenance
and byte-range bounds are checked. CLI sidecars retain their established
ownership. Common command outputs retain native inline provenance and actual
published-byte hashes.

No UI or runtime code changes. Entered credentials still block recovery until
explicit clearing, automatic whole-Studio recovery stays disabled, and the
existing 4 MiB checkpoint/group limits and test deadlines are unchanged. Browser
acceptance uses fixture-owned isolated contexts and preserves user data.

## Verification

- Native adapter integration: **one passing test**, three inputs rendered twice.
- Native CLI module conformance: **8/8 pass**, including the formerly failing
  original case, the shared graph variants, import cycles, depth/recursion,
  entrypoint/parameter refusals, component bundles and collection sidecars.
- Shared CEM-ML WASM package: **14/14 pass**, including the same graph variants
  on Node and browser bindings. The complete existing runtimes are exercised;
  no runtime source changed or optimizer rerun is needed for fixture corrections.
- Studio: **five selected worker cases pass**, including the new graph fixture,
  Login graph edits/provenance, CEM/XSLT entry artifacts and collection manifests.
  This is focused acceptance, not a rerun of the complete 276-case source cohort.
- Credential-refusal story passes. Package verification passes for the
  browser/Node ABI, **657 integrity records and 32 schema packages**.

Assembled demo acceptance passes: **898 files**, all owned hashes and all **333**
Studio files match the existing build, including paired runtimes and independent
factory memories. Selection, independent saved copies, HTML/CSS preview, offline
rebuild and worker scope pass. All **10** theme generators retain their CSS hashes;
**120** canonical playground URLs and the live tree gallery pass. Complete WASMs
remain 22.43/23.05 MiB, two copies each, below 25 MiB. Generated runtime assets,
screenshots and Vitest attachments are excluded from the commit.

The separate formatter/colorizer conformance discrepancy and historical
unattributed module-fetch recurrence watch remain in [todo.md](../todo.md).
