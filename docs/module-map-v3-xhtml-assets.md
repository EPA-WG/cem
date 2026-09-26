# Module-map v3 XHTML deployment resources

Status: native v3 XHTML resource support implemented; Site adoption and component
cutover remain pending.
The active checklist is in [todo.md](todo.md). This document specifies an
additive extension to the existing worker-safe
[module-map v3 contract](../packages/cem_ml/schema-packages/module-map-v3/v1/schema/module-map-v3.cem),
not a replacement for its typed imports or JavaScript edge rewriting.

## Problem and outcome

Canonical CEM components ship XHTML declarations. Removing `cem-action` from
the legacy JavaScript registry requires Site's search and interactive pages to
load its declaration explicitly. Site currently uses module-map v2, whose
resource vocabulary is JavaScript, CSS and WASM. A declared XHTML resource fails
with `cem.module_map.resource_type_unsupported`.

Extend v3 resources to deploy canonical `.xhtml` files through the same explicit
asset graph, digest evidence, cache inputs and atomic publication used by other
resources. Upgrade the affected Site maps to v3. Keep all v1/v2 acceptance and
rejection behavior unchanged, including v2's rejection of XHTML.

## Accepted resource contract

| Boundary | Contract |
| --- | --- |
| Schema identity | Existing `https://cem.dev/ns/data/module-map/3`. Update its schema-owned package, examples and documentation. |
| Entry kind | `resources` only. XHTML is not a JavaScript/JSON module import. |
| MIME and extension | `application/xhtml+xml` with `.xhtml`, using existing MIME-essence and extension-normalization rules. |
| Entry fields | Existing typed `path` and `contentType`; no new fields. |
| JavaScript edges | XHTML cannot declare nonempty `moduleImports` or serve as a JavaScript module-edge target. |
| Source/destination pair | Exact matching resource identities and content types; both maps declare v3. |
| Output path | Existing app-relative destination and containment rules, relative to each HTML export directory. |
| Browser import map | Only existing `imports` are projected. XHTML stays deployment-only. |
| Bytes | Copy the declared asset byte-for-byte. Do not rewrite embedded CEM-ML, CSS, comments, whitespace, IDs or URLs. |
| Discovery | Do not scan XHTML for scripts, links, imports or additional assets. Declare every dependency separately. |
| Evidence | Retain source/output digests, byte counts, output identity and cache dependency evidence. XHTML source/output digests match. |

This change authorizes XHTML only. It does not add `.html`, `.cemt`, XML data,
JSON resources, images, generic binary files, prefix mappings, discovery,
package-export selection, bundling, or fingerprinting. Those remain governed by
their existing contracts or require separate design.

Asset publication is a named deployment-copy boundary; it does not parse or
evaluate the XHTML. When a browser later loads the declaration, the existing
shared CEM import/render path owns parsing and diagnostics. Do not introduce a
JavaScript DOM parser, AST serialization, or application-specific template loader
into the build pipeline.

## Concrete source and destination

Source map excerpt:

```json
{
  "$schema": "https://cem.dev/ns/data/module-map/3",
  "imports": {},
  "resources": {
    "@epa-wg/cem-components/components/cem-action": {
      "path": "../../../packages/cem-components/src/components/cem-action/cem-action.xhtml",
      "contentType": "application/xhtml+xml"
    }
  }
}
```

Destination map excerpt:

```json
{
  "$schema": "https://cem.dev/ns/data/module-map/3",
  "imports": {},
  "resources": {
    "@epa-wg/cem-components/components/cem-action": {
      "path": "./assets/cem-components/cem-action.xhtml",
      "contentType": "application/xhtml+xml"
    }
  }
}
```

Maps identify the whole file. The consuming declaration chooses its named
fragment; `#cem-action` is not part of the copied asset's destination:

```html
<custom-element tag="cem-action"
  src="./assets/cem-components/cem-action.xhtml#cem-action"></custom-element>
```

