# CLI HTML page ingress and authored bases

Base commit `b4855cdfc271a7b7e2b243309808ceaa03bc7211`.

- [x] Admit one href-only `<base>` in the head of an SSR HTML page. Keep the
  authored tag in the export and resolve CSS tickets from the effective
  HTTP(S) base. Retain the base attribute's source map in the readiness
  report. Keep ordinary HTML resource checks active, including checks for
  additional resource slots that occur after the base.
- [x] Measure direct and concrete config page acquisition and native HTML
  parsing under the original operation. Test a parser CPU refusal before
  route preflight and a file byte refusal before opening. Preserve previous
  destinations on refusal.
- [x] Verify the emitted base, retained scoped CSS and zero external requests
  in Chromium.

The measured acquisition profile uses regular local files. The CLI assumes
there is no effective header CSP and refuses an authored base when the page
contains a CSP meta element. Glob acquisition, derived graph inputs, CEM-ML
and opted-in Markdown pages remain separate work items. For a native caller
that supplies bytes, CPU measurement begins with parsing.

Native verification passed ten CLI SSR cases, nine export cases per adapter
profile, three stripped artifact unit cases, and eleven ordinary graph,
template and stylesheet CLI regressions. The CLI-to-Chromium suite passed
48 cases.
