# CEM-QL order queries

Open **Examples → Content types and schemas → Queries and transformations →
CEM-QL order queries**, or find the same ordinary project under **Data**. Clone
it to edit independent saved resources. The project demonstrates filtering,
joining, typed results, original source ranges and native parameter maps; it
adds no application-specific parser or UI JavaScript.

## Authored identity reference

These are resource identities, including expected artifacts. The configurations
below determine their supported uses. An identity alone does not grant an
operation.

| Resource role | Content type | Schema |
| --- | --- | --- |
| Transformation graphs | `application/cem` | `https://cem.dev/ns/cli/transform-config/1` |
| Data and expected JSON artifacts | `application/json` | `https://cem.dev/ns/data/json/1` |
| CEM-QL modules | `application/vnd.cem.query+cem-ql` | `https://cem.dev/ns/query/cem-ql/1` |
| CEM-QL standalone expressions | `application/vnd.cem.query-expression+cem-ql` | `https://cem.dev/ns/query/cem-ql/1#expression` |
| CEM-native templates | `application/vnd.cem.template+cem` | `https://cem.dev/ns/template/cem-native/1` |

## Data and query identities

`data/orders.json` owns two customer records and four orders. Three orders have
status `ready`. One of those refers to `C-missing`, which intentionally has no
customer. That is valid JSON source, not an application-schema instance check.
The native queries decide whether an order participates in the inner join.

- `queries/ready-orders.cem-ql` declares
  `application/vnd.cem.query-expression+cem-ql` with schema
  `https://cem.dev/ns/query/cem-ql/1#expression`. It filters ready orders and
  returns three typed records containing `order`, integer `units` and integer
  `total` fields.
- `queries/customer-orders.cemql` declares
  `application/vnd.cem.query+cem-ql` with schema
  `https://cem.dev/ns/query/cem-ql/1`. Its explicit module URI, local functions
  and final expression inner-join ready orders totaling at least 30 to their
  matching customers. `O-101` joins to **Café Noord**, and `O-103` to **Harbor
  Books**. The unmatched order is excluded.
- `queries/ready-nodes.cem-ql` returns the three retained native order nodes,
  rather than constructing new records. Their source maps retain the original
  JSON URI and object ranges. The runtime-local opaque item identities are not
  portable resource addresses; the exported source URI/ranges describe the
  original data owner without granting a new navigation endpoint.

All queries consume the shared retained generic-data CEM tree. JSON result
export is a human-facing boundary, never a serialized AST fed back into query
execution. The computed-record fixtures preserve native record/atomic field
types rather than flattening integers or inventing data fields.

## Configurations

| Configuration | Typical use | Expected result |
| --- | --- | --- |
| `inspect-data` — Inspect retained order data | Review imported data structure and source positions | Native generic-data nodes and original JSON offsets |
| `inspect-expression` — Inspect standalone CEM-QL syntax | Review the typed expression AST | Original query URI and token/node ranges |
| `parse-expression-events` — Parse standalone CEM-QL events | Inspect lossless source events | Unicode comment and original byte positions |
| `validate-expression` — Validate the filter expression | Check input-bound expression syntax | Zero errors and warnings |
| `format-expression` — Format the standalone expression | Export canonical source layout | Exact native tabular fixture |
| `query-filter` — Filter ready order records | Select native data and construct typed fields | Three records matching `expected/filter-items.json` |
| `validate-module` — Validate the local query module | Check module identity/declarations | Zero errors and warnings |
| `format-module` — Format the query module | Preserve module/local-function source | Exact native tabular fixture |
| `query-join` — Join orders to customers | Combine order/customer records by key | Two typed records matching `expected/join-items.json` |
| `query-source-nodes` — Select native order nodes and source ranges | Inspect retained source ownership | Three original object spans from `expected/native-ranges.json` |
| `repair-expression` — Repair the missing CEM-QL closing brace | Diagnose and repair the same saved source | Two native parse errors, then zero after repair |
| `validate-parameter-template` — Validate the declared parameter model | Check declared parameters and expression slots | Zero errors and warnings |
| `report-ready` — Build ready orders from a parameter map | Apply status `ready`, minimum `30` | Two application JSON rows in `generated/ready-orders.json` |
| `report-draft` — Build draft orders from a parameter map | Apply status `draft`, minimum `0` | One application JSON row in `generated/draft-orders.json` |

