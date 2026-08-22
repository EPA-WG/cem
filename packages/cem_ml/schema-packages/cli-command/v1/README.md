# CLI Command Schema Package

Status: authored-command persistence contract for CEM-ML hosts.

This package fixes CLI command v1 to schema identity
`https://cem.dev/ns/cli/command/1` and content type
`application/vnd.cem.cli-command+json`. It stores the generated command-schema
version, compatible common CLI version, binary identity, and ordered literal
argv. The binary name is not duplicated inside argv.

The resource is authored configuration. Lowered requests, effective
configuration, resolver snapshots, execution results, and normalized run plans
are deliberately excluded. Hosts validate argv through the same generated
native CLI grammar before applying or executing a resource.

## Verification

Run:

```bash
yarn nx run cem_ml_schema_package_cli_command_v1:verify
```

The target validates the package, native parser and deterministic projection,
registry ownership, rejection fixtures, and Node/browser grammar parity.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>parse-ast</summary>

- Source: [`examples/parse-ast.command.json`](./examples/parse-ast.command.json)
- Content type: `application/vnd.cem.cli-command+json`
- Schema: `https://cem.dev/ns/cli/command/1`
- Expected result: `pass`
- README rendering: fenced `json` source

</details>

```json
{
  "$schema": "https://cem.dev/ns/cli/command/1",
  "schemaVersion": 1,
  "commandSchemaVersion": 1,
  "commonVersion": "0.1.0",
  "binaryName": "cem-ml",
  "argv": [
    "parse",
    "studio://feature-tour/data/cem-ml/basic.cem",
    "--content-type",
    "application/cem",
    "--schema",
    "https://cem.dev/ns/cem-ml/1",
    "--format",
    "ast",
    "--no-color"
  ]
}
```

<details>
<summary>inspect-source-offsets</summary>

- Source: [`examples/inspect-source-offsets.command.json`](./examples/inspect-source-offsets.command.json)
- Content type: `application/vnd.cem.cli-command+json`
- Schema: `https://cem.dev/ns/cli/command/1`
- Expected result: `pass`
- README rendering: fenced `json` source

</details>

```json
{
  "$schema": "https://cem.dev/ns/cli/command/1",
  "schemaVersion": 1,
  "commandSchemaVersion": 1,
  "commonVersion": "0.1.0",
  "binaryName": "cem-ml",
  "argv": [
    "inspect",
    "studio://feature-tour/data/cem ml/basic.cem",
    "--show",
    "source-offsets",
    "--format",
    "cem"
  ]
}
```

<details>
<summary>invalid-forward-version</summary>

- Source: [`examples/invalid-forward-version.command.json`](./examples/invalid-forward-version.command.json)
- Content type: `application/vnd.cem.cli-command+json`
- Schema: `https://cem.dev/ns/cli/command/1`
- Expected result: `fail`
- Expected diagnostics: `cem.cli_command.schema_version_unsupported`
- README rendering: fenced `json` source

</details>

```json
{
  "$schema": "https://cem.dev/ns/cli/command/1",
  "schemaVersion": 2,
  "commandSchemaVersion": 1,
  "commonVersion": "0.1.0",
  "binaryName": "cem-ml",
  "argv": ["parse", "input.cem"]
}
```

<details>
<summary>invalid-binary</summary>

- Source: [`examples/invalid-binary.command.json`](./examples/invalid-binary.command.json)
- Content type: `application/vnd.cem.cli-command+json`
- Schema: `https://cem.dev/ns/cli/command/1`
- Expected result: `fail`
- Expected diagnostics: `cem.cli_command.binary_name_invalid`
- README rendering: fenced `json` source

</details>

```json
{
  "$schema": "https://cem.dev/ns/cli/command/1",
  "schemaVersion": 1,
  "commandSchemaVersion": 1,
  "commonVersion": "0.1.0",
  "binaryName": "other-cli",
  "argv": ["parse", "input.cem"]
}
```

<details>
<summary>invalid-command-schema-version</summary>

- Source: [`examples/invalid-command-schema-version.command.json`](./examples/invalid-command-schema-version.command.json)
- Content type: `application/vnd.cem.cli-command+json`
- Schema: `https://cem.dev/ns/cli/command/1`
- Expected result: `fail`
- Expected diagnostics: `cem.cli_command.command_schema_version_unsupported`
- README rendering: fenced `json` source

</details>

```json
{
  "$schema": "https://cem.dev/ns/cli/command/1",
  "schemaVersion": 1,
  "commandSchemaVersion": 2,
  "commonVersion": "0.1.0",
  "binaryName": "cem-ml",
  "argv": ["parse", "input.cem"]
}
```

<details>
<summary>invalid-common-version</summary>

- Source: [`examples/invalid-common-version.command.json`](./examples/invalid-common-version.command.json)
- Content type: `application/vnd.cem.cli-command+json`
- Schema: `https://cem.dev/ns/cli/command/1`
- Expected result: `fail`
- Expected diagnostics: `cem.cli_command.common_version_invalid`
- README rendering: fenced `json` source

</details>

```json
{
  "$schema": "https://cem.dev/ns/cli/command/1",
  "schemaVersion": 1,
  "commandSchemaVersion": 1,
  "commonVersion": "0.1.0-01",
  "binaryName": "cem-ml",
  "argv": ["parse", "input.cem"]
}
```

<details>
<summary>invalid-control</summary>

- Source: [`examples/invalid-control.command.json`](./examples/invalid-control.command.json)
- Content type: `application/vnd.cem.cli-command+json`
- Schema: `https://cem.dev/ns/cli/command/1`
- Expected result: `fail`
- Expected diagnostics: `cem.cli_command.argument_control`
- README rendering: fenced `json` source

</details>

```json
{
  "$schema": "https://cem.dev/ns/cli/command/1",
  "schemaVersion": 1,
  "commandSchemaVersion": 1,
  "commonVersion": "0.1.0",
  "binaryName": "cem-ml",
  "argv": ["parse", "unsafe\u0000input.cem"]
}
```
