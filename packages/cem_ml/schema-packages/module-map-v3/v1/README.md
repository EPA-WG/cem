# Module Map v3 Schema Package

Status: schema-owned worker-safe module deployment contract.

This package owns `https://cem.dev/ns/data/module-map/3`. Paired maps declare
typed `imports` and `resources`. JavaScript entries may include `moduleImports`,
an exact mapping from an authored bare module specifier to another declared
asset identity. CEM-ML rewrites only those declared static import, export-from,
and quoted dynamic-import edges to relative deployed URLs.

JSON is supported as a browser import-map module target. CSS and WASM remain
explicit deployment resources. Relative and URL JavaScript specifiers remain
unchanged. Undeclared bare specifiers, unused rewrite declarations, computed
dynamic imports, dependency discovery, package-export selection, and undeclared
file copying are outside the contract.

## Verification

Run:

```bash
yarn nx run cem_ml_schema_package_module_map_v3_v1:verify
```

The cached target validates the schema package, native lowering and rejection
fixtures, CLI publication, WASM/type projections, and the resolved module-asset
cache key.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>source-worker-safe-modules</summary>

- Source: [`examples/source.module-map.json`](./examples/source.module-map.json)
- Content type: `application/vnd.cem.module-map+json`
- Schema: `https://cem.dev/ns/data/module-map/3`
- Expected result: `pass`
- README rendering: fenced `json` source

</details>

```json
{
    "$schema": "https://cem.dev/ns/data/module-map/3",
    "imports": {
        "@example/app": {
            "path": "../runtime/app.js",
            "contentType": "text/javascript",
            "moduleImports": {
                "@example/runtime-metadata": "@example/runtime-metadata"
            }
        },
        "@example/runtime-metadata": {
            "path": "../runtime/runtime.json",
            "contentType": "application/json"
        }
    },
    "resources": {
        "@example/worker": {
            "path": "../runtime/worker.js",
            "contentType": "text/javascript",
            "moduleImports": {
                "@example/app": "@example/app"
            }
        }
    }
}
```

<details>
<summary>destination-worker-safe-modules</summary>

- Source: [`examples/destination.module-map.json`](./examples/destination.module-map.json)
- Content type: `application/vnd.cem.module-map+json`
- Schema: `https://cem.dev/ns/data/module-map/3`
- Expected result: `pass`
- README rendering: fenced `json` source

</details>

```json
{
    "$schema": "https://cem.dev/ns/data/module-map/3",
    "imports": {
        "@example/app": {
            "path": "./assets/app.js",
            "contentType": "text/javascript",
            "moduleImports": {
                "@example/runtime-metadata": "@example/runtime-metadata"
            }
        },
        "@example/runtime-metadata": {
            "path": "./assets/runtime.json",
            "contentType": "application/json"
        }
    },
    "resources": {
        "@example/worker": {
            "path": "./workers/worker.js",
            "contentType": "text/javascript",
            "moduleImports": {
                "@example/app": "@example/app"
            }
        }
    }
}
```