## Parameter maps and result views

The two graph configurations bind the declared `status` and `minimum` parameters
of `templates/report.cemt`. The template constructs a native CEM-QL record,
`{status: status, minimum: num:integer(minimum)}`, and applies status, minimum and
customer-match filtering in separate variables. These bounded stages use the
existing native execution limits. The root record expression is parenthesized
inside its CEM-ML expression attribute to distinguish it from an attribute-value
template.

The native template creates generic-data result nodes, and the graph's JSON
writer publishes actual application records. It does not encode an internal AST
as transport data. These JSON arrays differ deliberately from the typed query
result envelope; Studio can display their output as a table using its shared
result/preview components. Each graph declares an explicit JSON entry artifact.

Select a graph resource and change its literal `minimum` parameter from `30` to
`45`, save and build. The ready report then contains only `O-101`. Changing the
status/minimum map changes selection without rewriting the query source. The
`query` command does not expose `--param`; parameter binding here belongs to the
existing template/graph API, with declarations that also validate independently.

Native graph invocation from this project directory:

```sh
cem-ml transform --config graphs/ready.cem \
  --config-content-type application/cem \
  --config-schema https://cem.dev/ns/cli/transform-config/1 \
  --scope-budget queueSize=1024
```

## Saved edits, source ranges and repair

Change **Café Noord** in the saved data, then rerun the module join and report.
The customer field changes; the input-only filter's selected order fields stay
the same. Changing the draft order's status to `ready` adds it to the standalone
filter, while the module's minimum-total rule still excludes its total of 12.
Unsaved drafts never replace persisted command inputs.

Native source-node selection preserves original JSON ranges. Computed records
do not claim arbitrary character-by-character lineage to their inputs. The
current generic-data JSON writer emits exact business output without character
source-map sidecars; the reports do not invent template or data mappings. Use
the source-node configuration and native source inspection for real original
data/query ranges. Synthetic formatting receives no invented source range.

Source inspection prints native coordinates but currently publishes no owned
artifact-source references through common Parse/Inspect. Exported node fields
describe original data ranges; Studio's separate Provenance selector follows
the query-source frames published by the command, including their admitted
query-buffer identity. Neither path interprets serialized AST fields as a new
source capability.

`failures/invalid-expression.cem-ql` omits the final block brace. Native source
validation reports two `cem.ql.parse_error` diagnostics. The primary missing-brace
diagnostic is at line 15, column 1, original byte offset 965; the parser also
retains its synchronization diagnostic. Insert the final `}` after `})`, save
and validate again. The repaired bytes match the valid standalone expression.

`failures/invalid-data.json` separately omits the colon after `orders` for native
graph refusal tests. It is outside normal graph inputs. Malformed input cannot
publish a replacement build; Studio retains the last successful artifacts.

## Limits and independent offline copies

Module execution supports local declarations and registered built-in helpers;
external query-library loading and ambient I/O are not enabled. A module header
URI is an identity, not a network request. Formatting uses the native
source-preserving tabular profile, rather than claiming an AST-aware beautifier.
All source/query identities remain explicit, and JSON source validation does not
assert order/customer business rules.

After runtime and project resources are cached, saved copies can reopen and run
offline. Data, query, template, parameter maps and drafts remain independent of
the catalog original and other copies. No example theme/storage key is introduced.
Entered passwords and one-time codes retain the existing recovery refusal, and
automatic whole-Studio recovery remains disabled.
