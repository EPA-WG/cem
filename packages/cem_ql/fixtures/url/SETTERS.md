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

## Decision before implementation

Keep Rust as chosen. Direct delegation cannot satisfy accepted D2. Recommend
fixing the Rust setter path before enabling assembly/updates, beginning with
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

## Validation

```sh
cargo test -p cem-ql --test url_setter_candidate --target-dir dist/target/cem_ql
```

Four characterization tests pass. No production/common module changed; no full
suite, global task or WASM build ran. Source byte/hash integrity, formatting and
whitespace checks pass.
