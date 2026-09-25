# URL-PARSE candidate vectors

These 51 cases are selected unchanged from WPT `url/resources/urltestdata.json`
at revision `c48d58747e1f211527fb695fd60548a997fae617`. `upstream_index` is the zero-based index in the original
JSON array, including comment strings. Each `case` object retains its original
keys and expected values. Missing expected fields remain unspecified.

- [Pinned source](https://github.com/web-platform-tests/wpt/blob/c48d58747e1f211527fb695fd60548a997fae617/url/resources/urltestdata.json)
- [Pinned license](https://github.com/web-platform-tests/wpt/blob/c48d58747e1f211527fb695fd60548a997fae617/LICENSE.md)
- Upstream source SHA-256: `81e85fd3c199c08ef9c34cf651b3580eeedd080316493bfaf277a6b5ff8cf652`
- [Redistribution license](LICENSE-WPT.md): BSD-3-Clause; copyright WPT contributors.

Selection covers credentials, whitespace, explicit relative bases, invalid ports,
Unicode/IDNA, special and non-special hosts, IPv4/IPv6, file paths, dot segments,
empty fragments and blob origins. It deliberately includes empty-punycode labels
and nested/non-HTTP blob origins to expose dependency differences. This bounded
sample is not the complete WPT suite or a claim of full WHATWG conformance.

The JSON file is an explicit upstream test-data boundary. It is never used as a
runtime CEM document or AST representation. Tests perform native URL operations;
JavaScript is not an implementation fallback. CEM's additional pure-profile
constraints are tested separately from unchanged upstream expectations.

## Candidate findings — decision pending

`url` 2.5.8 is pinned as a **dev dependency only**. The first conformance probe
failed with 11 field/outcome differences across eight cases. The other 43
selected cases match every provided expectation, including expected failures.
All successful parses also retain href through parse/serialize repetition.

| WPT indices | Candidate difference |
| --- | --- |
| 138 | Does not normalize a file drive's vertical bar to a colon |
| 664 | Loses the IPv6 host from a file URL with a drive path |
| 782 | Recursively derives an origin for a nested blob URL |
| 786–788 | Derives FTP/WS/WSS blob origins instead of an opaque origin |
| 839, 921 | Rejects empty punycode labels accepted by these file/HTTPS vectors |

[`url-2.5.8-differences.txt`](url-2.5.8-differences.txt) records exact observed
candidate differences, separately from the unchanged WPT data. The test asserts
that characterization so a dependency or fixture change cannot silently alter
it. **A passing characterization test is not conformance and this file is not a
production allowlist.** Production adoption must resolve every selected gap or
return to an explicit contract decision; do not edit upstream expected values
to match the dependency.

Additional authored tests cover invalid bases even with absolute input, absent
bases, opaque bases, relative resolution, empty query/fragment delimiters,
duplicate query decoding and pure file/non-blob origins. These test the native
candidate directly, not registered CEM-QL functions or runtime parity.

```sh
cargo test -p cem-ql --test url_parse_candidate --target-dir dist/target/cem_ql
```

Recommendation: keep the accepted semantics and investigate a maintained native
parser fix/replacement before production adoption. The four origin gaps are
bounded by the pure profile; host/drive preservation and IDNA rejection need
parser-level evidence. Do not assume a generic serialization wrapper can repair
information already discarded or input already rejected. Stop before choosing
between maintaining parser patches and adopting a different native dependency.
