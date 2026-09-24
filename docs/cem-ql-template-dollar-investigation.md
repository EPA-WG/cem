# Compound `$name` reference investigation

Status: investigation completed 2026-09-24; language scope awaits a user decision.
This records evidence and a proposal, not an accepted syntax contract.

The data-slices audit found that sample 4's authored
`@value="{$s ?? $a}"` fails compilation. Both instances settle with blank inputs
and `cem.ql.render.compile_failed`; their attribute outputs correctly show the
default `😁` and supplied `🤗`. The user chose to investigate shared syntax
support instead of correcting that demo expression alone.

## Current contract and behavior

[CEM-QL AC-QS-6](cem-ql-ac.md#2-surface-syntax) and
[CEM-ML AC-T-7](cem-ml-ac.md) distinguish the `$` content-expression node from
query source. In `{$s ?? a}`, the first `$` belongs to CEM-ML: the parser receives
`s ?? a`. Inside an attribute span, it receives `$s ?? $a` unchanged.
CEM-QL's documented Rust-inspired surface uses bare names and deliberately
rejects XPath variable syntax, with a regression in
`packages/cem_ql/tests/parser_recovery.rs`.

Two copies of `normalize_host_expression`, in `render.rs` and `embedded.rs`,
currently strip a leading `$` only for a simple name or dotted reference.
Compound expressions bypass that compatibility handling.

| Surface | Current result, confirmed natively |
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

The recommended implementation scope is a shared template-expression parser
mode. It would accept `$name` only where a name reference is allowed, preserve
original query source and token ranges, and reuse existing name resolution,
type checking and lowering. Declaration names, parameters, record keys and
type names would remain bare. The CEM-ML `$` content-node marker would retain
its current meaning.

The renderer's prepared compilation path and both embedded-audit parsing and
standalone-expression compilation must use the same mode. Merely changing
`render.rs` would leave the audit inconsistent. The two current normalizers
must be reconciled with that shared boundary. Default public CEM-QL parsing
and compilation would continue rejecting `$name` under this proposal.

Raw `slice-value` expressions use the separate legacy JavaScript evaluator in
`cem-elements.ts`, including `$target.value`, `$event.type`, XPath-like event
aliases and limited arithmetic. This proposal covers expressions compiled by
the template compiler, including attribute-value spans; expanding raw event
expression evaluation is separate work.

## Decision and implementation checks

The pending decision is whether `$name` reference aliases belong only in
template expressions (recommended) or in all CEM-QL queries. Either choice
needs an explicit acceptance-criteria update; no production syntax has changed.

After the decision, add native regressions before implementation for compound
references across attribute spans, content nodes and whole-expression
attributes; renderer/audit agreement; literal/comment preservation; diagnostic
source positions; and rejection of prefixed declarations, parameters, keys,
types and malformed prefixes. Test the actual sample 4 template with default,
supplied, edited and empty values, then verify both browser instances and
resume the remaining data-slices behavior audit and gallery contracts.