Site currently installs the compatibility `custom-element` declaration tag.
The component demo uses the equivalent `cem-element` declaration. Both must
resolve the same canonical template without copying it into application code.

## Native acceptance sequence

1. Add failing native lowering fixtures for a v3 XHTML resource alongside an
   existing JavaScript module, CSS and WASM. Assert resource identity, MIME,
   bytes/digests, deterministic order, and absence from the browser import map.
2. Add rejection fixtures for v2 XHTML, XHTML in `imports`, MIME/extension
   mismatch, missing/mismatched pairs, unsafe destinations, XHTML module edges,
   missing files and unsupported resource types. Reuse existing diagnostic codes
   where their meaning already fits; do not weaken other type checks.
3. Make resource validation schema-aware. The shared validator is currently
   used by v2 and v3; adding XHTML to its unconditional allowlist would silently
   expand v2 and is incorrect. Preserve v3's existing JavaScript/JSON typed
   imports and exact declared-edge rewrite behavior.
4. Add CLI publication fixtures proving byte preservation, atomic failure with
   no partial writes, report evidence, and source-byte cache invalidation.
   Include embedded CEM-ML, XML escaping, Unicode and named template IDs.
5. Update schema-owned examples, manifest indexing, generated documentation and
   exported projections. Run focused native checks before rebuilding the CLI
   and downstream browser consumers. Publishing remains part of release work.

## Site adoption and component cutover

Convert Site's paired maps and authored import-map fixture shapes to v3 typed
entries. Audit each JavaScript asset's bare specifiers and declare the exact
`moduleImports` edges v3 requires; do not rely on v2's opaque-JavaScript behavior.
Retain the complete current resource inventory and add the canonical action
XHTML once per deployed route.

Update search/interactive CEM layouts to load the named template, update source
provenance checks to recognize canonical component declarations, and include
XHTML in build/cache inputs and deployed-resource verification. Prove both
routes from static output under their real paths, including action activation,
scoped styles, no missing asset requests, and deterministic output.

Only then restore and complete the action cutover: remove its registry entry
and global CSS, retain the five passing colocated action stories, migrate
consumer loading, and reduce the legacy count. The package trial also exposed
legacy suite failures recorded in the action migration plan; do not claim a
fully green package gate until that baseline is resolved.

The native implementation permits XHTML only on the v3 resource path. The
regression fixture first reproduced `cem.module_map.resource_type_unsupported`,
then passed with schema-aware validation. It covers XHTML alongside existing
JavaScript, CSS and WASM, MIME/extension normalization, byte/digest identity,
deterministic manifests, and the resource's absence from the browser import map.
Rejection coverage retains v1/v2 behavior and rejects mixed schemas, import-map
XHTML modules, unsupported types, unsafe destinations, mismatched pairs and
JavaScript edges to/from XHTML.

CLI coverage proves byte-preserving publication, deterministic reports, a changed
cache key when only XHTML changes, and no partial output for a missing XHTML
source. The schema-owned cache fixture now includes an XHTML resource.

The next task is Site adoption. Resolve the recorded token-browser import-limit
failure before claiming end-to-end Site verification, then complete the component
cutover. No publishing is included in this implementation.

## Verification

- `cargo test -p cem-ml module_map --lib`: 27 focused module-map tests passed.
- `cargo test -p cem-ml-cli transform_config_module_map --lib`: v1/v2/v3
  publication tests passed.
- `yarn nx run cem_ml_schema_package_module_map_v3_v1:verify`: package/schema
  examples, lowering/rejection fixtures, CLI publication, cache-key projection,
  TypeScript projections and native/WASM builds passed.
- `yarn nx run cem_ml_schema_package_module_map_v3_v1:samples2readme`: generated
  documentation updated from the manifest-owned examples.

The verification target selects its library tests and schema-example integration
test explicitly; it does not run workspace-wide tests.
