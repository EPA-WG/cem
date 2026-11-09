# Scoped CSS adoption for canonical SSR groups

Base commit `637dc4daf66f29f0a1c31e0f72cb2e92a23e56b8`.

Closed stylesheet occurrences use portable native identity across CLI and
browser source origins. The key covers declaration or instance ownership,
scope, occurrence and CSS bytes. Existing retained stylesheet owners install
the matching native output without replacing its style node or CSSOM sheet.

Native group admission captures the actual declaration and instance style nodes,
rebuilds the resource-free closures, and compares the full emitted occurrences.
Instance CSS comes from the explicit HTML payload. Chromium checks the live
rule lists before publication, including edits that do not alter the raw text.
Missing, changed, disabled or unresolved styles and unadmitted shared owners
freeze the group. The profile covers private and shared declaration rules and
explicit instance styles, including scoped animation names. Shared source
owners must be in the admitted group. Older CLI output with escaped CSS or
origin-dependent keys requires re-export for this profile.

The shared HTML writer now projects literal text for HTML style bodies. It
checks adjacent fragments together for a raw closing tag before serialization.
This preserves scope combinators and other CSS syntax in the original browser
parse, before hydration starts.

Native verification includes 14 retained stylesheet checks, 14 SSR stylesheet
checks, 7 canonical admission checks, and 2 shared HTML writer checks. The
stripped SSR suite passed 123 checks and the component runtime passed 17. The
CLI and named export suites passed 4 and 6 cases respectively.

The optimized package passed 31/31 CLI-to-Chromium cases with zero external
requests, including 12 stylesheet cases. Both worker and main-thread processing
reused the original nodes and CSSOM sheets. The initial optimized browser run
encountered a local module fetch error. A clean rerun passed the full gate.
The core and packaged WASM match at 24,049,316 bytes with 33 imports and 177
exports. SHA-256 is
`964045ea6d74830a01de876f2ae1f8f9b2b87acc69add28f5ca247cebd603124`.

The source-admission gate passed 314 native checks and 30 browser cases. All ten
Edge SSR stories, WASM consumer regressions, Clippy and browser lint passed.
