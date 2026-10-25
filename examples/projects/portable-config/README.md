# Portable configurations

This ordinary Studio project distinguishes authored CLI commands, run
configurations, transformation graphs and portable project manifests. Open
**Content types and schemas → Schema authoring** or **Queries and
transformations**, select **Portable configurations**, and clone before editing.
There is no dedicated example page or private command runner.

## Authored identity reference

These are resource identities, including expected artifacts. The configurations
below determine their supported uses. An identity alone does not grant an
operation.

| Resource role | Content type | Schema |
| --- | --- | --- |
| Transformation graphs | `application/cem` | `https://cem.dev/ns/cli/transform-config/1` |
| Run configuration control source | `application/json` | `https://cem.dev/ns/cli/run-config/1` |
| Authored CLI commands | `application/vnd.cem.cli-command+json` | `https://cem.dev/ns/cli/command/1` |
| CEM project control sources | `application/vnd.cem.studio-project+cem` | `https://cem.dev/ns/studio/project/1` |
| JSON project control sources | `application/vnd.cem.studio-project+json` | `https://cem.dev/ns/studio/project/1` |
| CEM-native templates | `application/vnd.cem.template+cem` | `https://cem.dev/ns/template/cem-native/1` |
| XML data | `application/xml` | `https://cem.dev/ns/data/xml/1` |
| HTML source and expected artifacts | `text/html` | `https://cem.dev/ns/data/html/1` |

## Resources and configurations

`data/card.xml` and `data/alternate.xml` are ordinary XML inputs. The CEM-native
`templates/card.cemt` declares its `heading` parameter. `graphs/card.cem` resolves
its data/template paths relative to the graph and publishes the explicit entry
`generated/index.html`. CLI command and run-config paths are relative to the
project working directory; they contain no absolute host paths.

`authored/build.command.json` stores literal argv for that graph.
`authored/check.command.json` stores argv for `validate --config`, and
`authored/check.run.json` chooses the actual XML input and its root scope. Command
schema v1, command-grammar schema v1 and common CLI version `0.1.0` are separate
compatibility fields. The run-config identity is
`https://cem.dev/ns/cli/run-config/1`, selected explicitly by the command flags.

`project/card.project.json` and `project/card.project.cem` describe the same
seven-resource project, with two configurations referencing those actual
command resources. They use Studio project schema v1 and preserve stable IDs,
relative paths, real SHA-256 values and the explicit HTML entry. The native codec
normalizes each resource's reference path. The manifest sources are illustrative
control resources inside this outer example; editing one does not silently
replace the open project's metadata.

| Configuration | Typical use |
| --- | --- |
| `validate-command` | Validate the authored command envelope, supported versions and literal argv shape. |
| `run-check` | Parse the actual run config through the native command path, inspect its normalized plan and validate its selected input. |
| `validate-project-json` | Validate project IDs, references, path/hash syntax and portable-state policy in JSON. |
| `validate-project-cem` | Validate the equivalent canonical CEM control source. |
| `inspect-project-cem` | Inspect generic imported CEM structure and original project-source positions. |
| `json-to-cem` | Use the native project owner to produce canonical CEM with exact expected bytes. |
| `cem-to-json` | Convert back to native normalized portable JSON, preserving logical references. |
| `format-project-cem` | Check the exact native tabular project writer. |
| `repair-command` | Restore the supported command envelope version. |
| `repair-project` | Restore the supported portable project version. |
| `build-card` | Build and preview the project-relative HTML graph. |

## Source validation, inspection and execution

Command source validation checks its envelope, versions, binary identity and
ordered literal argv. It does not validate argv against the CLI grammar. The
shared **CLI Command** inspector parses the command using the generated native
grammar, shows its literal argv and effective semantic plan, and checks it before
execution. The imported inner project's configurations use the authored command
files themselves, not an application-specific equivalent.

Generic Parse/Inspect/format of CLI command JSON currently reaches a generic CEM
projection/target gap. JSON project Parse/Inspect has the same source-projection
limit, so those configurations are not advertised. CEM project Inspect is a
structural CEM view, not a typed project Parse export. JSON/CEM project conversion
and CEM source formatting use the actual native Studio project codec.
The CEM inspection's printed coordinates are not owned artifact-source links;
common Inspect currently publishes no such references to the Provenance selector.
Use native diagnostics or the graph's separately published output spans when
following an available source-selection path.

Run config has a native parser/normalizer used by `validate --config`; its schema
identity does not install a generic standalone source adapter. The `run-check`
configuration therefore runs that actual command. Its input scope selects XML
syntax validation; it does not apply the CEM template or run the graph. Selecting
an identity alone grants no resource read, native adapter or execution authority.
The graph and run config intentionally have separate input settings.

## Failure and repair

The two forward-version resources each produce one original native error:
`cem.cli_command.schema_version_unsupported` and
`cem.studio_project.schema_version_unsupported`. Replace `"schemaVersion": 2`
with `"schemaVersion": 1`, then validate the saved source again. These control
errors currently have null text coordinates and include their field path; the
example preserves that native boundary instead of inventing character ranges.

`failures/duplicate.run.json` is a separate policy fixture. Running it through
`validate --config` refuses duplicate input URIs with fatal
`cem.run_config.input_uri_duplicate` and CLI exit 2 before document execution.
This is a preparation refusal, not an ordinary execution result: it is not
advertised as an executable repair configuration that Studio could open before
native command preparation succeeds. Restoring one input yields the valid run
configuration. Its standalone source operations remain unsupported.

Replacing `data/card.xml` with `failures/card.xml` makes the graph fail without
publishing a new output. Studio retains its last successful preview. Restore
valid XML and rebuild. HTML output spans identify original
`templates/card.cemt` ranges; computed text does not gain invented character-level
lineage back into XML data.

## Portability, copies and offline use

The canonical inner project can be decoded by Studio's native project codec and
imported with the seven declared resource byte buffers. Import checks the actual
bytes against their hashes. Manifest validation alone does not read files or
construct an archive. Its recorded hashes describe the original source files;
after changing those files, export the actual stored project to obtain current
portable metadata rather than assume this illustrative source auto-updates.

Change both `@id=dispatch-card` and `@root-uri=studio://dispatch-card/` to a new
logical namespace to import a second project with the same relative files. Or
use **Clone selected**, which assigns an independent project/root identity through
the existing Studio action. Graph and command references remain relative and
need no host-specific path rewrite.

The existing file-system provider's archive export/import API packages the
portable manifest and exact resource bytes. It excludes provider handles,
credentials, selections, open tabs and drafts. Create-mode import refuses a
duplicate project ID; explicit replacement requires the expected current project
revision. There is no native CLI archive/bundler command implied by this example.

Change **Dispatch board** to **Saved dispatch** in the graph, or change **Café
Noord** to **Café Haven** in the data, then rebuild. Change the run input from
`data/card.xml` to `data/alternate.xml`: the native validation report and command
inspector name that input, while the graph still builds its independently
configured card. The shared draft-navigation dialog can save a control source
without invoking an unsupported generic source-validation adapter; then run its
explicit configuration. Independent imported/archived copies and their relative
resources rebuild offline after reloading Studio.

No sample-specific UI JavaScript, parser, resolver, browser-side transform or
JSON AST execution handoff is introduced. Existing saved projects and drafts are
preserved. Entered passwords and one-time codes still block recovery until
explicitly cleared; automatic whole-Studio recovery remains disabled.
