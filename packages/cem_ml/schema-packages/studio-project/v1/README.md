# Studio Project Schema Package

Status: fixture-proven portable project contract for CEM Studio.

This package fixes Studio project v1 to the schema identity
`https://cem.dev/ns/studio/project/1`. The canonical editable projection uses
`application/vnd.cem.studio-project+cem`; the equivalent JSON projection uses
`application/vnd.cem.studio-project+json`. The JSON Schema artifact is
`https://cem.dev/schema/studio/project.schema.json`.

Both projections normalize into the same native Rust model. They preserve
stable project, hierarchy, entry, and resource identities; deterministic
serialization; project-contained resource paths; and logical
`studio://{project-id}/...` resource URIs. Forward schema versions, unresolved
references, duplicate IDs, path escapes, invalid hashes, and unknown fields are
rejected with stable `cem.studio_project.*` diagnostic codes.

The portable manifest deliberately excludes provider bindings, absolute host
paths, credentials, browser file handles, open tabs, selections, and other
transient UI state. Those belong to host/provider storage, keyed by stable
project and resource identities.

## Verification

Run:

```bash
yarn nx run cem_ml_schema_package_studio_project_v1:verify
```

The target validates the schema package, checks built-in registry and JSON
Schema artifact ownership, and exercises CEM/JSON normalization, deterministic
round trips, logical URI derivation, and rejection fixtures.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>feature-tour-cem</summary>

- Source: [`examples/feature-tour.project.cem`](./examples/feature-tour.project.cem)
- Content type: `application/vnd.cem.studio-project+cem`
- Schema: `https://cem.dev/ns/studio/project/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/studio-project/v1/examples/feature-tour.project.cem,contentType=application/vnd.cem.studio-project+cem,schema=https://cem.dev/ns/studio/project/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns studio = "https://cem.dev/ns/studio/project/1"
@default studio

{project
    @schema-version=1
    @id="feature-tour"
    @name="CEM-ML Feature Tour"
    @description="Editable local-first Studio seed"
    @root-uri="studio://feature-tour/"
    @revision=1
    @created-at="2026-08-20T00:00:00Z"
    @updated-at="2026-08-20T00:00:00Z" |
    {entry
        @id="start-here"
        @kind="subproject"
        @name="00 Start Here"
        @description="First browser-capable workflows"
        @tags="tour getting-started"
    }
    {entry
        @id="validate-source"
        @parent-id="start-here"
        @kind="validation"
        @name="Validate and navigate diagnostics"
        @run-config-resource-id="validate-config"
        @resource-ids="tour-source validate-config"
        @tags="tour validation"
    }
    {resource
        @id="tour-source"
        @role="data"
        @source-kind="project-file"
        @path="data/tour.cem"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @revision=1
        @sha256="0000000000000000000000000000000000000000000000000000000000000000"
    }
    {resource
        @id="validate-config"
        @role="run-config"
        @source-kind="project-file"
        @path="config/validate.json"
        @content-type="application/json"
        @schema="https://cem.dev/ns/cli/run-config/1"
        @revision=1
        @sha256="1111111111111111111111111111111111111111111111111111111111111111"
    }
}
```

<details>
<summary>feature-tour-json</summary>

- Source: [`examples/feature-tour.project.json`](./examples/feature-tour.project.json)
- Content type: `application/vnd.cem.studio-project+json`
- Schema: `https://cem.dev/ns/studio/project/1`
- Expected result: `pass`
- README rendering: fenced `json` source

</details>

```json
{
    "$schema": "https://cem.dev/ns/studio/project/1",
    "schemaVersion": 1,
    "id": "feature-tour",
    "name": "CEM-ML Feature Tour",
    "description": "Editable local-first Studio seed",
    "rootUri": "studio://feature-tour/",
    "revision": 1,
    "createdAt": "2026-08-20T00:00:00Z",
    "updatedAt": "2026-08-20T00:00:00Z",
    "entries": [
        {
            "id": "start-here",
            "kind": "subproject",
            "name": "00 Start Here",
            "description": "First browser-capable workflows",
            "tags": ["tour", "getting-started"]
        },
        {
            "id": "validate-source",
            "parentId": "start-here",
            "kind": "validation",
            "name": "Validate and navigate diagnostics",
            "runConfigResourceId": "validate-config",
            "resourceIds": ["tour-source", "validate-config"],
            "tags": ["tour", "validation"]
        }
    ],
    "resources": [
        {
            "id": "tour-source",
            "role": "data",
            "sourceKind": "project-file",
            "path": "data/tour.cem",
            "contentType": "application/cem",
            "schema": "https://cem.dev/ns/cem-ml/1",
            "revision": 1,
            "sha256": "0000000000000000000000000000000000000000000000000000000000000000"
        },
        {
            "id": "validate-config",
            "role": "run-config",
            "sourceKind": "project-file",
            "path": "config/validate.json",
            "contentType": "application/json",
            "schema": "https://cem.dev/ns/cli/run-config/1",
            "revision": 1,
            "sha256": "1111111111111111111111111111111111111111111111111111111111111111"
        }
    ]
}
```

