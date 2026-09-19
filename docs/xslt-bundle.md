# XSLT compiled bundle contract

The approved delivery boundary is an explicit XSLT bundle, owned by
`https://cem.dev/ns/transform/xslt/1`, with content type
`application/vnd.cem.xslt-bundle+cem-bin` and format `cem-xslt-bundle/1`.
`cem_ql::xslt` integrates the independently owned CEMT and XPath codecs. It
does not change either codec or the default native-function registry.

The binary framing is `CEMXSLT1\n`, a little-endian u32 length and UTF-8 JSON
control manifest, then length-prefixed ordinary CEMT artifact bytes and XPath
artifact bytes in manifest order. JSON here describes deployment identity and
bindings; it never carries runtime documents or executable ASTs. Loading
requires the expected bundle hash and root stylesheet source hash from the
compiler or a trusted manifest. Hashes establish integrity, not publisher
authentication. Versions, member hashes and source identities are checked
before capabilities become available.

The manifest records an ordered, acyclic, fully reachable stylesheet closure.
Entry zero is the root. Each source has a unique URI, original byte length and
hash; each ordered dependency names an import/include and its target index.
Repeated edges are retained. The compiler resolves imports and generates one
linked CEMT artifact before packaging; loading never fetches dependencies,
parses source, or computes XSLT import precedence. Changes to imported sources
change bundle identity even when the root source is unchanged.

Generated CEMT has a separate source identity and its original source-map
mode and host bindings. Its host maps use source ID 1 for that generated source; embedded query maps
use source ID 0 with offsets local to the retained expression text. Each is
validated against its own byte length. Every XPath
member has a stylesheet owner, independent program hash, preserved static
context and original stylesheet ranges. Source IDs are local to each member;
the owning URI distinguishes equal numeric IDs across sources. Packaging does
not invent mappings from generated CEMT back to stylesheet instructions.

Program `i` supplies exactly `xslt.program.i` to the bundle's local registry.
Its positional ABI is a declared focus, optional group context, then ordered variables:

| Focus | Leading arguments |
| --- | --- |
| absent | none |
| singleton | one native context item |
| sequence | one native context item, integer position, integer size |

The additive program binding `group_context: true` inserts four arguments after
focus: group-present (boolean), native group sequence, key-present (boolean),
native atomic key sequence. Presence is distinct from an empty sequence; an
absent value must have an empty payload. The field defaults to false and is
omitted in that case, so existing version-1 bundles retain their ABI. Readers
that predate this field reject it through strict manifest validation. Arity and
argument limits include all four arguments. Only programs accessing group
functions need these bindings; document/control inputs cannot install context
capabilities. Native state is never encoded in the bundle.

Sequence focus uses the shared XPath contract `1 <= position <= size`.
Variables declare expanded names and scalar or native `any` types. Scalars
are checked, never inferred from records; `any` carries retained native XPath
items/CEM nodes. Variable names must agree with the program's static context.
Callbacks share the caller's operation control, scope and limits. Resource
and budget failures retain their uncatchable classification. Function items
cannot cross the CEM-QL return boundary.

Limits are 8 MiB per bundle, 128 KiB of manifest, 64 stylesheet sources,
128 XPath programs, 32 dependency levels and 4 KiB per identifier. The retained
host admits at most 16 bundles and 32 MiB of encoded bytes. Member codecs also
enforce their own limits. Handles increase monotonically and are never reused;
disposal releases host ownership while an already borrowed native handle
remains valid until dropped.

WASM explicitly imports with `importXsltBundle(bytes, contentHash,
rootSourceHash)`, renders with `renderXsltBundle(bundleId, scalarBindingsJson,
documentBindingsJson)`, and disposes with `disposeXsltBundle(bundleId)`.
Document bindings are control entries `{name, documentId}` referencing shared
retained CEM documents imported through `cem-ml`. Scalar bindings are declared
top-level host parameters only. Neither control input can install callbacks.
XML, JSON, and future external document formats resolve exclusively at the
[CEM import boundary](cem-data-import-principle.md).

The [typed runtime compiler](xslt-runtime-lowering.md) now produces these
bundles from a bounded stylesheet profile with recursive templates, parameters,
modes and import/include precedence. WASM also provides `compileXsltBundle`
and `retainXsltStylesheet` with default compiler options; native-built closures
load through the binary bundle API. Non-composite grouping and its dynamic
context also execute through this ABI. Sorting, broader output and the
data-table viewer remain subsequent checklist items. Verification covers
explicitly composed bundles, native/WASM stylesheet compilation and compiled
module closures with imported overrides, grouping over all shared import
formats, and absent group-context errors without partial output.
