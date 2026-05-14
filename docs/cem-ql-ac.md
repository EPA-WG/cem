# `cem-ql` — CEM-ML Query Language — Acceptance Criteria

> **Status: Primary decision driver for cem-ql.**
>
> This file is the acceptance-criteria source of truth for the CEM-ML Query
> Language (`cem-ql`). It is downstream of [`cem-ml-ac.md`](cem-ml-ac.md):
> the host AST, scope model, content-type set, schemas, source-map stacks,
> async stream APIs, scope-policy mechanism, and report tree are owned there.
> This document does not redefine those contracts; it constrains a query
> language that consumes them.
>
> This file **resolves** `cem-ml-ac.md` Open Question 9 ("CEM template/query
> syntax"). The cem-ql surface defined here is the normative replacement for
> the `ScopedQueryLanguage::CemScopedQuery` placeholder in
> `cem-ml-stack-design-impl.md §3.10`. Template-side embedding of cem-ql
> expressions is owned by the host AC at `cem-ml-ac.md` **AC-T-7** and uses
> XSLT 3.0-style `{ … }` AVTs and `select="…"`/`match="…"`/`test="…"`
> attributes; cem-ql itself does not define a template-embedding delimiter
> (see AC-QS-6).
>
> If a requirement here contradicts `cem-ml-ac.md`, `cem-ml-ac.md` wins until
> this document is aligned.

This document captures acceptance criteria for `cem-ql`. Each item is phrased
as a checkable statement. RFC 2119 keywords (MUST / SHOULD / MAY) and the
status legend below apply.

> **Status legend**
>
> - **MUST** — required for the language to be considered shippable.
> - **SHOULD** — required unless an explicit waiver is recorded in this file.
> - **MAY** — explicitly optional; in scope for a later release.
> - **OPEN** — needs a decision before AC can be tested.

## Goal

`cem-ql` is the in-process selector and transformation expression language
for CEM-ML. It is not a general-purpose programming language and it is not a
browser DOM language. Its single purpose is to read the cem-ml AST/DOM
projections (input DOM, CEM AST, machine-state slots, report AST, template
registries, source-map stacks) and produce streamed results for templates,
validators, transforms, plugins, CLI projections, and runtime hydration
rules.

This language is the normative resolution of `cem-ml-ac.md` Open Question 9
and replaces the `ScopedQueryLanguage::CemScopedQuery` placeholder in
`cem-ml-stack-design-impl.md §3.10`. Template-side embedding is defined by
`cem-ml-ac.md` AC-T-7 (XSLT 3.0-style AVTs and `select=`/`match=`/`test=`);
cem-ql defines no embedding delimiter of its own.

The long-range parity target is XPath 3.1 / XQuery 3.1 expression coverage,
XSLT-style user-defined functions and module composition, and Python-style
set/sequence operations. Parity is tiered.

## Conformance Tiers

Tiers mirror the host stack so a `cem-ml` Tier A release can ship a `cem-ql`
Tier A profile without leaving open contracts.

- **Tier A — Template selector and value-extractor MVP.** Enough of the
  language to satisfy `cem-ml-ac.md` AC-T-1 / AC-T-2: `descendants`,
  `children`, `parent`, `self`, name and namespace tests, attribute access,
  predicates, equality and comparison, lambdas, `.`-chained pipelines, the
  four set operators (`|`, `&`, `-`, `^`), variable binding, single-module
  function definition, schema-derived types as runtime guards, deterministic
  ordering. No async, no imports, no external documents.
- **Tier B — Multi-document, multi-content-type, async streaming.** Adds
  `read(uri, content-type)` against the cem-ml content-type transform set,
  external module imports gated by scope policy, async iteration, cancel via
  `AbortSignal`, the full XPath axis set, sequence comprehensions, grouping,
  and FLWOR-equivalent constructs.
- **Tier C — Full XSLT/XQuery surface.** Adds NVDL-style schema dispatch
  inside queries, regular-path operators, higher-order function library
  parity with XPath 3.1 + XPath 4.0 candidate functions per `qt4cg.org`,
  binary AST consumption, query-time hydration rule generation, and
  query-emitted DOM patch plans (Tier C in the host AC).

Each AC below is tagged `[A]`, `[B]`, or `[C]`.

---

## 0. Cross-Cutting Requirements

- **AC-Q-1 [A] MUST** consume only the AST projections defined by
  `cem-ml-ac.md` AC-P-1, AC-P-4, AC-V-4, AC-O-3, AC-O-4 (input DOM, CEM AST,
  schema frames, namespace context, report AST, source-map stacks). The
  language MUST NOT introduce a parallel DOM model.
- **AC-Q-2 [A] MUST** be **read-only** with respect to the host AST. Every
  expression returns a new stream/value; no expression mutates a host node,
  attribute, slot, registry entry, or scope. Mutation belongs to the
  Tier C async DOM mutation API in `cem-ml-ac.md §5`.
- **AC-Q-3 [A] MUST** evaluate inside a `QueryContextScope`
  (`cem-ml-stack-design-impl.md §3.10`) and reject any node, slot, or
  resource access outside that scope with `cem.ql.scope_violation`.
- **AC-Q-4 [A] MUST** be deterministic for a given input AST + scope +
  context. Iteration order, attribute order, set-operation results, and
  diagnostic ordering MUST be stable for snapshot use.
- **AC-Q-5 [A] MUST** carry source-map stacks on every expression,
  diagnostic, and emitted node so query origins resolve back to the query
  source per `cem-ml-ac.md` AC-P-7. Query source itself is a `SourceId`.
- **AC-Q-6 [A] MUST** expose a single async public evaluator API consistent
  with `cem-ml-ac.md` AC-A-1: `evaluate(query, context) -> Stream<Item>`. No
  blocking variant. Tier A may resolve synchronously for in-memory inputs;
  the surface is async.
- **AC-Q-7 [A] MUST** route diagnostics through the host report AST per
  `cem-ml-ac.md` AC-O-3. Query-time errors, type errors, undefined
  references, and policy violations attach to the active scope context
  frame and the originating expression's source-map stack.

---

## 1. Language Model

- **AC-QL-1 [A] MUST** model every expression result as a **stream of items**
  with deterministic order. A single value is a one-item stream; the empty
  result is a zero-item stream. There is no scalar/sequence distinction at
  the surface (XPath 3.1 sequence model).
- **AC-QL-2 [A] MUST** support these item kinds:
  - `node` — any host AST node (input DOM, CEM AST, report node);
  - `attribute` — host attribute occurrence by `ExpandedName`;
  - `text` — `scalar text` event content as a string;
  - scalar atoms: `string`, `integer`, `decimal`, `boolean`, `null`;
  - `record` — ordered map from string key to item-stream;
  - `array` — homogeneous or heterogeneous indexable item-stream that
    behaves as a single item when nested;
  - `lambda` — first-class function value;
  - `state-slot` — reference to a `MachineStateSlot` per
    `cem-ml-stack-design-impl.md §3.11`;
  - `resource` — opaque handle for unresolved external resources permitted
    by scope policy.
- **AC-QL-3 [A] MUST** treat strings, arrays of chars, arrays of numbers,
  and arrays of records as collections that compose with the same operators
  as node streams. A string is iterable as a stream of codepoints; an array
  is iterable as a stream of its items. This satisfies the project
  requirement that arrays of chars/numbers behave as collections.
- **AC-QL-4 [A] MUST** distinguish **streams** from **arrays** the same way
  XQuery distinguishes sequences from arrays: an array is one item that
  happens to contain other items; a stream auto-flattens at the expression
  boundary. Conversion functions: `array(stream)`, `stream(array)`.
- **AC-QL-5 [A] MUST** expose lambdas as values:
  `fn(arg, …) => expression`. Lambdas close over the lexical scope at
  definition time. They do **not** capture host AST identity for retention
  beyond the host's scope lifetime.
- **AC-QL-6 [A] MUST** define a tail position evaluation model so a long
  `.`-chain does not grow the stack proportional to chain length. Pipeline
  operators are stream-to-stream and lazy by default.

---

## 2. Surface Syntax

- **AC-QS-1 [A] MUST** support **dot-chained pipeline** form where the
  left-hand stream is the receiver and the right-hand call is applied to
  each item. Example:
  ```
  descendants(Component)
    .where(.name == "Button")
    .map({ name: .name, text: .descendants(Text).string() })
  ```
- **AC-QS-2 [A] MUST** allow a **leading dot** (`.field`, `.method()`) inside
  a pipeline step body to mean "the current item." Outside a pipeline step
  body, leading-dot is a syntax error. This matches the JQ-style projection
  the design discussion settled on without making `this` implicit globally.
- **AC-QS-3 [A] MUST** support **anonymous record literals** in the canonical
  XQuery 3.1 map form `{ "key": expression, … }`:
  - **Keys** are **string literals** (double-quoted), aligning with the
    XQuery 3.1 map constructor and avoiding ambiguity with XSLT-style
    `{ … }` AVT delimiters and `select="…"` attribute embedding used by
    CEM-ML templates per `cem-ml-ac.md` AC-T-7.
  - **Computed keys** use `[expression]` and evaluate to a string.
  - **Values** are cem-ql expressions — the same expression grammar that
    appears inside template `{ … }` AVTs and `select=` attributes.
  - Bare-identifier keys (`{ key: value }`, JSON-style) are **not**
    accepted in Tier A; the parser emits `cem.ql.parse_error` and suggests
    quoting. Tier C MAY reintroduce them as sugar if template embedding
    can be disambiguated without re-opening this contract.
- **AC-QS-4 [A] MUST** support **block expressions**
  `let name := expression in body` for local binding inside any expression.
  `let` cascades left-to-right; later bindings see earlier ones.
- **AC-QS-5 [A] MUST** support a **module declaration form** for top-level
  variable, function, and import statements:
  ```
  module urn:ex:my-module
  import "cem:stdlib/strings" as str       ;; platform stdlib, always available per AC-QI-2
  declare variable $TITLE := "hi"
  declare function local:greet($who) { "hello " || $who }
  ```
- **AC-QS-6 [A] MUST** define a stable lexical grammar that does not depend
  on host content type, host whitespace policy, or host trivia preservation
  policy. Query source is plain UTF-8 text and uses its own tokenizer; it
  is not embedded as XML/HTML attributes by default. Embedding inside CEM-ML
  template attributes is owned by the host AC at `cem-ml-ac.md` **AC-T-7**,
  which adopts XSLT 3.0-style syntax: `{ cem-ql-expression }` Attribute
  Value Templates with `{{` / `}}` escapes, and full-attribute
  `select="…"` / `match="…"` / `test="…"` forms. The cem-ql parser is
  invoked by the template compiler on the post-AVT-strip, post-XML-escape
  substring; cem-ql itself sees plain UTF-8.

---

## 3. DOM Access Surface

- **AC-QD-1 [A] MUST** expose XPath-equivalent axes over the input DOM and
  CEM AST projections. Tier A: `self`, `child`, `parent`, `descendants`,
  `descendants-or-self`, `ancestors`, `ancestors-or-self`, `following-sibling`,
  `preceding-sibling`, `attributes`. Tier B adds the remaining XPath axes
  (`following`, `preceding`, `namespace`).
- **AC-QD-2 [A] MUST** support **typed axis arguments**: `descendants(Button)`
  is equivalent to `descendants() .where(. is Button)` where `Button` is a
  schema-declared element type derived from the active schema's
  `StructuralSchemaIr` (see AC-QT-*). The argument is checked at query
  compile time; an unknown name is `cem.ql.unknown_type`.
- **AC-QD-3 [A] MUST** support attribute access by `QName` and by
  `ExpandedName` per `cem-ml-stack-design-impl.md §3.4.1`. Lexical name
  access on case-sensitive attributes returns the source spelling;
  `ExpandedName` access ignores prefix. HTML `data-*` attributes are
  reachable via the `cem:html-data` namespace.
- **AC-QD-4 [A] MUST** support reference resolution as a first-class step:
  `.target` on a node carrying an `id`/`for`/`aria-*` slot follows the
  resolved reference per `cem-ml-ac.md` AC-F-5 / AC-P-9 / AC-V-6. Unresolved
  slots emit `cem.ql.unresolved_reference` at the query's report scope; the
  effective scope policy decides severity (default warning, matching
  AC-V-6).
- **AC-QD-5 [A] MUST** preserve the host's source-map stack on every
  selected node and append a `TransformKind::Query` frame for every
  expression that produces a new value (record literal, sequence
  construction, computed atom).
- **AC-QD-6 [A] MUST** treat tainted recovered subtrees per `cem-ml-ac.md`
  AC-V-8 as visible-but-marked: queries see them by default; `where(.tainted)`
  / `where(not(.tainted))` filter accordingly. A scope policy MAY hide
  tainted subtrees from queries inside that scope.
- **AC-QD-7 [B] MUST** address machine-state slots and template-registry
  entries as first-class items (`state-slot`, `template-ref`) so transforms
  can write `state("user.email")` or `template("cem:button")`. Read access
  is gated by `QueryContextScope.state_slots` and the template-registry
  inheritance rules in `cem-ml-ac.md` AC-R-2.

---

## 4. XPath Functional Parity

- **AC-QX-0 [A] MUST** treat parity with XPath 3.1 / XQuery 3.1 as
  **functional, not syntactic**. cem-ql is expected to compute the same
  results over the same inputs for the subset listed in AC-QX-1 and the
  §4.1 matrix, but it is **not** required to mirror XPath/XQuery clause
  syntax where a simpler construct — typically a stdlib helper drawn
  from the Rust iterator / `itertools` ecosystem — covers the same
  functional surface. Where such a helper exists and the host AC has no
  contrary requirement, the helper form is canonical and the
  XPath/XQuery syntactic form is **not** added. Concrete instances:
  grouping uses `group_by()` (AC-QX-4, AC-QO-6) rather than a FLWOR
  `group by` clause; set arithmetic uses the four infix operators
  `| & - ^` (AC-QO-1) rather than the `union / intersect / except`
  keyword forms; collection helpers are named functions (AC-QO-6) rather
  than additional XPath-style operators. Decisions to omit XPath syntax
  in favor of a helper MUST be recorded inline at the relevant AC so the
  parity matrix remains testable.
- **AC-QX-1 [A] MUST** be **functionally equivalent to XPath 3.1** for the
  subset that operates over a tree-shaped node store: axes (per AC-QD-1),
  name and kind tests, predicates `[…]`, sequence construction, comparisons
  (`= != < <= > >=`), arithmetic (`+ - * div mod`), boolean (`and or not()`),
  conditional `if (…) then … else …`, `for $x in seq return …`, `let $x :=
  … return …`, quantified `some/every $x in seq satisfies …`, and the
  built-in function library subset listed in AC-QF-2.
- **AC-QX-2 [A] MUST** preserve XPath document-order semantics for axis
  results. Stream order matches host event-emit order from the cem-ml
  parser, which is document order.
- **AC-QX-3 [B] SHOULD** add XPath 3.1 maps and arrays (already covered by
  `record` and `array` in AC-QL-2; this item enforces the cast/round-trip
  rules).
- **AC-QX-4 [B] SHOULD** add **FLWOR**: `for / let / where / order by /
  return` with window clauses, semantically equivalent to XQuery 3.1
  *minus* the `group by` clause. **Functional parity with XPath/XQuery
  is not syntactic parity** (see AC-QX-0): grouping is provided by the
  `group_by(stream, .key)` stdlib helper (AC-QO-6) and its companions
  `count_by` / `partition`, which return `stream<record{key, items}>` and
  compose with the rest of FLWOR through ordinary `for`/`let` bindings.
  cem-ql does **not** ship the FLWOR `group by` clause at any tier; the
  Rust-ecosystem-style helper covers the same functional surface with a
  simpler construct, and AVT/`select=` attributes favor short pipelines
  over multi-clause FLWOR anyway. This **resolves** §15 Open Question 6.
- **AC-QX-5 [C] MAY** evaluate the XPath 4.0 candidate function library at
  `qt4cg.org` and adopt accepted functions; rejected items are explicitly
  waived.
- **AC-QX-6 [A] MUST NOT** import the XPath/XQuery `fn:doc()`,
  `fn:document()`, or `fn:collection()` functions in their host-fetching
  form. The cem-ql analogue is `read(uri, content-type)` per AC-QA-* and is
  off by default.

### 4.1 Parity Matrix (Informative Sketch)

| XPath 3.1 area                                        | cem-ql tier | Notes                                                                  |
|-------------------------------------------------------|-------------|------------------------------------------------------------------------|
| Forward axes (child, descendant, attribute, self, …)  | A           | Per AC-QD-1                                                            |
| Reverse axes (parent, ancestor, preceding, …)         | A/B         | parent/ancestor in A; preceding/following in B                         |
| Name tests, kind tests                                | A           | `*`, `prefix:*`, `*:local`, `Component`, `text()`, `comment()`         |
| Predicates                                            | A           | One predicate per step in A; positional `[1]` / `[last()]` in A        |
| Sequence operators `, \| except`                      | A           | Spelled `,`, `\|`, `-` (see AC-QO-1)                                   |
| Arithmetic                                            | A           | Integer, decimal, double; promotion rules per XPath                    |
| Comparisons                                           | A           | Value `eq ne lt gt`, general `= !=`                                    |
| `if/then/else`, `let`                                 | A           | Per AC-QS-4                                                            |
| `for…return`                                          | A           |                                                                        |
| `some/every…satisfies`                                | A           |                                                                        |
| FLWOR with `where/order by`                           | B           | No `group by` clause at any tier; grouping via `group_by()` per AC-QO-6 |
| Path expressions `step / step`                        | A           | `/` and `.` chains are interchangeable; `.` form is canonical          |
| Type expressions `instance of`, `cast as`, `treat as` | A           | Driven by schema-derived types                                         |
| Higher-order functions                                | A           | Lambdas; `function-call` first-class                                   |
| Maps and arrays                                       | A/B         | Records in A; XPath 3.1 array semantics in B                           |
| Try/catch                                             | B           |                                                                        |
| Regex (`fn:matches`, `fn:replace`)                    | B           |                                                                        |
| `fn:doc / fn:collection`                              | B (renamed) | `read(uri, accepts?)` per AC-QA-1; `accepts` omitted / `Accept`-header string / collection of canonical IDs (AC-QA-1.1) |

The full table will be tracked in `cem-ql-stack-design.md` once that document
is created; this matrix exists to make the parity contract testable.

---

## 5. Stream / Set Operations

- **AC-QO-1 [A] MUST** define exactly four binary infix set operators over
  streams of host items, semantically and notationally aligned with XPath
  node-set operators and Python set arithmetic:
  - `|` — **union** (XPath `|`; Python `a | b`). Removes duplicate items by
    item identity for nodes, by value equality for atoms.
  - `&` — **intersection** (XPath `intersect`; Python `a & b`).
  - `-` — **difference** (XPath `except`; Python `a - b`).
  - `^` — **symmetric difference** (Python `a ^ b`; not present in XPath).
- **AC-QO-2 [A] MUST** preserve **document order** in the result of every
  set operator when both operands are node streams. For atom streams, order
  follows the left operand, then any new items from the right operand in
  their source order. Duplicates inside one operand are deduplicated before
  combination. This makes set operators stable for snapshot tests.
- **AC-QO-3 [A] MUST** define identity for the operators:
  - node identity = host `AstNodeId` (stable for one parse);
  - attribute identity = `(AstNodeId, ExpandedName)` last-writer-wins per
    `cem-ml-stack-design-impl.md §3.4`;
  - record identity = structural deep equality of keys and values;
  - array identity = positional deep equality;
  - atom identity = XPath value equality.
- **AC-QO-4 [A] MUST** make set operators **streamed**: they MUST NOT
  materialize either operand fully unless the operator's semantics require
  it. `|` can stream both; `&`, `-`, `^` may buffer the right operand
  bounded by the host's scope-policy memory cap.
- **AC-QO-5 [A] MUST** map **boolean operators** to their classical XPath
  semantics: `and`, `or`, `not(…)`. The C-family `&&` and `||` are
  **reserved** and parse as syntax errors with `cem.ql.use_and_or` so
  authors are not misled into thinking they short-circuit set operators.
- **AC-QO-6 [B] SHOULD** expose Python-style collection helpers as named
  functions, not operators, so set/stream code reads consistently:
  - `union(a, b, …)`, `intersect(a, b, …)`, `difference(a, b)`,
    `symmetric_difference(a, b)`;
  - `unique(stream)`, `distinct_by(stream, .key)`;
  - `flatten(stream)`, `flat_map(stream, fn)`;
  - `zip(a, b, …)`, `enumerate(stream)`, `chunked(stream, n)`,
    `windowed(stream, n)`, `sliding(stream, n, step)`;
  - `group_by(stream, .key)`, `count_by(stream, .key)`,
    `partition(stream, fn)`;
  - `take(stream, n)`, `drop(stream, n)`, `take_while(stream, fn)`,
    `drop_while(stream, fn)`;
  - `sorted(stream, by: .key)`, `reversed(stream)`;
  - `reduce(stream, init, fn)`, `fold(stream, init, fn)`,
    `scan(stream, init, fn)`;
  - `any(stream, fn)`, `all(stream, fn)`, `none(stream, fn)`;
  - `min`, `max`, `sum`, `avg` with `by:` lambda parameter.
- **AC-QO-7 [B] SHOULD** support comprehension syntax sugar that desugars to
  the helpers above, e.g.
  `[ .name for c in descendants(Component) where .visible ]`. Desugaring
  rules MUST be one-to-one so authors can reason about cost.
- **AC-QO-8 [A] MUST** define **comparison rules across collection types**:
  comparing a string to an array of chars converts neither implicitly;
  authors call `string(…)` or `chars(…)` explicitly. Cross-collection
  equality is `false` by default and produces `cem.ql.cross_type_compare`
  warning when both operands are nonempty.

---

## 6. Pipeline Composition

- **AC-QP-1 [A] MUST** make `.` the canonical pipeline operator. `a.b` reads
  as "evaluate `a`, pass each item to `b`, concatenate." Path expressions
  `a/b` are accepted as a synonym for parity with XPath; the canonical form
  is `.`.
- **AC-QP-2 [A] MUST** allow lambdas as pipeline steps:
  `descendants(Button) .map(fn(b) => b.text)`. The `.map`, `.where`,
  `.flat_map`, `.take`, `.drop`, `.first`, `.last`, `.nth(n)`, `.peek(fn)`
  step methods MUST be available in Tier A.
- **AC-QP-3 [A] MUST** allow steps to be named functions:
  `descendants(Button) .my:enrich()` resolves `my:enrich` from the lexical
  scope (AC-QV-*) and invokes it with the receiver as an implicit first
  argument. Authors can also write `my:enrich(., extra)` if the implicit
  arg position is wrong.
- **AC-QP-4 [A] MUST** evaluate `.`-chains **lazily**. A pipeline step does
  not consume more of its input than its output requires.
- **AC-QP-5 [A] MUST** define **short-circuit semantics** for `.first`,
  `.exists`, `.empty`, and `if (…) then … else …` so they stop iteration as
  soon as the answer is known.

---

## 7. Variables, Functions, Scope Inheritance

- **AC-QV-1 [A] MUST** support variable declarations at module scope
  (`declare variable $name := expr`) and at expression scope
  (`let $name := expr in body`). All variables are immutable bindings.
- **AC-QV-2 [A] MUST** support function declarations at module scope
  (`declare function ns:name(args) { body }`). Functions are first-class
  and can be passed to higher-order operators.
- **AC-QV-3 [A] MUST** resolve variable, function, namespace, schema-type,
  and template-reference names using the **host's scope hierarchy as
  the outer lookup, with stdlib bindings interleaved per scope**, then
  the query module scope, then the local lexical scope. Each cem-ml
  `SchemaFrame`/`ScopeId` exposes a single ordered binding set:
  - the variables and functions declared by query modules attached at
    that scope;
  - the schema-derived types (AC-QT-*);
  - the namespace bindings (`NsContext`);
  - the template references (`TemplateRef`);
  - the machine-state slot keys;
  - the **stdlib overlay** for that scope — a map of
    `(module-uri, name) → binding` that re-binds any name reachable
    via a `cem:stdlib/<topic>` import, installed by the scope's policy
    or by the scope's authoring layer.

  Inner scopes inherit the entire binding set from outer scopes,
  including the stdlib overlay; an inner scope MAY shadow any
  inherited binding within parent override bounds per
  `cem-ml-ac.md` AC-P-4 / AC-P-5. Resolution order, per reference:
  ```
  local lexical
    → query module (declarations + non-cem: imports)
      → for each ancestor host scope, innermost first:
            { scope's bindings ∪ scope's stdlib overlay }
        → platform stdlib defaults (implicit root, host-crate-shipped
          implementations of every cem:stdlib/<topic> name)
  ```
  This **resolves** §15 Open Question 2 in favor of *interleaved per
  scope* — a scope is in charge of its mapping and inherits the
  parent's mapping by default. A deeply nested scope MAY override a
  stdlib name (e.g. swap `seq:flatten` for a custom implementation
  with the same signature) and the override applies to that scope and
  its descendants only; sibling subtrees and ancestors continue to see
  the inherited or platform binding. Module identity stability per
  AC-QI-6 is unaffected: the overlay shadows specific *names* within
  the resolution chain, it does not change which module body
  `cem:stdlib/<topic>` loads. The reservation in AC-QI-2 still
  prevents scope policies from claiming `cem:` as an *import grant
  source*; per-scope name shadowing is a distinct, narrower mechanism.

  Overlay-introduced bindings MUST be type-checked at compile time
  against any callers in the same scope subtree per AC-QT-3; overlays
  with incompatible signatures fail compile. Overlay state is
  captured in §14 cache stamps per AC-QC-3 — a binary compiled under
  one scope's overlay is reused only when the loading scope's
  resolved overlay matches. Every overlay hit MUST emit a structured
  resolution-trace event per AC-QV-8 recording the overlay-installing
  scope id alongside the resolved binding.
- **AC-QV-4 [A] MUST** make name resolution **lexical and source-position
  aware**, mirroring the cem-ml namespace-resolution rule in
  `cem-ml-stack-design.md §8`: a re-declaration in the same scope shadows
  earlier uses from its source position forward; previously resolved
  references keep their original binding.
- **AC-QV-5 [A] MUST** apply XSLT-style stylesheet/template-module precedence
  for query modules attached to the same scope: a later attachment with the
  same `module URI` overrides the earlier one for new resolutions but does
  not invalidate already-resolved references.
- **AC-QV-6 [A] MUST** define **closure capture rules**: lambdas capture
  only the variables visible in their lexical scope at definition time.
  Closures MUST NOT capture host AST nodes by reference if the closure
  outlives the host scope; the runtime detaches such captures into a value
  copy and emits `cem.ql.closure_detached` if information is lost.
- **AC-QV-7 [B] SHOULD** allow per-scope **policy hooks**: a scope policy
  may inject named bindings into the query environment for descendants
  through two distinct surfaces, both governed by AC-QV-3's
  per-scope inheritance:
  - **`$scope.*` injection** — fresh names introduced into the
    host-scope binding set, e.g. `$scope.theme` or `$scope.user`.
    Used for context that has no stdlib analogue.
  - **stdlib overlay** — re-binding of an existing
    `cem:stdlib/<topic>` name (the AC-QV-3 overlay map). Used to
    swap an implementation while keeping the same call site syntax,
    e.g. installing a tenant-specific `dom:url-encode` for a
    descendant subtree. Overlay entries MUST match the platform
    binding's signature per AC-QT-3, otherwise compile fails.

  This mirrors XSLT's `xsl:param` passing and the host's
  `MachineStateSlot` model. When a policy injects a binding via either
  surface, both forms below are available; the policy MUST declare which
  form applies per name (policy-declared, both available, with explicit
  cost ownership):
  - **`record(SchemaRef)`** — an eager value carrying a schema-derived
    record type per AC-QT-1. cem-ql code reads it as a normal record
    (`$scope.theme.name`, `$scope.theme.tokens.where(...)`). Static
    type-check applies. Use for small, immutable, public-facing context.
  - **`resource(content-type, SchemaRef?)`** — a host-mediated handle per
    AC-QL-2. cem-ql code dereferences it only through stdlib accessor
    functions in a companion `cem:stdlib/<topic>` module (AC-QI-3). The
    optional `SchemaRef` drives static type-check of accessor return types.
    Use for large, lazy, async, or privacy-sensitive bindings.
  - Inheritance is **by reference** through the AC-QV-3 / AC-QV-4
    resolution chain. Descendant scopes pay no per-inheritance memory
    cost. The **one-time realization cost** for `record` bindings is owned
    by the policy at the scope where the binding is introduced;
    `resource` bindings defer all work to accessor invocation and run
    under the active scope's resource budgets per AC-QR-1.
  - Accessor failures on `resource` bindings route through the host report
    AST per AC-O-3 with `cem.ql.policy_accessor_failed`, attaching the
    originating expression's source-map stack and the policy stamp under
    which the accessor ran.
- **AC-QV-8 [A] MUST** make the inheritance contract testable: every name
  resolution emits a structured event into the report AST recording
  resolved scope id, declaration site, and resolution rule. Used by
  AC-QV-V-* verification.

---

## 8. Type System

- **AC-QT-1 [A] MUST** derive **types** from the schemas attached to the
  current AST scope per `cem-ml-stack-design-impl.md §3.10` and §3.4. The
  type lattice is:
  - **node types**: `node`, `element(QName)`, `attribute(QName)`,
    `text()`, `comment()`, `processing-instruction()`, `document-node()`;
  - **schema-declared element types**: every CEM-native schema element
    becomes a type with the same `ExpandedName` and structural content
    model;
  - **atom types**: XPath-equivalent `string`, `xs:integer`, `xs:decimal`,
    `xs:double`, `xs:boolean`, `xs:date`, `xs:dateTime`, `xs:duration`,
    `xs:anyURI`;
  - **compound types**: `record(k1: T1, …)`, `array(T)`, `stream(T)`,
    `lambda(args …) -> T`;
  - **resource types**: `resource(content-type, schema?)` for unresolved
    external resources.
- **AC-QT-2 [A] MUST** support `instance of`, `cast as`, `treat as`, and
  `is` (identity for nodes). Type-test syntax in axis arguments
  (`descendants(Button)`) is sugar for `descendants() .where(. instance of
  Button)`.
- **AC-QT-3 [A] MUST** check types **statically at query compile time**
  when both sides are statically known, and fall back to runtime checks
  otherwise. Static failures are `cem.ql.type_error`; runtime failures emit
  the same code with the runtime span attached. The **default failure
  contract is strict**, as a deliberate departure from the host's
  forgiving-by-default stance (`cem-ml-ac.md` AC-V-2):
  - **Static** `cem.ql.type_error` — the query module fails to compile.
    No evaluator IR is produced, no compiled artifact is emitted to the
    §14 cache, and `evaluate(...)` MUST reject with the same diagnostic
    attached to the originating expression's source-map stack.
  - **Runtime** `cem.ql.type_error` — the current evaluation aborts at
    the failing call, exactly as for `cem.ql.budget_exceeded` per
    AC-QR-2. Sibling evaluations under unrelated scopes are not
    affected; the diagnostic bubbles per the host bubble-to-boundary
    contract (`cem-ml-ac.md §3.1`, AC-QE-4).
  - The shipped default scope policy MUST therefore stamp
    `cem.ql.type_error` (and the related static-resolution codes
    `cem.ql.unknown_type`, `cem.ql.unknown_function`,
    `cem.ql.unknown_variable`) at `error` severity so AC-QE-4's
    fail-fast/abort path applies.
  - A **development / debugging CLI profile** (a named scope-policy
    preset shipped by the host CLI) MAY relax this contract by
    remapping the codes above to `warning`. Under that profile the
    compiler still emits the diagnostic but **does** produce evaluator
    IR; runtime type failures substitute the empty stream for the
    failing sub-expression and continue evaluating siblings. The
    profile MUST be opt-in (e.g. `--profile=dev` or equivalent) and
    MUST NOT be the default for non-interactive callers (templates,
    validators, build pipelines, server-side hosts).
  - This **resolves** §15 Open Question 5.
- **AC-QT-4 [A] MUST** make schema-type identity **scope-relative**: the
  same lexical name `Button` may resolve to different schema-declared types
  in different host scopes if different schemas are active per
  `cem-ml-stack-design.md §8`. Resolution follows AC-QV-3.
- **AC-QT-5 [B] SHOULD** generate `record` types from schema-declared
  attribute groups so `node.@*` returns a typed record.
- **AC-QT-6 [B] MAY** emit type stubs (TypeScript and Rust) for query
  modules so external code can call them through the host's API per
  `cem-ml-ac.md` AC-S-3 / AC-S-4.

---

## 9. Async & External Data

- **AC-QA-1 [B] MUST** expose `read(uri, accepts?)` as the only built-in
  way to load an external structured document. The signature is a
  content-negotiation surface, not a content-type assertion: the caller
  declares which shapes it is prepared to consume; the resource and the
  host transform graph decide which one is produced. The function MUST:

  - resolve `uri` against the active scope's `base_uri`;
  - accept `accepts` in **three forms**, in precedence order:
    1. **Omitted (or `()` / `null`)** — equivalent to passing the
       full **CEM-supported floor list** from AC-QA-1.1 in its declared
       order. Authors writing `read(uri)` get "anything the host can
       turn into a CEM-supported type"; the host transform graph picks
       the highest-preference floor entry reachable from the wire type.
    2. **Single string** — parsed as an HTTP `Accept` header per
       RFC 9110 §12.5.1: comma-separated media-range entries with
       optional `;q=<weight>` parameters. Resulting preference order
       is q-value descending, then source order on ties. Wildcard
       ranges (`*/*`, `text/*`) MUST be expanded against the host
       transform graph at evaluation time, not at compile time.
    3. **Collection of strings** (stream or array) — each item is one
       full media-type identifier (no q-values, no media ranges).
       Order is caller preference order, left-to-right. Empty
       collection is equivalent to omission per (1).
  - convey the resolved preference list to the resource loader: HTTP(S)
    loaders MUST emit a corresponding `Accept` header (forms (2) and
    (3) round-trip exactly; form (1) emits the floor list with
    descending q-values); `file://` and other byte-stream loaders
    retrieve bytes and label the response from response metadata, file
    extension, or content sniff in that order;
  - dispatch the **returned** wire content type into the cem-ml
    content-type transform pipeline (`cem-ml-ac.md` AC-I-2 / AC-T-*
    and the stack design §3.2 / §9) and select the
    **highest-preference resolved `accepts` entry reachable from the
    wire type via the registered transform graph**. A wire type that
    is itself in the resolved `accepts` is a zero-step path and wins
    by definition;
  - produce a `Stream<node>` of the parsed-and-transformed document's
    roots, typed as the selected `accepts` entry;
  - reuse the host's external-resource I/O queue per
    `cem-ml-ac.md` AC-A-6 (no thread-pool slot, scope-bounded);
  - reject with `cem.ql.read_denied` when the active scope policy
    denies the scheme/host of the resolved URI or denies the wire
    content type;
  - reject with `cem.ql.read_unsatisfiable` when the loader returns
    bytes labelled with a wire content type from which no transform
    path exists to any entry in the resolved `accepts`. The diagnostic
    carries the wire type, the resolved `accepts` list, the form
    (omitted / header-string / collection) the caller used, and the
    originating expression's source-map stack.

  This **resolves** §15 Open Question 4. There is no frozen Tier B
  content-type set in the AC: the reachable types are exactly those the
  host plugin chain (`cem-ml-ac.md §7`) can produce or transform between.
  cem-ql expresses intent via `accepts`; the host owns the transform
  graph; cross-host portability of compiled artifacts is determined by
  whether a target host's transform graph can satisfy the same
  `accepts` lists.

### AC-QA-1.1 Canonical content-type identifiers (Tier B compliance floor)

The Tier B **compliance floor** — content types every conformant
Tier B engine MUST be able to satisfy as an `accepts` target when the
host has the corresponding plugin installed, and the **default
preference list** when `accepts` is omitted (form (1) above) — uses
the canonical identifiers below. These identifiers are **part of the
AC**: they are what `accepts` collections compare against, what
compiled artifacts stamp per AC-QC-3, and what HTTP `Accept` headers
emitted by `read()` carry. Aliases parse but normalize to the
canonical form before stamping or comparison.

| Canonical identifier         | Floor name      | Accepted aliases (informative)                  |
|------------------------------|-----------------|-------------------------------------------------|
| `text/html`                  | HTML            | —                                               |
| `application/xml`            | XML             | `text/xml`                                      |
| `image/svg+xml`              | SVG             | —                                               |
| `application/mathml+xml`     | MathML          | —                                               |
| `text/css`                   | CSS             | —                                               |
| `text/x-scss`                | SCSS            | `text/scss`                                     |
| `application/json`           | JSON            | `text/json`                                     |
| `application/yaml`           | YAML            | `text/yaml`, `application/x-yaml`               |
| `text/csv`                   | CSV             | —                                               |
| `application/javascript`     | JS island       | `text/javascript`                               |
| `application/typescript`     | TS island       | `text/typescript`, `application/x-typescript`   |
| `application/cem+xml`        | CEM-ML          | —                                               |

The **default preference list** (form (1)) is the rows above in the
order shown — HTML first, CEM-ML last. Plugin-registered content
types beyond the floor are usable through the same surface; their
identifiers are owned by the registering plugin per `cem-ml-ac.md §7`,
their portability is host-defined, and they are **not** added to the
default preference list — callers asking for a plugin type MUST pass
it explicitly via form (2) or form (3).

A cem-ql stdlib module `cem:stdlib/content-types` (Tier B; see
AC-QI-3) MUST expose the canonical identifiers above as exported
string constants (e.g. `ct:html`, `ct:json`, `ct:cemml`) and the
default preference list as `ct:floor`, so authors can write
`read($u, [ct:json, ct:yaml])` without re-typing string literals.
- **AC-QA-2 [B] MUST** support **awaitable** semantics: pipeline operators
  consuming a `read()` stream automatically await partial results without
  surfacing an explicit `await` keyword. Authors MAY write `await expr` for
  clarity; it parses but is a no-op when `expr` is already a stream.
- **AC-QA-3 [B] MUST** propagate `AbortSignal` per `cem-ml-ac.md` AC-A-7.
  An aborted query MUST stop fetching, stop iterating, and emit
  `cem.ql.aborted` with the active scope context. Pending stream items are
  released.
- **AC-QA-4 [A] MUST** make Tier A queries usable without `read()`. Tier A
  evaluators MUST NOT require an external loader to be configured.
- **AC-QA-5 [B] SHOULD** support **content-typed write helpers** that build
  in-memory results without touching the filesystem: `parse_html(string)`,
  `parse_xml(string)`, `parse_json(string)`, `parse_csv(string)`,
  `parse_yaml(string)`. They share the host's content-type transform
  pipeline and are off when the host has not enabled the relevant content
  type.

---

## 10. Imports, Modules, and Stdlib

- **AC-QI-1 [A] MUST** support a query-module `import` statement:
  `import "uri" as alias`. Imported modules contribute variables,
  functions, and types under `alias:`.
- **AC-QI-2 [A] MUST** define module-import behavior **per URI scheme**.
  cem-ql recognizes exactly three scheme tiers, in ascending order of
  trust requirement; this **resolves** §15 Open Question 3:

  | Scheme        | Source                                                    | Policy gate                                                      |
  |---------------|-----------------------------------------------------------|------------------------------------------------------------------|
  | `cem:`        | platform implementation, baked into the host crate        | none — always available, scope policy cannot deny                |
  | `urn:cem:`    | dynamically registered via plugins or a config-time registry map | host trust setup (plugin install / config) — per-scope policy cannot deny what was registered, but unregistered URIs fail with `cem.ql.import_unresolved` |
  | `https:` (and other network schemes: `http:`, `file:`, plugin-registered transports) | online or local resource carrying its own schema, content-type metadata, and module body | **scope policy** per AC-QI-4 — off by default, granted by scheme/host/path prefix |

  Both `cem:` and `urn:cem:` are **reserved**: scope-policy grants whose
  source matches either scheme are rejected at policy load with
  `cem.ql.reserved_scheme`. This prevents an attacker who can edit a
  scope policy from shadowing platform stdlib lookups. Network-scheme
  imports denied by the active scope policy emit `cem.ql.import_denied`
  and severity propagates per `cem-ml-ac.md §3.1`.

- **AC-QI-3 [A] MUST** ship a **cem-ml standard library** as the only
  out-of-the-box import set. Stdlib modules use the platform
  scheme `cem:stdlib/<topic>` per AC-QI-2 and resolve from the host
  crate without any policy grant. The shorter scheme reflects that
  these modules ship **with the platform** — they are not loaded
  dynamically and they are not fetched. Initial Tier A stdlib topics:
  - `cem:stdlib/sequence` — set/stream helpers from AC-QO-6;
  - `cem:stdlib/strings` — string manipulation, codepoint iteration,
    regex (Tier B for regex);
  - `cem:stdlib/numbers` — math, formatting, bigint helpers;
  - `cem:stdlib/datetime` — XPath `xs:date / xs:dateTime` helpers;
  - `cem:stdlib/dom` — host AST helpers (axes, attribute access,
    reference resolution) when authors want them as functions instead of
    pipeline steps;
  - `cem:stdlib/report` — diagnostic emit and severity helpers;
  - `cem:stdlib/state` — read-side machine-state slot helpers;
  - `cem:stdlib/template` — template-registry lookup helpers;
  - `cem:stdlib/cemml` — read CEM-ML canonical content from in-memory
    strings;
  - `cem:stdlib/content-types` (Tier B) — canonical media-type
    identifiers and the default `read()` preference list per
    AC-QA-1.1 (`ct:html`, `ct:xml`, `ct:svg`, `ct:mathml`, `ct:css`,
    `ct:scss`, `ct:json`, `ct:yaml`, `ct:csv`, `ct:js`, `ct:ts`,
    `ct:cemml`, `ct:floor`).
- **AC-QI-4 [B] SHOULD** support **scope-policy-gated user modules**
  under the network-scheme tier of AC-QI-2 (`https:`, `http:`, `file:`,
  plugin-registered transports). Grants are per scope, listed by
  scheme/host/path prefix, and follow the host's external-resource
  policy; nothing new is invented here. Modules under `urn:cem:` are
  **not** subject to this gate — their availability is determined by
  host trust setup (plugin registration or config-time registry
  mapping) per AC-QI-2, not by per-scope grants.
- **AC-QI-5 [A] MUST NOT** allow side-effecting imports. A module MUST be
  loadable, parseable, and type-checkable without executing code.
- **AC-QI-6 [A] MUST** make module identity stable: a module is keyed by
  its URI plus its declared `module` URI. Two attachments to the same scope
  with the same module URI deduplicate to one binding.
- **AC-QI-7 [B] MUST** mirror **XSLT include/import precedence** for query
  modules: `import` brings names with lower precedence; `include` (Tier B
  syntactic form) brings names at the importing module's precedence. This
  matches XSLT and is needed to keep XSLT-style override patterns
  expressible.

---

## 11. Errors, Diagnostics, and Reports

- **AC-QE-1 [A] MUST** route every diagnostic through the host report AST
  per `cem-ml-ac.md` AC-O-3. Stable diagnostic codes use the `cem.ql.*`
  prefix. Initial codes:
  - `cem.ql.parse_error`
  - `cem.ql.type_error`
  - `cem.ql.unknown_type`
  - `cem.ql.unknown_function`
  - `cem.ql.unknown_variable`
  - `cem.ql.scope_violation`
  - `cem.ql.unresolved_reference`
  - `cem.ql.cross_type_compare`
  - `cem.ql.use_and_or`
  - `cem.ql.import_denied`
  - `cem.ql.import_unresolved`
  - `cem.ql.reserved_scheme`
  - `cem.ql.read_denied`
  - `cem.ql.read_unsatisfiable`
  - `cem.ql.read_dynamic_accepts`
  - `cem.ql.aborted`
  - `cem.ql.budget_exceeded`
  - `cem.ql.closure_detached`
  - `cem.ql.policy_accessor_failed`
- **AC-QE-2 [A] MUST** support an XPath/XQuery-style `try { … } catch (code,
  msg) { … }` (Tier B for the surface keyword; Tier A reports through the
  diagnostic channel only).
- **AC-QE-3 [A] MUST** make every diagnostic include the query
  `SourceMapStack` and the active host scope context per `cem-ml-ac.md`
  AC-O-3.
- **AC-QE-4 [A] MUST NOT** abort the host parse on a query-time error
  unless the active scope policy maps the diagnostic code to fail-fast/abort
  behavior, exactly per the host bubble-to-boundary contract.

---

## 12. Performance, Resource, and Security Limits

- **AC-QR-1 [A] MUST** make every evaluation resource-bounded by the host's
  scope policy (`cem-ml-ac.md` AC-A-4 / AC-A-5 / AC-N-2). Bounded fields
  for queries:
  - max items materialized per pipeline stage;
  - max recursion depth per call;
  - max function-call count per evaluation;
  - max captured-closure size;
  - max regex backtracking budget (Tier B);
  - max external-document fetches (Tier B; uses the host's I/O queue).
- **AC-QR-2 [A] MUST** emit `cem.ql.budget_exceeded` and abort the failing
  evaluation when a limit is hit. The diagnostic carries the limit name and
  the offending source-map stack.
- **AC-QR-3 [A] MUST NOT** expose `eval`, dynamic-source compile, or any
  way to load executable code from data. Templates and modules are loaded
  through the import surface only.
- **AC-QR-4 [A] MUST** treat all input as untrusted per `cem-ml-ac.md`
  AC-X-1. Even AC-QA-5 in-memory parsers run inside the host's
  content-type transform sandbox.
- **AC-QR-5 [B] SHOULD** publish a benchmark suite for representative
  selectors over the `examples/semantic/*.html` fixtures, hooked into
  `cem-ml-ac.md` AC-N-3. Selector + transform end-to-end stays under the
  host's 150 ms Tier A budget when run together.

---

## 13. Verification Plan

A `cem-ql` Tier A release is acceptance-tested with:

1. `yarn nx run cem_ql:test` — unit tests for parser, type checker,
   evaluator, and stdlib.
2. `yarn nx run cem_ql:test:xpath-parity` — table-driven tests against the
   XPath 3.1 conformance suite, restricted to the AC-QX-1 subset. Failures
   on out-of-subset items are skipped, not reported as failures.
3. `yarn nx run cem_ql:test:fixtures` — runs every Tier A query the CEM
   templates need to transform `examples/semantic/*.html`. Output snapshots
   match the host's existing transform snapshots.
4. `yarn nx run cem_ql:bench` — selector benchmarks shared with the host
   `cem_ml_cli:bench` budget per AC-QR-5.
5. **AC-QV-V-1** — scope-inheritance test, three cases:
   (a) **User-name inheritance and shadowing**: a parent module
   declares `local:fmt(...)`; a child scope's query resolves it and
   the resolution-trace event (AC-QV-8) records the resolution rule.
   Re-declaring `local:fmt` in the child scope shadows it; uses
   earlier in the child still see the inherited binding (per AC-QV-4).
   (b) **Stdlib overlay inheritance**: a parent scope's policy
   installs an overlay binding for `seq:flatten` (matching the
   platform signature per AC-QT-3); a descendant scope's query
   resolving `seq:flatten` hits the overlay, and the resolution-trace
   event records the overlay-installing scope id. A sibling subtree
   without the overlay still resolves to the platform binding.
   (c) **Overlay shadowing in deep nesting**: a grandchild scope
   re-overlays `seq:flatten` again; the grandchild and its descendants
   see the grandchild's binding, the parent of the grandchild
   continues to see the parent-installed overlay, and platform stdlib
   remains visible at the root scope. Resolution traces distinguish
   all three layers.
6. **AC-QO-V-1** — set-operator fixture: produce four overlapping streams
   `A, B`; assert `A | B`, `A & B`, `A - B`, `A ^ B` against committed
   snapshots; confirm document order and identity rules per AC-QO-2 /
   AC-QO-3.
7. **AC-QI-V-1** — import gating test, one case per AC-QI-2 scheme tier:
   (a) **`cem:` (platform)**: `import "cem:stdlib/sequence"` resolves
   without any scope-policy grant; a scope policy that *attempts* to
   list `cem:` in its grant set is rejected at policy load with
   `cem.ql.reserved_scheme`. (b) **`urn:cem:` (dynamic registry)**: a
   plugin registers `urn:cem:acme/widgets`; import resolves without a
   scope-policy grant; importing the unregistered
   `urn:cem:acme/missing` fails with `cem.ql.import_unresolved`; a
   scope-policy grant for `urn:cem:` is rejected with
   `cem.ql.reserved_scheme`. (c) **`https:` (network)**: an
   unwhitelisted `https://example.com/mod` fails with
   `cem.ql.import_denied` at warning severity by default; raising the
   policy to `error` aborts the evaluation; whitelisting the URI loads it.
8. **AC-QA-V-1** — `read()` content-negotiation test under a Tier B
   policy that grants `file://fixtures/`. Covers all three input forms
   from AC-QA-1 plus the failure mode:
   (a) **Form (1) — omitted**: `read($u)` over an HTML, an XML, a
   JSON, and a CSV fixture; assert nodes typed as the matching floor
   entry from AC-QA-1.1 and that the emitted HTTP `Accept` header
   (when applicable) carries the floor list with descending q-values.
   (b) **Form (2) — header string**: `read($u, "application/json;q=0.9,
   application/yaml;q=1.0")` over a fixture the loader can return as
   either type; assert YAML wins on q-value and the `*/*` wildcard
   case expands at load time, not compile time.
   (c) **Form (3) — collection**: `read($u, [ct:cemml, ct:json])`
   from `cem:stdlib/content-types`; assert preferred type is
   selected when reachable, otherwise the next entry; assert alias
   inputs (`text/xml` → `application/xml`) normalize before
   comparison.
   (d) **Wire/accept transform**: read a YAML fixture with
   `accepts = [ct:json]`; assert the YAML→JSON transform fires and
   the result is typed as JSON.
   (e) **Unsatisfiable**: read a binary fixture whose wire type has
   no transform path to any `accepts` entry; assert
   `cem.ql.read_unsatisfiable` with the wire type, the resolved
   `accepts` list, and the original input form recorded in the
   diagnostic.
9. **AC-QD-V-1** — reference-resolution test: a query against a fixture
   with `for=` and `aria-labelledby=` resolves through `.target` and emits
   the documented warning when a target is missing.
10. **AC-QR-V-1** — budget test: a deliberately-wide `descendants()` query
    over a synthetic 10 MB fixture hits the per-pipeline materialization
    cap and aborts with `cem.ql.budget_exceeded`.
11. **AC-QV-V-2** — policy-hook test: a parent scope's policy injects two
    bindings — `$scope.theme` as `record(theme-schema)` and `$scope.user`
    as `resource("user-profile", user-schema)`. A descendant scope (a)
    reads `$scope.theme.name` via record-style field access and statically
    type-checks against the schema; (b) calls `user:has_role($scope.user,
    "admin")` from `cem:stdlib/user`; (c) confirms the bindings are
    inherited by reference (no clone-on-inherit cost on a deep scope
    chain); (d) forces the accessor to fail and asserts
    `cem.ql.policy_accessor_failed` is emitted with the correct source-map
    stack.

Each section above contributes a concrete check to one of these scripts; AC
items missing a check are not closeable.

---

## 14. Compiled Query Artifact & Cache Protocol

cem-ql queries participate in the **shared content-addressed cache and
transport protocol** defined by [`cem-ml-ac.md` §14](cem-ml-ac.md). The cem-ml
host owns the protocol; this section binds cem-ql to it so a single loader
implementation handles both kinds of artifact and a single `CEM-Hash` header
governs both.

- **AC-QC-1 [B] MUST** treat a cem-ql module — after parse, name resolution,
  schema-type resolution, and type-check — as a **compiled query artifact**
  hashable under `cem-ml-ac.md` AC-CC-1. The hash inputs are the canonical
  UTF-8 module source, the cem-ql version, the active schema fingerprint at
  compile time, the **resolved stdlib overlay fingerprint** at compile
  time per AC-QV-3 (so two hosts that compile the same source under
  different overlays produce distinct artifacts), and the hash-scheme
  tag. Hash identity MUST be reproducible across hosts whose overlays
  fingerprint identically.
- **AC-QC-2 [B] MUST** serialize the compiled artifact to the shared binary
  form per AC-CC-2: typed evaluator IR, resolved schema-type bindings (or
  rebindable stubs), captured source-map stacks (dev mode only per AC-CC-4),
  the import closure, and the policy stamps under which it was compiled.
  Reloading the binary MUST skip the cem-ql parser, type checker, and name
  resolver and resume at evaluation.
- **AC-QC-3 [B] MUST** carry **policy stamps** per AC-CC-3: declared
  imports, declared `read()` `accepts` lists (one stamp per `read()`
  call site, capturing the static `accepts` value when statically
  known), declared external resources, and the **resolved stdlib
  overlay** under which the artifact was compiled (per AC-QV-3). The
  overlay stamp is a deterministic hash of the
  `(module-uri, name) → binding-fingerprint` map for every overlaid
  name reachable from the artifact; the absent-overlay case (pure
  platform stdlib) hashes to a sentinel that any host can satisfy.
  A binary whose overlay stamp the loading scope's resolved overlay
  cannot reproduce MUST fail with `cem.cc.policy_mismatch`. Stamping
  rules for `read()` follow the AC-QA-1 input forms:
  - **Form (1) — omitted** stamps the canonical floor preference list
    from AC-QA-1.1 by reference (a single `accepts: floor` marker, not
    the expanded list, so a future floor extension does not invalidate
    older binaries).
  - **Form (2) — header string** is parsed at compile time when
    statically known, normalized through the AC-QA-1.1 alias table,
    and stamped as the canonical preference list with q-values; an
    unresolvable wildcard (`*/*`, `text/*`) stamps as the unexpanded
    range — wildcard expansion is deferred to load-time evaluation
    against the active host's transform graph.
  - **Form (3) — collection** is normalized through the alias table
    and stamped in caller order. Dynamically computed entries emit
    `cem.ql.read_dynamic_accepts` at compile and stamp as wildcard.

  A binary whose stamps the active scope policy cannot satisfy MUST
  fail with `cem.cc.policy_mismatch` and fall back to the source if
  available. Stamps record **caller intent**, not the wire content
  type observed at compile time — cross-host portability requires the
  target host's transform graph to satisfy the same `accepts` lists,
  not to expose the same wire types. Scope-relative schema-type
  identity (AC-QT-4) MUST re-resolve on load; unresolved types emit
  `cem.ql.unknown_type` exactly as on a fresh compile.
- **AC-QC-4 [B] MUST** participate in the **transport protocol** per
  AC-CC-6 / AC-CC-7. Servers that ship cem-ql modules — stdlib URIs,
  policy-granted user modules per AC-QI-4, plugin-supplied query modules —
  MUST emit `CEM-Hash`. Engines holding a cached compiled artifact MAY send
  `If-CEM-Hash`; a confirmation-only `304` is sufficient to satisfy the
  module load. This is the **resolution** of the previously-open
  "compiled query artifact" question: queries compile to a portable binary,
  share the host's cache/transport, and Tier C may add chunked or
  cross-artifact deduplication per AC-CC-9 / AC-CC-10.
- **AC-QC-5 [B] MUST** preserve source-map stacks in dev-mode binaries so a
  diagnostic emitted from a reloaded query — including `cem.ql.parse_error`
  surrogates, `cem.ql.type_error`, `cem.ql.unresolved_reference`, and the
  resolution-trace events from AC-QV-8 — is indistinguishable from the
  source-driven diagnostic.
- **AC-QC-6 [B] MUST NOT** ship dynamic-source compilation per AC-QR-3: the
  binary form is a **load-time** artifact emitted by a trusted compile
  stage (build pipeline, CLI, or in-process pre-warm). `eval`-style runtime
  compilation of arbitrary string input remains prohibited.
- **AC-QC-7 [B] MUST** scope the cache by `(content-type=cem-ql, hash, mode)`
  per AC-CC-5; dev and prod compiled artifacts are distinct cache entries.
- **AC-QC-V-1 [B]** — verification: compile a Tier A query corpus to
  dev-mode binaries, evict in-memory state, reload, re-evaluate against
  the same fixtures; assert diagnostics, stream order, set-operator
  identity, and source-map stacks match the source-driven run.
- **AC-QC-V-2 [B]** — verification: end-to-end `If-CEM-Hash` test through
  the cem-ml-cli loader: server returns `304` for an already-cached
  compiled query; engine evaluates from cache; assert the cem-ql parser
  is not entered on the second pass.

---

## 15. Open Questions

These must be answered before AC are testable:

1. **AC-QO-1 dedup identity for atoms** — XPath value-equality vs. strict
   IEEE-754 vs. string-canonicalization identity for `decimal`/`double`
   set-op deduplication.

---

## 16. References

- Primary host AC: [`cem-ml-ac.md`](cem-ml-ac.md), in particular
  AC-F-1 / AC-F-7 (scopes, async streams), AC-P-* (parser surface),
  AC-V-* (validation), AC-T-1 / AC-T-3 (transformation surface),
  AC-A-* (async APIs), AC-O-* (observability), AC-X-* (security).
- Stack design: [`cem-ml-stack-design.md`](cem-ml-stack-design.md) §4
  (source-map model), §8 (schema machine and namespace resolution), §12
  (transform execution backend), §13 (CEM schema language).
- Implementation contracts:
  [`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md) §3.4 / §3.4.1
  (schema frames, namespace context), §3.10 (`ScopedQuery`,
  `QueryContextScope`), §3.11 (visual content / machine state /
  hydration).
- XPath 3.1: <https://www.w3.org/TR/xpath-31/>
- XQuery 3.1: <https://www.w3.org/TR/xquery-31/>
- XSLT 3.0: <https://www.w3.org/TR/xslt-30/>
- XSLT 4.0 candidate (qt4cg): <https://qt4cg.org/specifications/xslt-40/>
- RELAX NG schema for XSLT 4.0: <https://qt4cg.org/specifications/xslt-40/schema-for-xslt40.rnc>
- JQ language reference (selector + lambda design influence):
  <https://jqlang.github.io/jq/manual/>
- Python data model and set/sequence operators (collection-op influence):
  <https://docs.python.org/3/reference/datamodel.html>,
  <https://docs.python.org/3/library/stdtypes.html#set>
- Companion docs to be created:
  - `cem-ql-stack-design.md` — concrete grammar, evaluator IR, parity
    matrix, type system layout.
  - `cem-ql-stack-design-impl.md` — Rust module map, evaluator algorithm,
    cost model, diagnostic table.