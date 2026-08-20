# Module Map v2 Schema Package

Status: schema-owned explicit web-resource deployment contract.

This package owns `https://cem.dev/ns/data/module-map/2` as an explicit-schema
alias of `application/vnd.cem.module-map+json`. Version 1 remains the primary
content-type owner and retains its JavaScript-only behavior. A v2 source and
destination pair keeps the same `imports` contract and adds `resources`:

```json
{
    "$schema": "https://cem.dev/ns/data/module-map/2",
    "imports": {
        "@scope/package": "../node_modules/@scope/package/index.js"
    },
    "resources": {
        "@scope/package/worker": {
            "path": "../node_modules/@scope/package/worker.js",
            "contentType": "text/javascript"
        },
        "@scope/package/styles": {
            "path": "../node_modules/@scope/package/styles.css",
            "contentType": "text/css"
        },
        "@scope/package/wasm": {
            "path": "../node_modules/@scope/package/runtime.wasm",
            "contentType": "application/wasm"
        }
    }
}
```

The destination document declares the same keys and content types, replacing
each path with an app-relative `./` URL. `imports` entries alone become the
browser import map. `resources` entries are deployment-only logical identities.

Supported resource types are intentionally bounded:

- `text/javascript` with `.js` or `.mjs`;
- `text/css` with `.css`;
- `application/wasm` with `.wasm`.

All declared assets lower to opaque typed graph imports and byte-preserving
exports beside every HTML destination. CEM-ML does not parse JavaScript, scan CSS,
traverse imports, select npm exports, or copy undeclared siblings. The shared
module-asset manifest records every import and resource with its resolved source,
destination, content type, byte length, SHA-256, and host-neutral aggregate cache
key. Existing v1-only manifests keep the same hash construction.

JSON resources, prefix mappings, inlining, fingerprinting, and dependency
discovery remain outside v2.

## Verification

Run:

```bash
yarn nx run cem_ml_schema_package_module_map_v2_v1:verify
```

The cached target validates `package.cem`, checks native lowering and negative
contracts, publishes text and invalid-UTF-8 binary fixture bytes through the CLI,
replays the v1 compatibility lane, builds the WASM/type-projection surfaces, and
uses the resolved manifest digest as an Nx runtime input.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>source-modules-and-resources</summary>

- Source: [`examples/source.module-map.json`](./examples/source.module-map.json)
- Content type: `application/vnd.cem.module-map+json`
- Schema: `https://cem.dev/ns/data/module-map/2`
- Expected result: `pass`
- README rendering: fenced `json` source

</details>

```json
{
    "$schema": "https://cem.dev/ns/data/module-map/2",
    "imports": {
        "@epa-wg/custom-element": "../../../../custom-element/dist/index.js"
    },
    "resources": {
        "@epa-wg/custom-element/runtime": {
            "path": "../../../../custom-element/dist/custom-element.js",
            "contentType": "text/javascript"
        },
        "@epa-wg/custom-element/theme": {
            "path": "../../../../cem-theme/dist/lib/css/cem-combined.css",
            "contentType": "text/css"
        },
        "@epa-wg/custom-element/cem-ql-wasm": {
            "path": "../../../../custom-element/dist/vendor/@epa-wg/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql_bg.wasm",
            "contentType": "application/wasm"
        }
    }
}
```

<details>
<summary>destination-modules-and-resources</summary>

- Source: [`examples/destination.module-map.json`](./examples/destination.module-map.json)
- Content type: `application/vnd.cem.module-map+json`
- Schema: `https://cem.dev/ns/data/module-map/2`
- Expected result: `pass`
- README rendering: fenced `json` source

</details>

```json
{
    "$schema": "https://cem.dev/ns/data/module-map/2",
    "imports": {
        "@epa-wg/custom-element": "./assets/custom-element/index.js"
    },
    "resources": {
        "@epa-wg/custom-element/runtime": {
            "path": "./assets/custom-element/custom-element.js",
            "contentType": "text/javascript"
        },
        "@epa-wg/custom-element/theme": {
            "path": "./assets/theme/cem-combined.css",
            "contentType": "text/css"
        },
        "@epa-wg/custom-element/cem-ql-wasm": {
            "path": "./assets/custom-element/vendor/@epa-wg/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql_bg.wasm",
            "contentType": "application/wasm"
        }
    }
}
```
