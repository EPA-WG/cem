# Rust setter candidate gate (2026-09-25)

`wpt-url-setters.json` is the unchanged WPT `url/resources/setters_tests.json`
from revision `c48d58747e1f211527fb695fd60548a997fae617`, covered by the existing
[WPT BSD license](LICENSE-WPT.md). Source SHA-256:
`62e4b14bdaddd66b0f7d99bf066aed1c4a588667f4aa97bf26da212134fc24bf`.
[Original source](https://github.com/web-platform-tests/wpt/blob/c48d58747e1f211527fb695fd60548a997fae617/url/resources/setters_tests.json).
JSON is an explicit test-data boundary, not a runtime document representation.

The native test runs all 277 cases in the nine component setter arrays, in the
CEM setter order. The source's comment and single `href` assignment case are
retained but not executed: `href` is an assembly seed, not a CEM component setter.
Indices below are zero-based positions within the named component array.
All supplied expected fields are checked. Each case first parses its seed with
`url` 2.5.8, then applies its `quirks` setter: differences characterize that
combined path and may originate in seed parsing as well as the setter itself.

## Results

255/277 cases match. The initial zero-difference assertion fails with 36 field
differences across 22 cases. The final test asserts the exact
[recorded differences](url-2.5.8-setter-differences.txt), with WPT expectations
unchanged. Passing characterization does not mean setter conformance, and the
baseline is not a production allowlist.

| Cases | Difference |
| --- | --- |
| host[59], hostname[41] | Empty punycode label rejected; existing host retained |
| hostname[34–35] | Non-special path serialization retains an extra `/.` |
| port[26] | Newline/tab-only port assignment clears existing 3000 instead of preserving it |
| pathname[5] | Empty path becomes `/` for `foo:///some/path` |
| pathname[13] | Caret is not percent encoded |
| pathname[21–23] | File path slash segments collapse |
| pathname[24–26] | Non-special hostless path serializes as `non-spec://p` instead of `non-spec:/.//p`, changing interpretation on reparse |
| pathname[27] | Extra `./` remains in non-special path |
| search[10–13], hash[16–19] | Opaque trailing spaces are lost or retained unencoded instead of the expected encoded ending |

Additional authored probes (outside the 277) show:

- Invalid host port `other.test:70000` correctly changes hostname and preserves
  port 8443, but `set_host` returns `Ok(())`. The required `host.port` warning
  needs independent outcome detection.
- Equal port and normalized hostname assignments succeed without changing href;
  prefix port `123abc` succeeds; invalid port and special-to-opaque protocol
  assignments return errors. Comparing only old/new href cannot classify them.
- Opaque pathname assignment is ignored with no return signal.
- `file:` protocol assignment returns `Err(())` for both equal `file://host/a`
  and an eligible `https://example.test/a` conversion. Equal valid assignments
  must not become false warnings; the eligible conversion should yield
  `file://example.test/a`. A local Node URL check confirms these two expected
  serializations; that is supplemental evidence, not the native implementation.
  See the [standard scheme-state rules](https://url.spec.whatwg.org/#scheme-state).

## Initial scope decision (accepted)

Keep Rust as chosen. Direct delegation cannot satisfy accepted D2. The user
chose to fix the Rust setter path before enabling assembly/updates, beginning with
port preservation, hostless-path serialization and reliable ignored/partial
outcome detection. File/opaque-path behavior and protocol transitions also need
coverage. Assess fixes individually as bounded adapters or parser-level changes;
do not assume string replacement can repair every case. A maintained fork has
not been authorized. Alternatively defer the assembly/update API while keeping
the completed read/parse core. Do not silently extend the prior parse-gap
exception to these newly discovered setter differences.

Importance and the complete case list remain in
[the wishlist](../../../../docs/wishlist.md#rust-url-setter-compatibility-gaps).
Assembly validation, fixed multi-field order, warning diagnostics and query
registration remain unimplemented; this change adds evidence only.

## Initial probe validation

```sh
cargo test -p cem-ql --test url_setter_candidate --target-dir dist/target/cem_ql
```

Four characterization tests pass. No production/common module changed; no full
suite, global task or WASM build ran. Source byte/hash integrity, formatting and
whitespace checks pass.

## Bounded port/path adapters completed

The native [component primitives](../../src/stdlib/url_setters.rs) fix port[26]
and pathname[24–27]. All 27 port cases and these four hostless path cases now
match unchanged WPT expectations. The full adapted matrix matches 260/277:
17 cases retain exactly 30 differences. The original raw-dependency baseline
still records 22 cases/36 differences.

A truly empty port assignment clears the port. A nonempty assignment consisting
only of stripped tabs/newlines is ignored, with an explicit `Ignored("port")`
outcome. Equal, normalized and valid-prefix assignments are classified by
applicability/parser outcome rather than serialization equality.

For hostless hierarchical pathname updates, the adapter uses the normalized
path component, scheme and retained query/fragment to rebuild only that URL's
serialization, adding/removing the `/.` guard as required. It does not reparse
the dependency's ambiguous authority-looking output. Opaque pathname updates
report `Ignored("pathname")`. The low-level mutators are intended for a private
clone; assembly/update query APIs remain unregistered and unimplemented.

Four [native adapter tests](../../tests/url_setters.rs) cover outcomes, WPT
expectations, preserved caller seeds, and reparse invariants. One is explicitly
characterization: expected file outputs `file://monkey//`, `file://////` and
`file://///` are collapsed by `Url::parse` itself. The pinned parser's
`parse_path` removes leading empty file segments, so the hostless reconstruction
strategy cannot solve this loss. A fix at the parser layer needs its own scope
and regression review. No dependency source or version changed.

**Subsequent decision:** the user accepted the scoped Rust parser patch; see
the completed work below. This does not reopen
the Rust toolchain choice. Other setter gaps remain listed in the wishlist.

Validation: all 31 focused URL tests pass, including both raw and adapted setter
matrices. Only the URL module export and new component code changed in runtime
sources; shared evaluator/type behavior is unchanged. No full/global tests ran.

The package `yarn nx run cem_ql:build:wasm` gate also passes. This verifies
compilation, not query execution parity for the unregistered primitives.

## File parser patch completed

The private [cem-url patch](../../../../vendor/url/CEM-PATCH.md) corrects file
path-start and preserves leading empty segments. Cases pathname[21–23] now
pass, including parse/serialize round trips. The full patched/adapter matrix
matches 263/277 with 14 cases/24 differences remaining. The raw registry probe
still matches 255/277. Parse case 138 also improves through the same patch.

The adapted matrix now lives in `tests/url_setters.rs`; the raw candidate test
imports `url_unpatched` so it remains independent. All 31 focused URL tests,
66 dependency unit tests and the dependency's historical WPT harness pass.
No upstream expected outputs changed; only 23 fixed expected-failure entries
were removed from that historical harness. Next is opaque trailing-space and
partial-host/protocol outcome work. No full/global tests ran.

The CEM-QL Nx WASM build passes with the private parser dependency. This is
compilation verification; registered query execution parity remains pending.
No full workspace/package test suite or global task ran.

## Opaque spaces and host/protocol outcomes completed

Opaque boundary encoding now fixes search[10–13] and hash[16–19]. The adapted
matrix matches 271/277; six cases retain 12 differences. CEM-pinned expectations
and raw-registry evidence remain unchanged. The eight stale historical vendor
setter cases were refreshed from these exact pinned cases; see the expanded
[patch provenance](../../../../vendor/url/CEM-PATCH.md).

Additional authored regressions prove partial host.port rejection preserves
hostname effects, IPv6 colons are not mistaken for port delimiters, normalized
and prefix assignments succeed, and equal/eligible file protocol updates avoid
false rejection. All 34 focused URL tests and dependency tests pass. Source-
mapped warnings and assembly/update query registration remain pending. Next
are hostname[34–35], pathname[5] and pathname[13]; empty-punycode setter cases
remain linked to deferred IDNA compatibility in the wishlist.

The package Nx WASM build passes; no full/global test suite ran. Shared
evaluator/type behavior and CEM-ML registry dependency remain unchanged.
