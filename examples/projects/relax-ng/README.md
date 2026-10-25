# RELAX NG source authoring

This ordinary project compares XML and compact RELAX NG source for a small note
contract. Open **Content types and schemas → Schema authoring**, clone the
read-only example, and use the shared editor and native command controls.

## Authored identity reference

These are resource identities, including expected artifacts. The configurations
below determine their supported uses. An identity alone does not grant an
operation.

| Resource role | Content type | Schema |
| --- | --- | --- |
| RELAX NG XML source | `application/relax-ng+xml` | `https://cem.dev/ns/data/relax-ng/1` |
| RELAX NG compact source | `application/relax-ng-compact-syntax` | `https://cem.dev/ns/data/relax-ng/1` |
| XML data | `application/xml` | `https://cem.dev/ns/data/xml/1` |

## Configurations

| Configuration | Purpose |
| --- | --- |
| `validate-xml` | Validate the typed RELAX NG XML vocabulary, start pattern and declaration syntax. |
| `parse-xml` | Inspect the typed XML source projection with original URI, namespace and UTF-8 ranges. |
| `format-xml` | Apply native lexical-lossless formatting and compare exact XML source bytes. |
| `validate-compact` | Check native compact lexical tokens, delimiters, namespace/start declarations and reference policy. |
| `parse-compact` | Inspect lossless typed compact tokens and their original byte ranges. |
| `format-compact` | Apply native lexical-lossless formatting and compare exact compact source bytes. |
| `validate-data-syntax` | Validate the example note as ordinary XML syntax only. |
| `repair-xml-start` | Restore the missing XML start/ref declaration. |
| `repair-compact-close` | Close the unclosed compact element pattern. |
| `repair-reference-policy` | Replace a rejected external reference with an inline pattern. |

Both syntaxes use `https://cem.dev/ns/data/relax-ng/1`. XML uses
`application/relax-ng+xml` and the structure namespace
`http://relaxng.org/ns/structure/1.0`; compact uses
`application/relax-ng-compact-syntax`. They are distinct native inputs. Generic XML
and CEM aliases do not replace their typed source contracts.

The XML source has a local `define` and `ref`; compact has corresponding local
name declarations. Their preservation demonstrates source syntax, not complete
reference resolution or grammar compilation. The compact parser is a lossless
lexical model with delimiter, string, start, namespace, keyword and reference
policy checks. It is not a complete RELAX NG grammar compiler. Neither syntax
provides general instance validation in this release.

`data/note.xml` uses ordinary XML syntax validation. It does not load the authored
RELAX NG schema and does not establish instance conformance. Source validation
and browser/editor presentation remain separate from any future instance engine.

## Formatting, failure and repair

All native formatter profiles currently preserve source lexemes and append a
final newline if absent. The tabular configurations compare
`expected/formatted-note.rng` and `expected/formatted-note.rnc`; they do not
promise reflow or cross-syntax conversion. The typed Parse views retain original
native source ranges. Generic Inspect is not advertised as a typed RNG view.
The Parse coordinates are presentation facts; common Parse currently publishes
no owned artifact-source references to Studio's Provenance selector. Native
repair diagnostics remain the separate source-selection path.

The missing XML start case reports `cem.relax_ng.missing_start` at buffer start.
Restore `<start><ref name="note"/></start>` after the grammar opening line.
The unclosed compact case reports `cem.relax_ng.compact_parse_error` at its
original opening brace (line 3, column 21, byte 56). Add the final closing brace.
Both repaired buffers exactly match their independently valid schema sources.

The external-reference failure reports `cem.relax_ng.external_ref_rejected`
(line 1, column 9, byte 8). Replace `start = external "remote.rnc"` with
`start = element note { text }`. Includes and external references remain
reject-only in both syntaxes: no network or filesystem schema resolver is called.
Enabling them requires a separate bounded resolver policy, not a sample-local
fallback.

Edit saved XML and compact schema element names independently, validate and format
them again. This tests source revisions without pretending to validate the example
XML instance. A second project clone can keep different declarations, repairs and
data. Offline reruns use the saved bytes and native adapters; read-only examples,
user projects and drafts remain unchanged. Credential privacy is preserved and
automatic whole-Studio recovery remains disabled.
