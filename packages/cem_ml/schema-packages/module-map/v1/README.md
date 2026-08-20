# Module Map Schema Package

Status: schema-owned JavaScript deployment-map contract.

This package owns `application/vnd.cem.module-map+json` and
`https://cem.dev/ns/data/module-map/1`. A source map and destination map use the
same JSON shape:

```json
{
  "$schema": "https://cem.dev/ns/data/module-map/1",
  "imports": {
    "@scope/package/subpath": "../node_modules/@scope/package/dist/subpath.js"
  }
}
```

Source-map values resolve relative to the source map. Destination-map values
must be app-relative `./` JavaScript URLs and resolve beside each exported HTML
document. Both maps must declare exactly the same keys.

The `imports` object is the complete JavaScript dependency manifest. CEM-ML
loads each declared source as an opaque `text/javascript` graph artifact,
exports its bytes to the paired destination, and projects destination values
into the browser import map. It does not parse JavaScript, traverse imports,
select npm package exports, or copy undeclared files.

The first version accepts exact bare npm specifiers and `.js`/`.mjs` files only.
CSS, JSON, WASM, prefix mappings, inlining, and fingerprinting require later
schema versions or graph policy.

Every lowered graph exposes a deterministic module-asset manifest. Each record
contains the exact specifier, resolved source-map and source URIs, app-relative
target, resolved output destination, content type, byte length, and SHA-256 of
the bytes read. The manifest's aggregate SHA-256 is derived from ordered,
host-neutral specifier/target/content tuples, so the same build has the same
cache key in different checkout directories. `cem-ml transform --config ...
--module-asset-cache-key` prints only `sha256:<digest>` and stops before graph
execution or output publication; Nx consumes that projection as a runtime
input in `verify:nx-cache-input`.

## Verification

Run:

```bash
yarn nx run cem_ml_schema_package_module_map_v1:verify
```

The target validates the package manifest and exercises native graph lowering,
CLI byte-preserving copy/import-map projection, manifest reporting, and the
Nx runtime cache-key input.

## Examples

Source module map:

```json
{
  "$schema": "https://cem.dev/ns/data/module-map/1",
  "imports": {
    "@epa-wg/cem-ml/wasm": "../../node_modules/@epa-wg/cem-ml/dist/wasm/browser/cem_ml.js"
  }
}
```

Destination module map:

```json
{
  "$schema": "https://cem.dev/ns/data/module-map/1",
  "imports": {
    "@epa-wg/cem-ml/wasm": "./assets/cem_ml.js"
  }
}
```
