# JSON and YAML report

Use this ordinary project to inspect a saved API response, read a YAML report
configuration, filter records and publish the same result as HTML, JSON and YAML.
All service names and request counts are fictional. No service is contacted.

Open **Examples → Content types and schemas → Data → JSON and YAML report**.
The same project also appears under **Queries and transformations**. Create an
editable project copy before changing resources; the original remains read-only.
Each copy owns its saved resources, revisions, configurations and drafts.

## Resources and identities

| Resource | Native content type | Schema | Purpose |
| --- | --- | --- | --- |
| `data/payload.json` | `application/json` | `https://cem.dev/ns/data/json/1` | API response containing three service records. |
| `data/settings.yaml` | `application/yaml` | `https://cem.dev/ns/data/yaml/1` | Report title and exact status filter. |
| `queries/select-records.cem-ql` | `application/vnd.cem.query-expression+cem-ql` | `https://cem.dev/ns/query/cem-ql/1#expression` | Standalone expression selecting active record IDs. |
| `queries/inspect-settings.cem-ql` | `application/vnd.cem.query-expression+cem-ql` | `https://cem.dev/ns/query/cem-ql/1#expression` | Standalone expression reading configuration fields. |
| `templates/model.cemt`, `templates/view.cemt` | `application/vnd.cem.template+cem` | `https://cem.dev/ns/template/cem-native/1` | Shared native templates for the typed report model and HTML table. |
| `graphs/report.cem` | `application/cem` | `https://cem.dev/ns/cli/transform-config/1` | Explicit imports, named settings input, template stages and exports. |
| `expected/report.html` | `text/html` | `https://cem.dev/ns/data/html/1` | Reference bytes for the generated HTML artifact. |

The malformed source resources use the corresponding JSON/YAML identities.
`expected/` contains deterministic reports, query results and formatted source.
Portable configurations under `config/` are generated from `example.json` using
Studio's existing project pipeline. Validation configurations use the native run
contract; other configurations use the versioned native command contract.

## Configurations

| Configuration | Result |
| --- | --- |
| **Inspect imported JSON tree** (`inspect-json`) / **Inspect imported YAML tree** (`inspect-yaml`) | CEM-ML AST presentation of the imported generic-data tree, with original source start offsets. |
| **Validate JSON source** (`validate-json`) / **Validate YAML source** (`validate-yaml`) | Native source diagnostics; both valid inputs pass. |
| **Format JSON source** (`format-json`) / **Format YAML source** (`format-yaml`) | Native package formatting using the `tabular` profile, without overwriting saved source. |
| **Select active API record IDs** (`query-json`) | Explicit JSON query result containing `API-101` and `API-103`. |
| **Inspect report settings** (`query-yaml`) | Explicit JSON query result containing `title=Active service report` and `status=active`. |
| **Repair malformed JSON** (`invalid-json`) / **Repair malformed YAML** (`invalid-yaml`) | Intentional source failure, with the exact repairs below. |
| **Build and preview report** (`build-report`) | HTML report plus typed JSON and YAML arrays of the selected records. |

JSON and YAML import through the same retained native CEM generic-data boundary.
Queries and templates navigate that typed tree. The example contains no
JavaScript parser, object projection, serializer, filter or preview implementation.
The standalone JSON query deliberately selects the literal status `active`;
the report graph instead reads its status from the saved YAML configuration.

Tree inspection describes the imported CEM data projection, rather than a
lossless JSON/YAML language AST. Its current inspection envelope identifies that
CEM projection; the selected input identities are declared explicitly above.
Source validation is a separate operation. In particular, these configurations
check JSON/YAML syntax and native source contracts. They do not validate payload
instances against an authored JSON Schema, require business fields, or implement
a general application-configuration schema validator. Arbitrary JSON Schema
instance assertions are currently unavailable.
Generic JSON/YAML Parse source identity still has a known native gap, so these
configurations use the supported imported-tree Inspect path instead. Displayed
inspection coordinates do not populate owned artifact-source references in the
Provenance panel; the report graph's output maps are a separate native contract.

## Build and preview

Choose **Build and preview report** and run it. The graph reads both saved inputs,
passes settings as a named native input, and filters records by exact status while
preserving API record order. It constructs typed strings and numeric request
counts; package-owned native exporters produce JSON and YAML from that model.
The view template receives the same model and settings to generate an HTML table.

- `generated/report.html` is the explicit entry artifact and default report preview.
- `generated/report.json` and `generated/report.yaml` contain two records:
  `API-101` / Harbor weather / active / 120 and
  `API-103` / Museum hours / active / 42.
- Both data artifacts have a root array. The JSON artifact uses Studio's shared
  table preview; YAML remains available as source text. Select an output artifact
  to inspect its exact bytes.
- `expected/report.html`, `.json` and `.yaml` define the baseline publications.

Native CLI equivalent, run from the repository root:

```sh
cem-ml transform --config examples/projects/json-yaml-report/graphs/report.cem \
  --config-content-type application/cem \
  --config-schema https://cem.dev/ns/cli/transform-config/1 \
  --scope-budget queueSize=1024
```

## Saved edits

In an editable copy, change `status: active` to `status: paused` in
`data/settings.yaml`, and change its title to `Paused service report`. Save and
validate, then rerun **Build and preview report**. All three artifacts contain
only `API-102` / Canal alerts / paused / 7, and the HTML heading uses the new title.
An unmatched status produces an empty array and an empty table body; it is not
reported as a syntax error. Reset both settings to restore the baseline.

An unsaved draft is not a saved build input. Studio's normal save/discard/cancel
guard applies when switching resources or configurations. Saved edits survive
offline reopening; a second project copy keeps its own settings and outputs.
All inputs, templates, queries, expected results and configurations are owned by
the project archive, so rebuilding does not require a remote API or asset fetch.

## Failure and repair

The deliberately malformed resources are teaching inputs, separate from the
valid report inputs. A failed validation produces diagnostics, rather than a
new report artifact.

1. Run **Repair malformed JSON** against `data/payload-invalid.json`. The
   `cem.json.parse_error` diagnostic identifies the missing colon after the
   `records` member name. Replace `"records" [` with `"records": [`, save and
   rerun that same configuration: validation succeeds.
2. Run **Repair malformed YAML** against `data/settings-invalid.yaml`. The
   `cem.yaml.parse_error` diagnostic identifies an unclosed flow sequence.
   Replace `status: [active` with `status: active`, save and rerun: validation
   succeeds.

To see a build stop on the same failure, make the corresponding syntax change in
its valid report input and save. **Build and preview report** fails at native
import and publishes no new successful result. Repair and save that input, then
rerun to publish the report again. Files left by an earlier successful native
CLI run remain earlier outputs; they are not results of the failed run.

## Graph and source provenance

The shared graph view shows JSON/YAML imports, the named `config` dependency,
model/view stages and all three exports. Build input identities include both
saved data resources and the templates, independently of the current editor
selection.

The HTML output source map links generated ranges to authored instructions in
`templates/view.cemt`. Follow the explanatory sentence's provenance to that
resource, change it, save and rebuild. The graph explains the inputs and filtering
rule; the current source map does not promise a character-level trace from every
HTML cell to an original JSON or YAML field. JSON/YAML formatter output and HTML
template source ranges are separate boundaries.

This project adds no recovery policy or credential storage. Existing Studio
credential privacy remains in force: recovery stays blocked while a live Login
view contains a password or one-time code, until the user clears it.
Automatic whole-Studio recovery remains disabled.
