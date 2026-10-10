# Native datatype and function scalar composition

Status: adopted 2026-10-09 through [the archived adoption checklist](archive/todo-snapshot-2026-10-10.md#5-integrate-schema-validation-and-construct-reuse).
This is the maintained contract for single-declaration composition sites. The
[datatype design](cem-datatype-compilation-design.md) governs datatype semantics;
the [reference design](cem-ql-cem-ml-node-references-design.md) governs retained
references, lifecycle resolution and scope grants. This document adds the function
consumer contract without changing those boundaries.

## Scope and implementation status

Here, *scalar composition* means that one authored slot selects one declaration.
It does not require that declaration to validate scalar data: a selected datatype
may describe a list or a native node sequence. Selection cardinality and the
selected contract's input cardinality are independent.

The registered datatype compiler already consumes attribute `@type`, datatype
`@base` and native datatype `@rule`. Passive function selection is implemented and
verified through the native ML and QL hosts. Checked bindings now support
validation (including diagnostic-only results), conversion, equality and constant
preparation through explicit host registration. Shared compilation/invocation
limits and coordinated package replacement are verified. The opt-in package
compiler now assembles admitted behavior collections and registers checked
function bindings before datatype and attribute activation. The existing
diagnostic behavior compiler supports literal function reuse; source-only native
function slots remain incomplete until an executable consumer is installed.

| Original scalar site | Literal meaning | Explicit native target | Availability |
| --- | --- | --- | --- |
| Attribute declaration `@type` | Datatype local name or QName | Exactly one named original `{type}` | Implemented by the registered datatype compiler |
| Datatype `@base`, except explicit list kind | Inherited datatype local name or QName | Exactly one named original `{type}` | Implemented; omitted kind inherits the base kind |
| Datatype `@base` with `kind="list"` | Item datatype local name or QName | Exactly one named original `{type}` describing the supported item representation | Implemented; this is an item edge, not whole-list inheritance |
| Datatype `@rule` | Existing descriptive rule text, including name-like text | Exactly one original `{behavior}` admitted as a registered datatype validation rule | Implemented; selecting a function directly is invalid |
| Behavior `@function`, with `implementation="function"` | Existing inline/local/qualified function lookup under the applicable profile | Exactly one named original `{function}` owned by an admitted behavior | Selection, checked profile binding and opt-in coordinated activation implemented |

The rows define consumer roles, not new schema datatype names or global rules for
attributes with the same spelling. Element `@base` retains its implemented
`element-base` contract. Function parameter `@type`, function `@returns`, behavior
input/result/detail type metadata and shared value-contract field `@type` keep
their existing literal signature vocabularies. They do not acquire native
datatype or record-contract selection through this adoption. Diagnostic and
constraint `@behavior` and reference normalizer/operator strings also keep their
existing consumer contracts. There is no new function alias, pipeline, partial
application, overload, function inheritance or typed prelude syntax.

## Slot admission and singleton selection

A native slot contains one explicit reference constructor in the retained AST,
using the existing `@field={#expression}` form. Parsing retains that occurrence;
the designated consumer evaluates it later with supplied context. A quoted
string resembling a query or reference remains a literal. An ordinary expression,
inline declaration, scalar result, mixed text/native payload or missing native
constructor cannot stand in for this reference slot.

The shared resolver may follow intermediate references under the same consumer
role. A complete terminal selection must contain exactly one declaration of the
expected expanded metamodel name, with the required literal nonempty local name
and original declaring context. Matching only a local name or an object shape is
insufficient. Two occurrences of the same target are still two results: singleton
selection does not deduplicate or choose the first/last result. A complete empty
selection is a cardinality error; a pending selection has not proved emptiness.
Any known wrong target is an error even when another branch is pending.

An absent optional slot retains that site's existing meaning. It is distinct from
an authored native slot that selects nothing. Existing scalar declaration
precedence and duplicate-field rules continue to apply; native selection adds no
merging or fallback. Datatype duplicate fields remain invalid. Function admission
must reject malformed or duplicate binding/signature fields rather than silently
dropping them through a string-only accessor.

## Names, visibility and retained identity

Datatype literals retain the original slot's captured namespace names, declaring
schema aliases and explicit exports. Native and literal datatype dependencies
converge on the same original descriptor. Descriptive literal `@rule` text never
performs behavior lookup or selects an implementation by spelling. Its semantics
must be supplied by the explicit datatype registration as in the datatype design.

Preserve the existing literal function search order: a matching function in the
owning behavior takes precedence; otherwise search reusable functions in that
behavior's declaring schema. A qualified name uses its declaring `uses` binding
and the explicitly available schema registry. Multiple visible reusable matches
are ambiguous, not an overload set. Existing source-only alias compatibility
remains available on that path. The new retained consumer uses original lexical
bindings; it must not borrow a consuming schema's conflicting or missing aliases,
scan unrelated retained documents, load a URL or infer exports from names.

Native selection identifies a declaration directly, but still checks function
visibility relative to the original behavior and schema owners:

| Function visibility | Admitted caller |
| --- | --- |
| Absent or `private` | Its own declaring behavior |
| `package` | A behavior in the same declaring schema identity |
| `public` | An explicitly admitted caller, including another schema |

Unknown visibility is invalid. Visibility and directed scope grants are separate
checks; `public` alone grants no crossing, and a crossing grant cannot expose a
private function. Reusing a behavior preserves its declaring ownership. Equal
namespace strings on different retained sources do not by themselves establish
package membership; the host must admit their schema identity explicitly.

A selected function retains its original owner, declaring behavior/schema,
parameter and body nodes, source frames and lexical/module bindings. Dependencies
of that function are resolved in its own declaration context. Selection does not
import the enclosing behavior's inputs, defaults or result policy into the caller,
nor import a whole schema. Required source, exports and execution capabilities
must be supplied explicitly. Compiled query IR may be cached with its original
source attribution; declarations are never copied into a consuming AST or passed
through JSON to establish identity.

## Datatype composition

Base, item and rule edges retain their roles throughout selection. Restrictions
intersect in the order required by the datatype compiler; a selected rule cannot
replace a base rule or widen a base vocabulary. Kind/representation compatibility,
lexical preparation, facets, defaults and original diagnostic bindings must all
complete before an attribute consumer becomes ready. Unsupported declarations
must not silently lose fields, rules or restrictions.

List `@base` validates each item using the supported scalar item descriptor.
Whole-list inheritance, list-of-list/native-node items and inherited capability
replacement remain deferred under the datatype design. A node datatype receives
the original ordered target sequence from its consumer; it neither resolves
descendant references automatically nor converts nodes to strings.

A native `@rule` ends datatype dependency traversal at a behavior. The behavior
adapter then checks its execution placement, typed signature and exact original
registration. This separate stage can consume a checked `@function` binding;
the datatype walker must not traverse arbitrary behavior children as type edges.
Validation remains separate from conversion, equality and constant preparation.
Each capability requires its own registered profile and checked result contract.

## Function binding and invocation

The new function consumer first assembles admitted behavior declarations and
their original function members, then binds each `@function` occurrence. Forward
literal names are resolved after assembly. An incomplete relevant collection
cannot prove a local function absent. A native target may come from an explicitly
admitted retained source without inserting its enclosing behavior into the
consumer's collection, but the compiler must establish original membership and
lexical ownership before using it.

For `implementation="function"`, either lookup form yields one checked function
descriptor. Unrelated inline helpers are not alternative implementations, and a
failed selection never falls back to an inline name. Existing profiles with one
matching inline function remain compatible. For `implementation="engine"`, the native
primitive registration remains the implementation binding; a competing
`@function` or function implementation is invalid under the existing profile.

The descriptor records the selecting attribute, selected function and owner,
declaring bindings, signature, body/implementation identity, dependencies and
readiness. A registration authorizes the applicable behavior, exact selected
function and signature together. A public declaration, matching name, body text,
`trusted` flag or apparently compatible returned record does not register code.
A changed selected owner or function requires a new checked binding, even if its
name and body text are equal. Source-only compilation retains the native slot as
unsupported/incomplete until its explicit consumer is installed; it must not
silently treat the slot as absent and activate a literal fallback.

The retained implementation exposes `compile_selected` on the validation,
conversion, equality and constant behavior contracts. It accepts a completed
`FunctionSelection` and the finite selection budget; public resolution metadata
cannot manufacture a binding from an incomplete selection. Signature and body
inspection charge that budget, and the selected function's original source owns
its parameter names and query body. Existing explicit query registrations compile
only the profile's fixed roles and retain the original body frames in query IR.
Unsupported signatures and query dependencies fail compilation. Authored and
reused native slots keep source-only readiness guards. The opt-in
`CemQlSchemaPackageCompiler::with_scalar_datatypes` stage consumes a host-supplied
`ScalarPackagePlan`: original sources, explicit exports, exact behavior/profile
admissions, datatype roots and primitive registrations. It assembles collections,
selects each admitted function, checks its profile and registers its query.
Completed registrations satisfy only guards belonging to the same original
behavior owner. The package model retains the immutable `FunctionBindings`
snapshot alongside datatype descriptors. Missing admissions leave native guards
pending; source labels never create authority. Failed function attempts do not
invoke datatype constant preparation through partial or older registrations.

Before invocation, check parameter names, arity, representation, requiredness,
sequence cardinality and result contract against the calling behavior profile.
Defaults, nullable values and empty sequences retain their existing meanings;
the selection contract adds no coercion or automatic datatype conversion. A
signature field not supported by the chosen profile is a contract error, not
metadata the adapter may ignore. A required candidate cannot be fabricated.

Datatype validation/conversion profiles retain the fixed `value`, `datatype`,
`candidate` roles; equality retains `left`, `right`, `datatype`; constant
preparation retains its existing `value`, `datatype`, `candidate` roles. Role
values are supplied by the invocation and never inferred from arbitrary parameter
names or captured from ambient bindings. The selected function must satisfy the
specific caller's result contract. Diagnostic-only behaviors keep their own
contract; using one as a validator requires the existing explicit acceptance
mapping. Signature compatibility alone grants no extra native navigation.

Returned values pass the registered result adapter even when static checks
succeed. Missing or mistyped fields and invalid cardinality are execution
failures. A validation result's `accepted` boolean determines validity;
diagnostic severity or absence cannot replace it. Failed invocation cannot
become success, an empty result, or a fallback implementation.

Ordinary CEM-QL calls and imported function aliases keep their module contracts.
The separate host API `native:call` keeps exact identifier/arity registration and
operation control. A schema function reference neither installs that registry
nor becomes a query function item. Any such calls made by an admitted function
still need their independently supplied capabilities.

## Bounds, recursion and readiness

One compilation attempt owns a finite cumulative allowance for all requested
scalar sites, dependency edges and signature/body checks. Every selection also
obeys the shared resolver's request and destination depth/work limits, active-link
identities, unresolved policy and directed grants. Neither a literal alias nor a
transition from datatype to behavior/function binding resets the allowance.
Candidate source declarations cannot raise the active limits.

The explicit host path shares a `ScalarCompilationBudget` (also exposed under
the compatible `FunctionSelectionBudget` name) through collection, selection,
signature/body inspection and `compile_datatypes_with_budget`. Datatype traversal
and descriptor compilation debit the remaining allowance, including failed work;
later function binding cannot regain spent work. The older datatype-only entry
points retain their own bounded attempt. Supplying a lifecycle runtime also checks
cancellation before datatype traversal and during descriptor compilation.

Detect dependency cycles using original declaration/occurrence identities and
consumer roles, not names. Datatype inheritance/item cycles, reference cycles and
cyclic binding dependencies cannot establish readiness. Reuse in a diamond graph
is permitted after the shared dependency completes; a visited declaration is not
automatically a cycle. Do not recursively expand or clone source declarations.

During validation, rules, list items, function calls and any explicitly supported
nested validation share the enclosing operation control and remaining work,
call-depth, result and diagnostic limits. Already supported recursive query calls
retain the evaluator's policy; this adoption creates no new recursion permission.
Native callbacks must cooperate with those controls and charge nested work,
rather than starting fresh unlimited validation requests. Cancellation and budget
exhaustion cannot be caught and converted into successful validation. Signature
checking itself never invokes user datatype rules recursively.

Datatype invocation establishes one runtime `QueryExecutionBudget` and retains it
through rules, list items, conversion, preparation and comparisons. Cumulative
query work and simultaneous call depth survive query reentry; per-stage item and
closure-size checks keep their existing local meanings. Native query callbacks
receive that same budget for cooperative reentry. The first budget failure keeps
its original query diagnostic and prevents outer acceptance, including through
query catch blocks and callbacks that return successful values. Independent
invocations receive fresh budgets unless the host explicitly supplies a shared
enclosing budget.

| Outcome | Consumer action |
| --- | --- |
| Complete singleton, valid dependencies, compatible registered implementation | Bind a ready descriptor; invoke only at the consumer's execution stage |
| Complete empty/multiple selection, wrong target, invalid declaration/signature or ambiguous literal lookup | Report an attributed contract error; prevent activation |
| Pending context/name/source, unavailable registration, denied crossing, cycle, cancellation or exhausted budget | Preserve the specific failure/dependency state and available sources; prevent activation |
| Complete execution with `accepted=false` | Report completed invalid data, independently of diagnostic severity |
| Incomplete or malformed execution | Report execution incompleteness/failure; never report successful validation |

Diagnostics identify the original selecting slot and relevant target, dependency
or call-site source. Keep evaluator diagnostics, original owner/source maps and
the distinction between invalid data and unavailable execution. Warning/ignore
disposition does not make an incomplete binding ready.

Prepare replacements off the active package snapshot. Publish schema, function
bindings, datatype descriptors, converters and artifacts together only when all
required consumers are ready and no hard errors remain. A failed or pending
candidate stays inspectable while the previous complete package remains active.
Retry uses the same retained sources with a fresh invocation context and current
grants. Never save selected targets on authored reference nodes or reuse a binding
across changed contexts/owners merely because the scalar text is unchanged.

## Required verification before enabling function sites

Implemented 2026-10-09: `schema::function_references::FunctionCatalog` collects
direct functions from explicitly supplied original `ValueContractSource` schemas.
It keeps declaring bindings, original membership and source owners; external
literal names require explicit public exports. `select` uses the shared resolver
for native references and symbolic literal links, so both obey directed grants
and traversal policies. A `FunctionSelectionBudget` bounds collection, lookup
and repeated selection work. Unresolved collection references remain incomplete;
`FunctionCatalog::assemble` resolves admitted collection references under the
same host policies before forward literal lookup. Reused functions retain their
original behavior/schema membership, visibility and lexical bindings. All supplied
source collections must be complete for automatic registration.

`FunctionSelection::target()` exposes only a complete, visible singleton. It is
not an executable descriptor or a package-readiness claim. The caller must still
bind the appropriate registered signature/profile, preserve shared limits across
the full compilation/invocation graph, and coordinate activation. Checked binding,
shared limits, source-only readiness guards and opt-in package assembly and
registration are implemented. Datatype-only compiler hooks retain their existing
behavior; automatic function admission requires the scalar package hook.

The [ML fixtures](../packages/cem_ml/tests/function_references.rs) cover retained
selection, names/visibility, malformed slots, explicit exports, partial failures,
shared selection work and cycles. The [QL fixtures](../packages/cem_ql/tests/function_references.rs)
use the production query host to verify grants, source ownership, fresh context
selection, scalar rejection and untouched authored targets.

The [composition limit fixtures](../packages/cem_ql/tests/datatype_validation/function_limits.rs)
cover cumulative compilation and invocation work, datatype diamonds versus
cycles, list items, cancellation and original failures. The
[query call fixtures](../packages/cem_ql/tests/call_budgets.rs) cover completed
recursion, active-depth limits, native reentry and unsuppressible budget failures.
The [explicit package fixture](../packages/cem_ml_transform_cem_ql/tests/schema_package_lifecycle/functions.rs)
keeps the old schema, executable bindings, converters and artifacts active across
pending, denied and invalid function replacements; retry selects the new original
function from the same retained candidate. It uses an explicit compiler hook and
does not itself activate source-only native function slots. The automatic package
fixtures in the same suite additionally cover authored/reused native slots,
existing inline literals, all five profiles, missing admissions, grants, bounded
work, malformed signatures, source attribution and atomic retained-source retry.
The QL function fixtures cover forward lookup through completed behavior
collections and rejection of wrong collection targets.

The completed fixture and implementation actions are preserved in
[archived checklist](archive/todo-snapshot-2026-10-10.md#5-integrate-schema-validation-and-construct-reuse). Required coverage includes:

- Complete zero/one/multiple selections, repeated identical targets, intermediate
  references, wrong-kind/unnamed targets and malformed slot payloads.
- Literal inline, local and qualified lookup; shadowing and ambiguity; all three
  visibility levels; original aliases under conflicting consumer bindings;
  explicit exports and grants; foreign owners and equal-name replacements.
- All registered datatype function profiles and diagnostic compatibility; exact
  roles, candidate absence/cardinality, malformed signatures/results and missing
  implementations; no implicit conversion or string reference construction.
- Shared bounds across many sites and nested datatype/function work, dependency
  cycles versus diamond reuse, supported recursive calls, cancellation and
  preserved attribution on failures.
- Source-only guarding, incomplete collections, retained-source retries and
  coordinated package replacement preserving the previous active snapshot.

The native suites now exercise selection, checked signatures, invocation and
coordinated package activation. Typed prelude slots remain outside this scope;
their separate [contract](cem-typed-prelude-design.md) is implemented under
explicit 1.1 admission, with verification recorded in todo.md.