<details>
<summary>invalid-forward-version</summary>

- Source: [`examples/invalid-forward-version.project.json`](./examples/invalid-forward-version.project.json)
- Content type: `application/vnd.cem.studio-project+json`
- Schema: `https://cem.dev/ns/studio/project/1`
- Expected result: `fail`
- Expected diagnostics: `cem.studio_project.schema_version_unsupported`
- README rendering: fenced `json` source

</details>

```json
{
    "$schema": "https://cem.dev/ns/studio/project/1",
    "schemaVersion": 2,
    "id": "future-project",
    "name": "Future project",
    "rootUri": "studio://future-project/",
    "revision": 1,
    "createdAt": "2026-08-20T00:00:00Z",
    "updatedAt": "2026-08-20T00:00:00Z",
    "entries": [],
    "resources": []
}
```

<details>
<summary>invalid-escaping-path</summary>

- Source: [`examples/invalid-escaping-path.project.json`](./examples/invalid-escaping-path.project.json)
- Content type: `application/vnd.cem.studio-project+json`
- Schema: `https://cem.dev/ns/studio/project/1`
- Expected result: `fail`
- Expected diagnostics: `cem.studio_project.resource_path_invalid`
- README rendering: fenced `json` source

</details>

```json
{
    "$schema": "https://cem.dev/ns/studio/project/1",
    "schemaVersion": 1,
    "id": "escaping-project",
    "name": "Escaping project",
    "rootUri": "studio://escaping-project/",
    "revision": 1,
    "createdAt": "2026-08-20T00:00:00Z",
    "updatedAt": "2026-08-20T00:00:00Z",
    "entries": [],
    "resources": [
        {
            "id": "outside",
            "role": "data",
            "sourceKind": "project-file",
            "path": "../outside.cem",
            "contentType": "application/cem",
            "revision": 1,
            "sha256": "2222222222222222222222222222222222222222222222222222222222222222"
        }
    ]
}
```

<details>
<summary>invalid-forbidden-state</summary>

- Source: [`examples/invalid-forbidden-state.project.json`](./examples/invalid-forbidden-state.project.json)
- Content type: `application/vnd.cem.studio-project+json`
- Schema: `https://cem.dev/ns/studio/project/1`
- Expected result: `fail`
- Expected diagnostics: `cem.studio_project.invalid_json`
- README rendering: fenced `json` source

</details>

```json
{
    "$schema": "https://cem.dev/ns/studio/project/1",
    "schemaVersion": 1,
    "id": "stateful-project",
    "name": "Stateful project",
    "rootUri": "studio://stateful-project/",
    "revision": 1,
    "createdAt": "2026-08-20T00:00:00Z",
    "updatedAt": "2026-08-20T00:00:00Z",
    "entries": [],
    "resources": [],
    "providerBinding": {
        "absolutePath": "/Users/example/project"
    }
}
```

<details>
<summary>invalid-duplicate-id</summary>

- Source: [`examples/invalid-duplicate-id.project.cem`](./examples/invalid-duplicate-id.project.cem)
- Content type: `application/vnd.cem.studio-project+cem`
- Schema: `https://cem.dev/ns/studio/project/1`
- Expected result: `fail`
- Expected diagnostics: `cem.studio_project.id_duplicate`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/studio-project/v1/examples/invalid-duplicate-id.project.cem,contentType=application/vnd.cem.studio-project+cem,schema=https://cem.dev/ns/studio/project/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns studio = "https://cem.dev/ns/studio/project/1"
@default studio

{project
    @schema-version=1
    @id="duplicate-project"
    @name="Duplicate project"
    @root-uri="studio://duplicate-project/"
    @revision=1
    @created-at="2026-08-20T00:00:00Z"
    @updated-at="2026-08-20T00:00:00Z" |
    {entry @id="duplicate" @kind="subproject" @name="Group"}
    {resource
        @id="duplicate"
        @role="data"
        @source-kind="project-file"
        @path="data/input.cem"
        @content-type="application/cem"
        @revision=1
        @sha256="3333333333333333333333333333333333333333333333333333333333333333"
    }
}
```
