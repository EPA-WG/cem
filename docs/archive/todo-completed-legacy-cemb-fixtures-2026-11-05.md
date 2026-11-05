# Genuine legacy CEMB fixtures — 2026-11-05

- [x] Repair the stale version-3 literal-attribute compatibility fixture using
  genuine historical encoder output, preserving its literal/empty-slot assertion.
- [x] Generate a frozen native v2/v3/v4 corpus with producer/source/hash evidence;
  replace the related positive header-relabeling fixtures while retaining exact
  reference ownership, edges, source frames and inspection checks.
- [x] Verify corrupt-hash, re-sealed truncation, count-limit and unsupported
  metadata/reference refusals in both native feature profiles.

## Baseline and cause

The original focused version-3 test reproduced `UnexpectedEof` with **one failed
case / four filtered out**. The unchanged reference suite separately reproduced
**9/11 pass**, with the v2 ordinary-body and v3/v4 cyclic-graph cases also failing
at `UnexpectedEof`. The earlier [conversion checkpoint](todo-completed-native-conversion-fixture-2026-11-05.md)
retains its broader **36/37** result in each feature profile. These are separate
invocations, not a combined before-run count.

All three positive fixtures encoded the current version-5 body, rewrote only its
header to an older version, then recomputed integrity. The version-5 source
metadata remained in the payload and was read as older chunk metadata. A valid
checksum cannot make that layout into a version-2/3/4 body. The production
decoder correctly rejected the malformed inputs.

## Historical corpus

The [fixture directory](../../packages/cem_ml/tests/fixtures/cemb-legacy/README.md)
contains **eight binaries totaling 4,130 bytes**:

| Wire version | Original native producer revision | Cases |
| ------------ | --------------------------------- | ----- |
| 2 | `75fe1fa664cc30e57a8b42fe057c110060b608f9` | Literal attribute; ordinary subtree |
| 3 | `4ba5a9ff626c55de8fac506040cbdea0c33cffb4` | Literal; pending reference; reference graph |
| 4 | `1d44d28284234e9a74bb80b97aab897bf85c9f97` | Literal; pending reference; reference graph |

The replay script archives those exact revisions and builds an example driver
with Rust **1.96.0** and their original locked dependencies. Each checkout has
its own Cargo target directory, preventing historical libraries with the same
package/version from sharing artifacts. The copied driver invokes unchanged
historical parsers, formatters and encoders. Each original decoder must accept
its producer's output. Temporary checkouts and build products are removed after
generation; normal tests need only the committed files.

The [provenance receipt](../../packages/cem_ml/tests/fixtures/cemb-legacy/provenance.json)
records producer revisions, encoder/format/lockfile hashes, driver hash and each
binary/source/observation SHA-256. The exact `.cem` sources retain the original
fixture bytes without an added newline. Explicit test-only JSON observations
record native attributes, references, source stacks, node counts and canonical
source **before** historical encoding; they are independent expectations for
today's decoder.

## Assertions and refusal coverage

The [shared test helper](../../packages/cem_ml/tests/support/legacy_cemb.rs)
verifies receipt/schema/driver and binary/source/observation hashes, the declared
wire version, exact canonical source/node count and every recorded attribute or
reference field/source frame. Legacy metadata defaults remain empty.

The original literal regression now covers versions 2, 3 and 4, retaining exact
`authored` value, empty native slots and the original ID binding. Pending root
references preserve their expression, context and `None` targets. Graphs retain
duplicate, self and cyclic non-owning targets, `Some([])` versus `None`, complete
retained-owner identity and inspection assertions. A current-v5 graph round trip
also retains the original full equality against today's parser source stack.

Historical stacks legitimately lack an expression-embedding frame added by a
later parser. Comparing those old stacks against today's parse would manufacture
new provenance. They instead must match their **complete historical stacks**
from the producer observations; no frames are discarded or normalized away.

Negative cases explicitly retain header mutation where it is the malformed
input being tested: v3/v4 reference tags under a v2 header must reject with
`UnknownKindTag`, and a current metadata body relabeled as v2/v3/v4 must refuse.
Corrupt payloads must produce `IntegrityMismatch`; structurally truncated bodies
with recomputed valid hashes must still produce `UnexpectedEof`; zero count
budgets must return `CountLimitExceeded`. Production decoder and integrity
implementations, wire versions and resource limits are unchanged.

## Verification

Both native feature profiles pass the same ten-suite cohort. The final fixture
checksum guard is included in these runs:

| Integration suite | Default | No default features |
| ----------------- | ------- | ------------------- |
| `native_attribute_values` | 7/7 | 7/7 |
| `node_references` | 11/11 | 11/11 |
| `typed_prelude_transport` | 8/8 | 8/8 |
| `cross_surface_conversion_rules` | 6/6 | 6/6 |
| `attribute_expression_lexing` | 4/4 | 4/4 |
| `reference_syntax_adoption` | 8/8 | 8/8 |
| `reverse_conversion` | 2/2 | 2/2 |
| `cross_surface_projection` | 4/4 | 4/4 |
| `fixture_pair` | 5/5 | 5/5 |
| `namespace_rebinding_fixtures` | 3/3 | 3/3 |
| **Total** | **58/58** | **58/58** |

No cases are filtered or skipped in either cohort. A separate focused run passes
all **18** native-attribute/reference cases. The first run against genuine
historical observations exposed the old-versus-current source-frame expectation;
that **17/18** result is retained here rather than described as a decoder fault.

Changed Rust files pass `rustfmt --check`; the regeneration script parses and
all eight binary/source/observation hashes plus producer source hashes verify.
The frozen presentation archive retains its exact size and SHA-256. Existing
tree-sitter C compiler unused-label warnings remain. No production Rust, JS/UI,
formatter, wire codec, capability policy or published runtime artifact changed.
This is not a claim that every repository release gate has been run.

```sh
python3 packages/cem_ml/tests/fixtures/cemb-legacy/regenerate.py
cargo test -p cem-ml --test native_attribute_values --test node_references --no-fail-fast
cargo test -p cem-ml --no-fail-fast --test native_attribute_values --test node_references --test typed_prelude_transport --test cross_surface_conversion_rules --test attribute_expression_lexing --test reference_syntax_adoption --test reverse_conversion --test cross_surface_projection --test fixture_pair --test namespace_rebinding_fixtures
cargo test -p cem-ml --no-default-features --no-fail-fast --test native_attribute_values --test node_references --test typed_prelude_transport --test cross_surface_conversion_rules --test attribute_expression_lexing --test reference_syntax_adoption --test reverse_conversion --test cross_surface_projection --test fixture_pair --test namespace_rebinding_fixtures
```

Commands use the pinned Rust toolchain with `CARGO_BUILD_JOBS=2`. Browser/WASM and
native CLI outputs were preserved; the [matched size receipt](../stripped-wasm-profile-measurements.json)
remains the historical artifact comparison at its recorded source revision.
