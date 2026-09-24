# Compound `$name` reference investigation

Status: investigation completed 2026-09-24. The user accepted reference aliases
in **all CEM-QL queries**, with declaration names kept bare. The normative
contract is [AC-QS-7](cem-ql-ac.md#2-surface-syntax); this note records the
investigation and resulting implementation.

The data-slices audit found that sample 4's authored
`@value="{$s ?? $a}"` failed compilation. Both instances settled with blank inputs
and `cem.ql.render.compile_failed`; their attribute outputs correctly showed the
default `😁` and supplied `🤗`. The user chose to investigate shared syntax
support instead of correcting that demo expression alone.

## Behavior before the change

[CEM-QL AC-QS-6](cem-ql-ac.md#2-surface-syntax) and
[CEM-ML AC-T-7](cem-ml-ac.md) distinguish the `$` content-expression node from
query source. In `{$s ?? a}`, the first `$` belongs to CEM-ML: the parser receives
`s ?? a`. Inside an attribute span, it receives `$s ?? $a` unchanged.
CEM-QL's previous documented Rust-inspired surface used bare names and
rejected XPath variable syntax, with a regression in
`packages/cem_ql/tests/parser_recovery.rs`.

Two copies of `normalize_host_expression`, in `render.rs` and `embedded.rs`,
stripped a leading `$` only for a simple name or dotted reference.
Compound expressions bypassed that compatibility handling.

| Surface | Result before the change, confirmed natively |
| --- | --- |
| Attribute `@title="{$a}"` | Compiles; embedded audit normalizes `$a` to `a`. |
| Attribute `@title="{$s ?? $a}"` | Renderer and embedded audit retain both prefixes; compilation fails. |
| Content `{$s ?? a}` | Compiles; query source is `s ?? a`. |
| Content `{$s ?? $a}` | Fails; query source is `s ?? $a`. |
| `cem:for-each @select='$s ?? $a'` | Fails through the same renderer compilation boundary. |
| Attribute expression containing the string `"$s"` | Compiles and retains its literal dollar sign. |

An earlier native probe of the actual sample 4 template replaced only the
attribute expression in memory with `s ?? a`. It compiled without diagnostics
and rendered default, supplied, edited and explicitly empty input values
correctly. Slice fallback and empty-value semantics do not need to change.

## Feasibility and boundaries

A temporary lexer-based prototype replaced dollar tokens adjacent to identifier
tokens with spaces. Coalescing, dotted references, function arguments and local
binding values then parsed. String literals, line/block comments and Unicode
text remained byte-identical, and all original identifier token offsets were
retained. Malformed `$`, `$$a`, `$1` and `$ a` remained invalid.

Token replacement alone is too broad: it also made `declare let $name = 1`,
`{ let $name = 1; name }`, function parameters named `$arg`, and record keys
named `$key` parse. Supporting reference aliases requires parser context, not
a global string replacement or unconditional token filter. The three native
investigation tests passed; the temporary test and browser instrumentation
were removed after the observations were recorded.

The initial recommendation was a template-expression parser mode. The user
instead selected the same reference syntax for every CEM-QL entry point.
The expression parser now accepts an adjacent `$` before a name and produces
the same name expression used by bare references. The expression range includes
the prefix; the QName retains the identifier's original span. Resolution,
type checking and lowering are unchanged. Declaration names, parameters,
static record keys, member names and type names remain bare, including the
special `treat_as` type argument. The CEM-ML `$` content-node marker retains
its meaning.

Both host normalizers were removed. The renderer's prepared compilation path,
embedded audit, module API and standalone-expression API use the ordinary
parser. Embedded `normalized_source` retains the original reference spelling,
keeping query-local offsets aligned with the extracted host span.

Raw `slice-value` expressions use the separate legacy JavaScript evaluator in
`cem-elements.ts`, including `$target.value`, `$event.type`, XPath-like event
aliases and limited arithmetic. The accepted change covers all CEM-QL query
entry points, including template attribute-value spans; expanding raw event
expression evaluation is separate work.

## Implementation checks

Six new native regressions cover parsing and byte ranges; standalone evaluation,
literals, comments, binding errors and existing type rules; module declarations
and qualified references; malformed prefixes and forbidden name positions;
template/embedded-audit agreement and mapped diagnostic offsets; and the actual
sample 4 template with default, supplied, edited and empty values. The tests
first reproduced the rejection, then passed with the shared parser change.

The complete CEM-QL native suite passes 722 tests, with 9 existing ignored tests.
Browser verification and data-slices audit results are recorded in
`todo.md` and the demo coverage report.
