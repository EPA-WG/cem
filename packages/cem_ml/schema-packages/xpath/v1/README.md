# XPath Schema Package

Status: package, lossless XPath 3.1 syntax, lifecycle loading, typed evaluation contracts, and portable compiled programs

This package owns standalone and embedded XPath expression syntax. Host
languages declare expression slots and static context, then associate the
resulting `XPathExpressionAst` with a document, subtree, element, or attribute.
The XPath AST remains independently addressable when its events are fused into
an XSLT, XML, CEMT, or CEM-QL transformation stream.

## Owned Identities

- Schema URI: `https://cem.dev/ns/query/xpath/1`
- Namespace prefix: `xpath`
- Primary content type: `application/vnd.cem.xpath`
- Interoperability alias: `text/xpath`
- Compiled program content type: `application/vnd.cem.xpath-artifact+cem-bin`
- Result artifact content type: `application/vnd.cem.xpath-result+json`
- Preferred extension: `.xpath`
- Syntax baseline: [XPath 3.1](https://www.w3.org/TR/xpath-31/)

XPath is specified as a component used by host languages and has no standalone
media type in the [IANA media-type registry](https://www.iana.org/assignments/media-types/media-types.xhtml).
The package therefore uses a CEM vendor media type as primary identity and does
not present `text/xpath` as a registered standard.

## Syntax And AST Model

The current foundation uses a CEM-owned longest-match scanner that preserves
exact token lexemes, UTF-8 byte ranges, line/column positions, nested comments,
whitespace, delimiter depth, lexical errors, and source-map frames. Its
package-private token categories distinguish numeric forms, strings, EQNames,
keywords, word and symbol operators, punctuation, trivia, and errors. The
scanner follows the normative XPath 3.1 lexical grammar and does not call Xee.
The pinned `xee-xpath-lexer` crate is a development-only differential oracle
for package examples and ambiguous lexical boundaries.

The CEM-owned recursive-descent parser consumes the scanner token stream
directly, resolves names from the attachment static context, and constructs the
typed package AST without reparsing or an intermediate representation. The
`xee-xpath-ast` and `xee-xpath-lexer` crates are development-only differential
oracles for the completed grammar slices. The
[Xee source pinned at commit `200b1e3356ea9d6dd2901d67bd941b779df7e5b7`](https://github.com/Paligo/xee/tree/200b1e3356ea9d6dd2901d67bd941b779df7e5b7)
is an MIT-licensed, non-normative implementation reference, never an AST or
execution boundary. XPath 3.1, XDM 3.1, and Functions and Operators 3.1 remain
normative, and adapted implementation ideas require recorded source provenance
and license review.

Full XPath 3.1 is the accepted destination. Delivery is staged through explicit
conformance slices and the schema-owned
[`tests/xpath-3.1-conformance.cem`](./tests/xpath-3.1-conformance.cem) gap
matrix. Behavior outside a completed slice remains visible through stable typed
diagnostics rather than silently inheriting omissions from the reference
implementation.

The primary syntax contract is a strongly typed W3C expression model with
typed names, literals, operators, sequence types, paths, steps, node tests,
maps, arrays, and function items. The lossless token stream remains a separate
source-fidelity artifact. XSLT, CEMT, and CEM-QL fusion consumes a derived
start/end syntax event stream rather than weakening the primary AST into a
generic property bag. The current parser slice represents rooted and relative
paths, axes, node tests, predicates, simple maps, variables, binary operators,
function calls, arrow expressions, type operators, maps, arrays, typed
names/literals, and host-adjusted ranges directly. Typed sequence types retain
`empty-sequence()`, `item()`, atomic EQNames, unconstrained node-kind tests,
parenthesized item types, occurrence indicators, and exact ranges. Typed single
types retain their resolved atomic EQName, optional-empty indicator, and exact
range for `cast as` and `castable as`. Simple-map
grammar nodes retain one input path and an ordered vector of mapping paths
rather than masquerading as generic binary operators. Their balanced syntax
events expose each path child in source order. Named arrows lower canonically to
ordinary function calls with the left operand inserted as argument zero;
variable and parenthesized specifiers lower to the existing postfix dynamic-call
form. This preserves authored token fidelity without adding an arrow-only public
AST variant. The runtime parser and public syntax module have no Xee, serde,
Xot, or JSON representation dependency.

The lifecycle stream emits one zero-width `start-expression` event, one event
for each lossless token, and one zero-width `end-expression` event. Token events
retain their token index, delimiter depth, absolute host-adjusted range, and
source map, so a host can fuse the stream without rewriting XPath identities.

The adapter follows XPath 3.1 longest-match tokenization. It does not reuse the
legacy custom-element XPath rewriter or infer XPath by applying CEM-QL syntax.

## Schema-Owned Diagnostics

The native parser emits neutral facts for decode, lexical, parse, namespace,
delimiter, host-association, external-resource, source-map, and event-lifecycle
conditions. `schema/xpath.cem` owns the diagnostic code, severity, contract,
behavior, and policy bound to each reportable fact.

Standalone validation accepts the primary content type, the `text/xpath` alias,
or the package schema URI and maps those facts without lowering the expression
to CEM. Diagnostics retain exact byte, line, column, and source-map coordinates.
Manifest-owned pass and failure fixtures exercise the same validation path.

## Host Association

Standalone expressions use their own source identity. Embedded expressions add
an attachment envelope containing:

- host source, content type, schema, node kind, node identity, and source range;
- the absolute expression range within the host source;
- namespace, variable, and function static-context bindings;
- expected sequence result and evaluation phase;
- resolver and safety policy stamps.

This allows an XPath tree to be attached to an XML document or AST subtree and
also fused into an owning XSLT stream without transferring grammar ownership to
XSLT. Whole-expression attributes and every schema-classified literal-result or
XSLT 3.0 instruction AVT expression directly own the package AST, exact
attribute-value range, owning XML event identity, and inherited namespace
context. Their single parse views the generic XML AST's decoded
scalar-to-source map, so entity-decoded tokens and diagnostics retain original
XML coordinates without a serializer, replacement tree, or range rewrite.

## Compiled Programs

XPath owns its executable artifact independently of CEM-QL and XSLT. The
`validation::xpath::artifact` API compiles an already parsed
`XPathExpressionAst` to `XPathCompiledArtifact`, checks externally supplied
content hashes when loading bytes, and reloads the typed program against an
expected owning-source hash and invocation host. For an embedded expression,
the source hash identifies the original host stylesheet bytes, not a
reconstructed attribute string. The expected hashes must come from trusted
compiler output or a manifest; a digest checks integrity, not publisher
authenticity.

The binary format is `cem-xpath-artifact/1`, with typed program format
`cem-xpath-program-v3`. The v3 program retains inline-function parameters,
result types and bodies alongside typed lookups; earlier v1/v2 programs are
rejected and must be recompiled. Its envelope identifies the XPath schema, content type,
grammar/compiler versions and source hash. It preserves typed syntax, source
ranges, namespace/static context and host attachment. Reload derives syntax
events without retaining or reparsing source text/tokens. The private codec
does not add serialization dependencies to the public source syntax model.
Runtime data ASTs, result sequences, callbacks and dynamic bindings are never
encoded. Round-tripping a syntax construct does not make an unfinished XPath
evaluation slice executable.

The explicit WASM control-plane entry points are:

- `compileXPathArtifact(source, sourceUri)`: returns standalone XPath artifact
  bytes.
- `importXPathArtifact(bytes, expectedContentHash, expectedSourceHash,
  invocationHost)`: validates and retains a program, returning JSON handle and
  identity metadata, not a serialized expression or data AST.
- `disposeXPathArtifact(artifactId)`: releases the registry's program owner;
  unknown or disposed handles return false and handles are never reused.

Invocation hosts are `query`, `standalone-transform`, `xslt`, `cemt` and
`cem-ql`; they must match the program's source attachment. An arbitrary XML
association does not imply a language host and is rejected for executable
reload. Rust host integration may retain the typed program through
`api::wasm::retained_xpath_artifact`.
Importing a program does not register a callback or enable a CEMT capability.
Ordinary JSON data cannot install capabilities. An XSLT-owned bundle and its
explicit host bindings remain separate, incomplete work.

The registered binary content type is consumed through these explicit artifact
APIs, not automatically through the source/lifecycle importer. Existing XPath
source identities, default importers and generic CEMT artifacts are unchanged.
Artifacts are limited to 2 MiB, 128 codec nesting levels, 65,536 encoded/decoded
values and 4,096 entries per collection. WASM retains at most 64 programs and
accounts for at most 16 MiB of their encoded sizes. Invalid tags, lengths,
UTF-8, source-range arithmetic, identities and trailing bytes fail closed.

## Transformation Boundary

XPath is a transformation language peer to CEM-QL, CEMT, and XSLT. The package
owns parsing and static syntax; the CEM-ML `transform` path owns execution
planning. Hosts may supply context items and bindings, but must not implement a
private parser, evaluator, or external-resource resolver.

Execution will consume the package-owned AST and existing CEM XML AST/event
streams directly. It must not reparse source text, copy XML into Xot or another
evaluator-owned replacement tree, or project input/results through JSON. The
strict native-AST transform data-plane contract tracked in `docs/todo.md` is
enforced by the registered standalone execution path.

The current implementation defines `XPathEvaluationRequest`,
`XPathEvaluatorCapabilities`, and `XPathResultArtifact`. Result sequences retain
XPath order across node, atomic, map, array, function, and mixed items. Node
items retain the exact lifecycle AST owner plus a typed node handle alongside
source/node identity, atomic values retain type plus lexical value, and function
items are evaluator-scoped handles rather than serialized closures. Every
artifact and item carries an origin-first source map, and the result keeps the
static context plus resolver and safety policy stamps.

The native evaluator slices execute literals, variables, context items,
expression sequences, rooted-descendant paths, and every non-namespace axis
with axis-aware name or kind tests directly over the package-owned XPath AST and
lifecycle-owned XML event AST. Axis-step predicates receive focus in axis order,
including reverse order for ancestor and preceding axes, before path results are
identity-deduplicated and returned in document order. Postfix filter predicates
preserve their base sequence order. Both predicate forms implement numeric
position filtering and XPath effective-boolean-value rules, with native
`position()` and `last()` focus functions. Logical `and` and `or` apply that
same EBV kernel directly to AST operand results, evaluate left to right, skip a
right operand when the left result determines the answer, and retain the full
logical-expression source map on the typed boolean result. A native atomic
kernel represents integer and decimal values as unbounded normalized
coefficient/scale values and executes all general and value comparison
operators for supported strings, URIs, booleans, untyped XML values, integers,
decimals, floats, and doubles.
Comparisons atomize retained XML nodes directly, distinguish existential general
comparison from singleton value comparison, apply untyped conversion and
numeric promotion rules, preserve IEEE NaN behavior, and report cardinality or
cast errors with expression source maps. Node comparisons require
optional-singleton retained native nodes: `is` compares exact AST-owner and node
handle identity, while `<<` and `>>` compare the existing document-order keys
for nodes in the same owner. Empty operands propagate the empty sequence,
operand failures retain operand ranges, and node-shaped values without their
native handles are rejected rather than reconstructed. Cross-owner node
ordering remains explicitly unsupported until the host defines a stable
multi-document order. The `union`/`|`, `intersect`, and `except` operators
likewise consume retained node sequences directly, reject non-node or detached
operands, perform membership and duplicate elimination by exact native node
identity, and return same-owner results in document order with each node's
original source map. A set result that would span owners is rejected until that
same stable multi-document order exists; cross-owner intersection or difference
still succeeds when its result is empty or belongs to one owner. The optional,
deprecated namespace axis remains an explicit host-language omission.

Native unary `+`/`-` and binary `+`, `-`, `*`, `div`, `idiv`, and `mod`
execute for the supported numeric types, with direct atomization, untyped
conversion, numeric promotion, empty-operand propagation and typed errors.
Integer/decimal operations use the exact coefficient/scale kernel; decimal
division uses its fixed precision policy. Integer `to` ranges also execute,
including empty/reversed bounds, and require an explicit `xpathItems` limit
before materialization. String concatenation `||` executes with the shared
atomization, conversion and text/work limits. These are native evaluation
capabilities, not merely parsed or serialized AST forms.

Typed simple-map expressions execute each mapping path once per item from the
previous stage with that stage's item, position, and size focus. Stage results
are concatenated left to right without the node-only checks, identity
deduplication, or document sorting required by `/`, so atomic values, native
nodes, duplicates, and input order remain intact. An empty stage skips every
remaining mapping path. Operand diagnostics retain their exact path ranges,
while evaluated intermediate and final sequences enforce `xpathItems` budgets;
the evaluator never serializes or reconstructs a mapped item.
Native function calls resolve one supported signature by expanded QName and
arity before evaluating arguments. The dispatcher executes the focus functions
`fn:position()` and `fn:last()` plus the pure sequence functions `fn:count()`,
`fn:exists()`, `fn:empty()`, `fn:boolean()`, and `fn:not()`, preserving native
focus, effective-boolean-value rules, exact result and argument source maps,
namespace isolation, deterministic unsupported-signature diagnostics, and
evaluated-work budgets. The same dispatcher executes zero-argument `fn:true()`
and `fn:false()` as focus-independent typed constants with full-call source
maps. Named arrow expressions execute through the dispatcher after canonical
lowering, including left-to-right chains and normal operator precedence; an
arrow into a zero-argument constant fails arity resolution before its inserted
operand runs. Dynamic arrow specifiers use the typed postfix-call path: maps and arrays
accept one key argument. Retained native inline functions also support dynamic
calls, as described below.

The existing accessor/numeric dispatcher also executes these signatures:

| Functions | Arities | Native scope |
| --- | --- | --- |
| `fn:string`, `fn:data`, `fn:number` | 0, 1 | Context/default arguments, retained-node access and the supported atomic conversions |
| `fn:abs`, `fn:ceiling`, `fn:floor` | 1 | Optional numeric arguments with typed results |
| `fn:round`, `fn:round-half-to-even` | 1, 2 | Numeric rounding with optional integer precision |
| `fn:format-integer`, `fn:format-number` | 2, 3 | Existing CEM numbering policy and typed static-context decimal formats |

Their native regression coverage does not imply a browser demo for every
signature or every formatting policy. Demo coverage is recorded per slice in
`docs/todo.md`; compiled artifact support and host bindings are documented
separately below. Other atomic families and broader standard-library/QT3
coverage remain incomplete.

The non-regex text slice implements `fn:normalize-space` and `fn:string-length`
with zero or one argument, `fn:string-join` with one or two arguments, and
one-argument `fn:tokenize`. Normalization/tokenization recognize only XML
whitespace (space, tab, CR, LF); non-breaking and other Unicode spaces remain
text. Length counts Unicode codepoints. Joining preserves order, duplicates,
and empty members, atomizes native nodes, and uses canonical supported atomic
string conversions. Optional empty input yields an empty string, zero length,
or no tokens; zero-argument calls use the context item's string value.
Incorrect argument types/cardinality retain the argument range. These contracts follow
[Functions and Operators 3.1](https://www.w3.org/TR/xpath-functions-31/).

The initial regex subset implements `fn:matches` at arities two/three,
`fn:replace` at three/four, and `fn:tokenize` at two/three. It validates
XPath syntax before translating it to a bounded NFA; authored Rust regex
syntax is never accepted implicitly. See
[F&O regex rules](https://www.w3.org/TR/xpath-functions-31/#regex-syntax) and
[the engine API](https://docs.rs/regex-automata/0.4.14/regex_automata/nfa/thompson/pikevm/struct.PikeVM.html).

| Supported | Contract |
| --- | --- |
| Patterns | Unicode literals; `^`/`$` whole-string anchors; alternatives; capturing and `(?:...)` groups; positive/negative classes and literal ranges; greedy/reluctant `?`, `*`, `+`, `{n}`, `{n,}` and `{n,m}` |
| Escapes | XPath single-character escapes; `\s\S` use XML whitespace; `\d\D` use Unicode Nd; `\w\W` use XPath's P/Z/C category complement; `\p{...}`/`\P{...}` accept general-category names |
| Flags | `s` includes CR/LF in dot; `x` removes XML whitespace outside classes without introducing comments; `q` quotes pattern and replacement literally. Flags may repeat. `q` ignores `s/m/x` |
| Replace | Leftmost nonoverlapping matches; numbered captures including `$0`; unmatched/nonexistent groups are empty; excess capture-number digits become literal per XPath; only `\$` and `\\` escape replacement characters |
| Tokenize | Captures are not tokens; boundary/adjacent separators preserve empty tokens; empty input yields no tokens. Arity one keeps its XML-whitespace normalization behavior |

Unsupported valid constructs produce `cem.xpath.regex_unsupported`:
backreferences, class subtraction, XML name escapes, Unicode block escapes,
and `i`/`m` modes. The latter require XPath-specific Unicode case variants
and trailing-line boundary semantics; engine flags would change their meaning.
Invalid flags, patterns, empty-matching replace/tokenize patterns, and invalid
replacements report `FORX0001/2/3/4` respectively at the owning argument.
Engine-only lookarounds, inline flags, named groups, word boundaries, hex/octal
escapes and POSIX classes are invalid XPath syntax. Unsupported capabilities
and resource failures remain uncatchable as CEM-QL data errors.

Hard ceilings are 2048 pattern/flag bytes, 16 KiB input/replacement,
32 nested groups/captures,
1024 per counted repetition, and 128 KiB per compiled NFA. Compiled NFA
state/edge counts times remaining input bytes and capture slots are
charged before **every** search, including repeated suffix searches. The
counts do not depend on native/WASM pointer sizes. The per-call regex work
ceiling is 16 Mi units, in addition to shared XPath work,
text and item limits. Compilation/searches are bounded, non-interruptible
engine calls; operation control is checked before and after them. No
unbounded cache or regex program is serialized into portable XPath artifacts.
Output appends and token storage share existing limits.

The [validation demos](../../../../cem-elements/demo/xpath-validation.html)
combine lexical checks, guarded numeric casts and quantified rules. Regex alone
is not presented as complete form/address validation; the IPv4 example previews
an authored prefix-length allow-list, without testing subnet membership.

The native XML node slice implements `fn:local-name` and `fn:namespace-uri`
at arities zero/one, following [F&O 3.1 node functions](https://www.w3.org/TR/xpath-functions-31/#func-local-name).
Their optional argument is a node, without atomization; omitted arguments use
the context node. Results are `xs:string` and `xs:anyURI`, respectively, with
empty values for absent names/namespaces. Names come directly from retained
XML metadata, including attributes and processing-instruction targets. Argument
type/cardinality errors retain the argument range; missing context and shared
text/work limits remain errors. `fn:node-name` and QName values/comparisons
are not implemented by this slice. Local table/tree demos use CEM-QL sorting
on the original native nodes. The sorting gallery uses standard `fn:sort`, and
the HTTP loader binds retained CEM documents through the same node capability.

The sequence slice implements `fn:head`, `fn:tail`, `fn:reverse`, and
two/three-argument `fn:subsequence`. Selection preserves original items,
source maps and retained node owners; arrays, maps and function handles remain
opaque items. Subsequence bounds use `xs:double` function conversion (numeric
promotion or untyped casting), one-based positions, and rounding with ties
toward positive infinity. NaN and infinities follow the standard position
inequalities, including empty results for indeterminate bounds. These semantics
follow [F&O 3.1 sequence functions](https://www.w3.org/TR/xpath-functions-31/#func-subsequence).

`fn:distinct-values` atomizes nodes/arrays and supports the existing string,
untyped, URI, boolean, integer, decimal, float and double comparison matrix.
Untyped values compare as strings; numerics use existing exact comparison and
promotion, NaNs collapse, and signed zeros compare equal. Incomparable supported
types remain distinct. The optional second argument accepts only
`http://www.w3.org/2005/xpath-functions/collation/codepoint`; other collations
fail with `FOCH0002`, and unsupported atomic families remain explicit errors.
The implementation retains first encountered representatives, but the standard
does not guarantee order or representative choice. The XML table demo therefore
authors first-seen heading order explicitly in XPath, comparing node kind,
local name and namespace URI against earlier cells. Duplicate scans and their
text comparisons share the invocation work budget, bounding quadratic cases.

The numeric aggregate slice implements `fn:sum` at arities one/two, `fn:avg`
at arity one, and numeric `fn:min`/`fn:max` at arities one/two, following
[F&O aggregate functions](https://www.w3.org/TR/xpath-functions-31/#func-sum).
It atomizes native nodes/arrays, casts untyped values to double, and promotes
the complete numeric sequence before reduction. Integer/decimal sums remain
exact; averages preserve terminating decimal quotients and round repeating
quotients to 18 significant digits using the existing half-even division
policy. Minimum/maximum preserve the selected integer/decimal type unless
float/double promotion is required. NaNs propagate after argument validation;
equal extrema keep the first representative, including its zero sign.

Empty input returns integer zero for `sum`, or its optional atomic/empty
`$zero` argument unchanged; `avg`, `min` and `max` return empty. The explicit
zero does not affect nonempty sums. Numeric extrema ignore the optional
collation as required, after validating its string argument contract.
Nonnumeric extrema and duration arithmetic remain explicitly unsupported;
incompatible numeric/nonnumeric mixtures fail with `FORG0006`, invalid untyped
casts with `FORG0001`. Input, converted sequences, intermediate sums, results,
and reduction/division work share the existing limits. Decimal comparison also
handles zero separately from scale padding, preserving fractional ordering.
The aggregate demos validate and explicitly cast decimal lists and native XML
basket amounts before computing results; adding a fruit needs no new field
expression. Invalid data remains visible as an error, and ∅ denotes an absent
minimum, maximum or average.

Map constructors and square/curly array constructors execute natively. Square
arrays retain each member sequence, including empty and nested members; curly
arrays produce one member per item of their input sequence. Unary and postfix
lookup accept names, integers, parenthesized key expressions and wildcards.
Parenthesized keys use the outer focus. Array positions start at one; invalid
positions raise `FOAY0001`. Map/array function-call syntax takes one key argument.
The same dispatcher implements `map:contains`, `map:get`, `map:keys`, `array:size`
and `array:get`. Absent entries and empty-valued entries both return the empty
sequence on lookup, while `map:contains` distinguishes them. Constructor duplicate
keys raise `XQDY0137`. Key matching follows [op:same-key](https://www.w3.org/TR/xpath-functions-31/#func-same-key)
for the supported atomic matrix: strings/untyped/URIs compare by codepoint,
booleans by value, and numeric keys by exact mathematical value without lossy
promotion, with NaN, infinities and signed zero handled explicitly. Map keys are
returned in insertion order here; authors must not depend on that order portably.
Other atomic families, broader map/array functions and higher-order type matching
remain unsupported. Nested values retain native XML owners and item origins.
Container traversal, copies, comparisons and empty member visits share the work
counter; atomic text inside containers shares the byte bound. `xpathItems` still
counts each materialized sequence, not container membership as a flattened list.

CEMT XPath bodies may use the existing triple-backtick rich-content fence to
protect constructor braces. The compiler parses the original fence-body bytes
and retains their physical source coordinates; compiled lookup execution does
not reparse key expressions. Named-function results can retain opaque maps and
arrays across CEM-QL calls. Scalar IP-filter and native XML basket demos exercise
this route. Imported CEM trees from XML, JSON, YAML and CSV now share the same
native node view, including the explicit standard JSON-to-XML mapping. Native
HTTP responses use retained CEM owners as well; standard JSON parsing functions
remain separate.

Inline `function($parameter as type) as type { body }` expressions retain their
lexical variable bindings and typed program owners. Types are optional and default
to `item()*`; the existing primitive atomic, generic item and unconstrained node
sequence types support function conversion, cardinality and result checks. Names
are expanded before duplicate-parameter checking. Captures respect lexical
shadowing, retain native CEM node identities and do not inherit the caller's
focus. Dynamic calls and dynamic arrows share the invocation's work/text/item
limits, a hard ceiling of 32 nested inline calls, and 32 active expression
frames while an inline body executes. CEM-QL cannot accept or
return executable function values, including functions nested in maps/arrays.
Explicit result export contains metadata only; it cannot recover executable
closures or captured trees. Named references, partial application, function
lookup and higher-order sequence-type coercion remain unsupported.

`fn:sort` at arities one, two and three uses stable lexicographic ordering of
atomized keys, with numeric, boolean and string/untyped/URI comparisons and
Unicode codepoint collation. The key may be a retained inline function or an
existing callable map/array. Missing collation uses the codepoint default;
unknown collations raise `FOCH0002`. Empty keys precede nonempty keys; NaNs precede
other numeric values and tie with each other. Incomparable keys raise `XPTY0004`.
Keys are evaluated once and share cumulative storage/work bounds; merging moves
the original source items and preserves node identity and source maps. The
[sorting demos](../../../../cem-elements/demo/xpath-sort.html) explicitly author
missing/invalid-last and direction policies. `fn:sort` does not inherit
`seq:sorted` policy or implement `xsl:sort` options.

`XPathEvaluationLimits` separates item, text, and work bounds. Defaults allow
1 MiB of UTF-8 atomic lexical bytes per materialized sequence/string and
16,777,216 work units per evaluator invocation. Intermediate values, bindings,
node atomization, string conversion/concatenation, casts, and numeric formatting
share these bounds with the new text functions. The work counter charges
expression steps and text visits/copies, including bounded formatting passes;
it is deterministic accounting, not elapsed CPU time. Nested expressions share
one counter. Native XML owners are retained without charging their document
size as output text; extracting their string/typed values is bounded while
traversing the original events. The byte limit is neither total heap accounting
nor a cumulative allocation counter. Operation control still supplies
cancellation/deadline checks.

Standalone scopes accept `xpathTextBytes` and `xpathWorkUnits` alongside
`xpathItems`; query execution accepts the same names (`queryWork` remains a
work alias). Rust hosts can set all three limits directly; `None` explicitly
opts out on the low-level evaluator. Host adapters use bounded defaults for
omitted text/work fields. Result safety stamps include each enabled bound.
Failures use `cem.xpath.text_byte_limit_exceeded` and
`cem.xpath.work_limit_exceeded` with the expression's original source map.
They publish no partial result and cannot become `castable=false` or a caught
CEM-QL data error. CEM-QL libraries can receive host-owned per-call limits via
`install_with_limits`; portable libraries do not grant themselves larger limits.

Typed `instance of` and `treat as` expressions follow their grammar precedence
between arrow expressions and set operators. Their CEM-owned sequence types
match cardinality, supported native atomic identity and subtype relationships,
generic items, and retained XML node kinds directly. `instance of` returns a
typed boolean with the full expression source map; a successful `treat as`
returns the original items without changing source maps or native owners, while
a mismatch reports the expression range. Unsupported atomic types,
constrained or schema-aware kind tests, and function/map/array types fail before
operand evaluation. Matching never parses the derived `sequence_type` display
string from a result artifact.

Typed `cast as` and `castable as` expressions use a closed native conversion
matrix for `xs:untypedAtomic`, `xs:string`, `xs:boolean`, `xs:integer`,
`xs:decimal`, `xs:float`, `xs:double`, and `xs:anyURI`. Targets resolve before
operand evaluation. Retained nodes atomize directly; empty operands follow the
single type's optional indicator; multi-item operands fail conversion; numeric
values preserve unbounded integer/decimal behavior and deterministic exact
binary-float-to-decimal conversion; and string-derived values follow XML
whitespace and supported lexical rules. `cast as` returns a newly typed atomic
item with the full expression source map or reports the full expression range.
`castable as` returns false for cardinality or conversion failure, while operand
evaluation and atomization errors remain errors. Abstract, derived,
namespace-sensitive, schema-defined, list, and union targets remain fail-closed,
and casting never parses result display strings or crosses a serialized
boundary.

The expanded-QName-and-arity function dispatcher also recognizes the matching
one-argument XML Schema constructor functions for those eight concrete atomic
types. Direct and named-arrow calls reuse the optional cast atomization and
conversion kernel, so empty arguments return empty, retained nodes atomize
without projection, and exact numeric, lexical, cardinality, diagnostic, and
full-call source-map behavior stays identical to `cast as Target?`. Unsupported
constructor names and arities resolve before argument evaluation. Abstract,
derived, namespace-sensitive, schema-defined, list, and union constructors
remain fail-closed.

Typed `for` expressions accept one or more comma-separated bindings. The parser
lowers later bindings into nested typed `For` nodes with dependent lexical
scope, preserving the complete source range on the outer node and each
`$binding`-through-return suffix on its inner node. Evaluation concatenates
results in Cartesian binding order without replacing the outer focus and
enforces the cumulative sequence-item budget. Empty binding sequences skip all
dependent bindings and the return clause; lexical shadowing, native node owners,
item source maps, and diagnostics remain intact. Typed `let` expressions use the
same nested source-range convention for comma-separated bindings, but bind each
complete sequence once and evaluate each return once, including after an empty
binding. Later bindings see earlier sequences while lexical shadowing, outer
focus, native owners, item source maps, exact diagnostics, and sequence-item
budgets remain intact. Typed conditional expressions retain an owned expression
sequence for the condition and typed nodes for both branches. Evaluation applies
the native effective-boolean-value rules once, evaluates exactly one branch with
the unchanged focus and bindings, and preserves its native owners, item source
maps, exact diagnostics, and sequence-item budget behavior; unselected work is
not evaluated. Typed `some` and `every` expressions lower comma-separated
bindings into nested same-quantifier nodes with dependent lexical scope and
exact outer and binding-suffix ranges. Evaluation binds each item as a retained
singleton, applies native effective-boolean-value rules to required `satisfies`
tuples, short-circuits decisive results, and preserves vacuous empty-binding
truth, outer focus, shadowing, native owners, exact diagnostics, and
evaluated-work sequence-item budgets. The XPath 3.1 control-flow expression
slice is otherwise executable; XQuery-only switch and typeswitch inputs remain
explicit fail-closed exclusions. Other atomic families, remaining functions,
other constructor functions, named function references and partial application
fail with a stable schema-owned diagnostic. The evaluator does not read expression source
text, project through CEMT or JSON, reparse XML, or construct a replacement
tree.

The standalone executable adapter is registered for XPath template identities.
It compiles template source once at the lifecycle/compile boundary, evaluates a
primary lifecycle-owned XML document AST as the context item, and returns the
typed `XPathResultArtifact`. The `transform` command invokes the registered JSON
result exporter only after native evaluation completes. Parameters, named
entrypoints, secondary inputs, and non-XML input AST families are rejected until
their context and XDM binding contracts are defined.

Host invocation now uses one typed request contract that names the host
language, carries the already parsed `XPathExpressionAst`, keeps the native
context item separate from variables, and keys variable sequences by expanded
namespace URI plus local name. CEMT, CEM-QL, and XSLT adapters each accept only
an AST attached to their typed owner kind, pass native node and atomic sequences
directly to the evaluator, and preserve resolver, safety, owner, and source-map
identity. The CEM transform schema owns a dedicated authored `xpath`
function-body form. The CEM-QL schema owns an explicit `slot-kind=xpath`
programmatic expression slot, and the CEM-QL crate compiles its lexical island
once before invocation. XSLT consumes the XPath ASTs already fused into direct
expression attributes and AVT segments. None of these runtime adapters parses
an expression string or maps a CEMT, CEM-QL `ItemStream`, or JSON value into
XDM.

`XPathDynamicContext` accepts optional `context_position` and `context_size`
as `u64` invocation metadata. Supply both alongside a context item, with
`1 <= position <= size`. Omitting both preserves singleton focus (`1/1`) for
a present item and absent focus otherwise. Partial coordinates, zero values,
position greater than size, or coordinates without an item fail before
expression execution with `cem.xpath.focus_invalid`, retaining the expression's
source location. These are host-contract errors, not a new XPath language error.
Positions and sizes remain exact `xs:integer` values on native and WASM targets;
the size does not allocate a sequence or replace existing item/work limits.

Predicates, path steps and simple maps establish their own focus and restore
the caller's focus afterward. Inline functions still have absent focus;
authors can explicitly capture `position()`/`last()` results in variables.
The new fields are runtime-only: artifact formats and hashes are unchanged,
and reloaded programs accept a different focus on every call. The native XSLT
adapter now accepts this contract; stylesheet-loop lowering and bundle/WASM
binding remain separate pending work. Unknown streaming size is not supported
by this materialized-focus contract. See
[XPath dynamic context](https://www.w3.org/TR/xpath-31/#id-xq-evaluation-context-components).

The result media type is intentionally distinct from expression source and does
not enter the XPath source parser. XML, JSON, CEM, and text serialization remain
explicit downstream conversion edges. CEMT has both the typed adapter boundary
and the authored schema form. Its explicit host-selected dispatcher resolves
one compiled XPath body by exact function name, consumes only the native XDM
binding arena, and returns the typed XPath artifact body. CEM-QL exposes the
same native boundary through its schema-owned XPath slot API; XSLT exposes it
for fused attribute and AVT ASTs. No adapter infers renderer input aliases or
introduces a generic-value or serialization bridge.

## Formatter And Colorizer Profiles

The package registers `compact`, `pretty`, and `tabular` formatters plus
`terminal`, `html`, and `md` colorizers. In this foundation slice all formatter
profiles are intentionally lexical-lossless aliases. Reflow is deferred until
grammar-node boundaries and embedded expression source maps are exercised by
the lifecycle output pipeline.

## Safety

Parsing performs no I/O and does not evaluate expressions. Evaluator capability
validation requires the package-owned AST, deterministic native and WASM
results, item-origin source maps, all XPath 3.1 item kinds, and CEM resolver-only
resource access. Functions such as `doc()`, `collection()`, and
`unparsed-text()` cannot receive a direct filesystem or network boundary.
Current time, timezone, environment variables, randomness, recursion,
cancellation, and work budgets must also be explicit request capabilities; the
evaluator cannot read ambient process or host state.

## Verification

`yarn nx run cem_ml_schema_package_xpath_v1:verify` validates the schema-package
manifest and fixture expectations, runs lossless lexer/parser, schema-diagnostic
handoff, lifecycle loading, no-fallback validation, and host-attachment tests,
verifies the full-destination conformance matrix, native evaluator owner/path,
scalar, dependent `for` and full-sequence `let` binding, logical-EBV, and
short-circuit semantics,
standalone transform routing,
mixed result artifacts and evaluator capability rejection, verifies embedded
catalog identity, and checks that README examples use fenced XPath source with
no SVG fallback. It also checks deterministic compiled programs, typed
source-free reload against changed native owners, malformed binary input and
bounded decoding.
`yarn nx run cem_ml_schema_package_xpath_v1:verify:compiled-artifacts` rebuilds
the shared WASM host and checks native/WASM byte equality, XSLT-owned program
loading, identity/hash rejection, retention limits, disposal and capability
isolation.
`yarn nx run cem_ml:build:wasm` verifies that the CEM-owned scanner and parser
remain compatible with the browser WASM target.
`yarn nx run cem_ql:test` verifies that CEM-QL XPath slots compile once, retain
CEM-QL ownership and source ranges, invoke native XDM bindings, preserve result
node identity, and reject non-CEM-QL AST owners and runtime value bridges.

## Tracked Incomplete Work

- Implement a CEM-owned XPath 3.1 compiler/evaluator and prove native/WASM AST
  consumption through CEM-only resolver and safety capabilities.
- Define grammar-aware formatting before making profile output differ.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-path</summary>

- Source: [`examples/basic-path.xpath`](./examples/basic-path.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `pass`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/book[@lang = "en"]/title
```

<details>
<summary>functions-and-variables</summary>

- Source: [`examples/functions-and-variables.xpath`](./examples/functions-and-variables.xpath)
- Content type: `text/xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `pass`
- README rendering: fenced `xpath` source

</details>

```xpath
for $book in /catalog/book
return normalize-space($book/title)
```

<details>
<summary>maps-arrays-and-comments</summary>

- Source: [`examples/maps-arrays-and-comments.xpath`](./examples/maps-arrays-and-comments.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `pass`
- README rendering: fenced `xpath` source

</details>

```xpath
(: Preserve nested comments (: including inner trivia :). :)
map {
    "titles": array { /catalog/book/title/string() },
    "count": count(/catalog/book)
}
```

<details>
<summary>unicode-qname</summary>

- Source: [`examples/unicode-qname.xpath`](./examples/unicode-qname.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `pass`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/π/@γλώσσα
```

<details>
<summary>explicit-axes-and-escaped-string</summary>

- Source: [`examples/explicit-axes-and-escaped-string.xpath`](./examples/explicit-axes-and-escaped-string.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `pass`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/descendant::book[@title = "The ""Quoted"" Book"]/ancestor-or-self::node()
```

<details>
<summary>unknown-prefix</summary>

- Source: [`examples/unknown-prefix.xpath`](./examples/unknown-prefix.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xpath.unknown_namespace_prefix`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/ns:book
```

<details>
<summary>invalid-token</summary>

- Source: [`examples/invalid-token.xpath`](./examples/invalid-token.xpath)
- Content type: `text/xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xpath.lexical_error`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/`book
```

<details>
<summary>mismatched-delimiter</summary>

- Source: [`examples/mismatched-delimiter.xpath`](./examples/mismatched-delimiter.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xpath.parse_error`, `cem.xpath.mismatched_delimiter`, `cem.xpath.unclosed_delimiter`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/book[1)
```

<details>
<summary>external-resource-denied</summary>

- Source: [`examples/external-resource-denied.xpath`](./examples/external-resource-denied.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xpath.external_resource_denied`
- README rendering: fenced `xpath` source

</details>

```xpath
doc("catalog.xml")/catalog
```

<details>
<summary>invalid-unclosed-predicate</summary>

- Source: [`examples/invalid-unclosed-predicate.xpath`](./examples/invalid-unclosed-predicate.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xpath.parse_error`, `cem.xpath.unclosed_delimiter`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/book[1
```

## Imported CEM trees

`XPathNativeNode::cem_document` and `cem_node` accept an
`Arc<RetainedCemTree>` from `cem_ml::import` or a native CEM producer. All node
navigation, values, comparisons and bounded text traversal use the common
semantic tree. The XML compatibility constructors delegate to import.
`owner()` now returns the retained CEM tree; `source_owner()` exposes a retained
lifecycle parser owner when available. Native CEM producers need no external
format owner. The lifecycle query entrypoint also imports XML/JSON/YAML/CSV
through the shared boundary before invoking XPath.

Source-oriented CEM fields stay available alongside import-supplied semantic
values. XML import normalizes literal line endings and attribute whitespace
before decoding/merging references; decoded JSON/YAML/CSV values are not subjected
to XML lexical rules. The source-format boundary is enforced by the
[import principle](../../../../../docs/cem-data-import-principle.md) and native
cross-format fixtures. Standard `fn:parse-json`/`fn:json-to-xml` functions are
separate API work; importing a tree does not implement them.
