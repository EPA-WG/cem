# XPath and CEM-QL viewer function parity

The 2026-09-19 audit covers the shared capabilities added while implementing the
XSLT data viewer, and the three examples on the
[XPath functions page](../packages/cem-elements/demo/xpath-functions.html).
It compares native CEM-QL operations with XPath operations; calling an optional
XPath companion is not counted as a native CEM-QL implementation.

| XPath capability | Native CEM-QL / CEMT counterpart | Semantics and evidence |
| --- | --- | --- |
| `parse-xml`, `json-to-xml`, `import:parse-csv`, `import:parse-yaml` | `data:parse(source, format, options?)` | Added by this audit. Same CEM-ML string import profiles, duplicate/escape options, CSV headers and retained owners. `string_import_parity.rs`, `import_strings.rs`. |
| `base-uri`, `document-uri` | `data:base_uri(node)`, `data:document_uri(node)` | Added by this audit. Shared retained URI metadata, inherited `xml:base`, optional native nodes. String imports have no document URI. |
| Source `node-key`, `line-number` | `data:node_key`, `data:line_number` | Already shared; `source_provenance.rs` compares both languages on the same owner. Selection keys survive repeated imports; node identity does not. |
| `current-group`, `current-grouping-key` | `seq:group_by(...).items` and `.key`, explicitly passed to templates | No ambient group state in CEM-QL. The viewer groups by one string expanded-name key. CEM identity equality and zero-or-one atomic keys differ from XPath promotion and multiple-key grouping; neither contract is silently replaced. `sequence_collections.rs`, `xslt_grouping.rs`. |
| Stable sorting and population focus | `seq:sorted`, `cem:for-each`'s `position`, `seq:count` | Native finite-number/text sort keeps missing keys last. The stylesheet explicitly implements that policy. Multi-key sorting can be composed from stable passes; XSLT's typed comparison rules remain distinct. `xslt_data_view.rs` verifies the authored shared policy. |
| Host focus, function calls and closures | Explicit CEM-QL arguments/lambdas and CEMT template parameters | Native nodes remain native across calls. CEM-QL has explicit values rather than XPath's implicit focus; both share active call-depth and cumulative call budgets. `xpath_function_companion.rs`, `native_functions.rs`, `xslt_optional_focus.rs`. |
| QName catch clauses and buffered recovery | Query `try`/`catch`, CEMT `try`/`catch` and native diagnostic values | The shared renderer exposes error namespace/local name and preserves buffered native values. Native parsing adds typed import categories and nested parser diagnostics; limits/cancellation/unsupported capabilities cannot be caught. `error_recovery.rs`, `xslt_data_recovery.rs`, `string_import_parity.rs`. |
| Result nodes, atomics, attributes and spacing | Shared CEMT native result constructors | Already the same renderer and native ownership path. These are template instructions, not a missing query function. `native_result_construction.rs`, `xslt_output.rs`. |
| Node axes, names and values | Native `.children`, `.descendants`, `.attributes`, `.name`, `.namespace`, `.value` with sequence predicates | Generic fields preserve source nodes; XPath is a semantic node view. Explicit kind/namespace filters implement the viewer and demo selections. Text concatenation includes text/CDATA/whitespace and excludes comments. `data_import.rs`, `xpath_demo_pairs.rs`. |
| `normalize-space` and the viewer's whitespace-edge regex | `str:normalize_space(text, "xml")`, `str:trim(text, "xml")` | Added explicit XML whitespace profiles. NBSP remains data. Existing default Unicode normalization and ECMAScript trimming stay unchanged. `string_methods.rs` and a native viewer NBSP regression. |
| String concatenation, boolean predicates and existential tests | `str:concat`, equality, `seq:any` | The demo pairs share empty/Unicode strings, matching rules and numeric quantity tests. Numeric operands are explicitly converted to the same CEM numeric type. `xpath_demo_pairs.rs`. |

All missing viewer capabilities identified above are implemented. This is a
bounded use-case audit, not a claim that the two languages have interchangeable
type systems or that CEM-QL implements the entire XPath standard library.
For example, `str:replace` is literal replacement, not XPath regex replacement.
The viewer's regex use is whitespace trimming, covered by the explicit XML
profile. The XPath library's broader regex subset remains separately documented.

## Native string-import contract

`data:parse(source, format, options?)` is Tier B. Source is zero or one string;
empty sequence returns empty sequence. A present empty string is parsed and can
fail. The format is one string (`xml`, `json`, `yaml`, `csv`, or the documented
media types). Options are a CEM-QL record of scalar strings/booleans:

- JSON: `duplicates` = `retain` (default), `use-first`, or `reject`;
  `escape` = boolean (default false).
- CSV: `header` = `absent` (default) or `present`.
- All formats: `base-uri` = explicit string. It does not perform URI access.

The format and option resolver lives in `cem_ml::import`. Query code validates
value types, calls import, and returns a generic native CEM document. XML is not
an intermediate serialization step for JSON. No JavaScript document records,
format AST switches or document decoding enter the query evaluator.

Each invocation creates a new retained owner, like XPath string parsing. Source
keys remain deterministic for the same source, profile and URI metadata. The
provenance URI is `data:parse/source`; base URI is absent unless supplied. XPath
uses its expression's static source URI as its base. Supply the same explicit
base URI when comparing relative URI behavior across languages.

Malformed input raises `cem.ql.import_malformed` with expanded error name
`Q{urn:cem:import}invalid-source`; duplicate rejection raises
`cem.ql.import_duplicate_key` / `Q{urn:cem:import}duplicate-key`. Query call
locations and `importDiagnostics` retain parser diagnostic metadata. Invalid
format/options raise `cem.ql.import_options`; bad query argument types use the
ordinary query type error. Import ceilings, unsupported capabilities and host
cancellation stay fatal. Standard XPath error QNames remain its own public
contract; CEM-QL uses the native categories above.

The existing `data:read` / `cem-data` report and its 16-document LRU are unchanged.
They retain their stricter input profile and `cem` default projection. JSON
string parsing uses the standard XML-shaped projection, including its Unicode
and duplicate options. The 32 KiB, depth 64 and 4096-value/event import caps and
ordinary query/render budgets still apply.

## Demonstration and verification

The [paired demo](../packages/cem-elements/demo/xpath-functions.html) links to
detailed CEM-QL use cases for labels, predicates, native node selection, parsing,
metadata, grouping and sorting. Each pair has independent state and uses the
same authored input. XML quantity samples require numeric attribute values;
absence selects the low-stock rule.

Native tests run the authored sample templates before browser integration.
WASM bundle verification also executes the new native parsing, options, URI,
recovery and whitespace functions without JavaScript document conversion.
Browser stories cover edits, independent instances, focus/caret and malformed
XML recovery; the inventory verifier exercises standalone and source-loaded
documents. Completion counts are recorded in [todo.md](todo.md).
