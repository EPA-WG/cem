# Required Login walkthrough

Use the ordinary **Login** project (catalog ID `login`, version `2.8.1`) in
Studio's Amsterdam collection. The checklist below pins resource and configuration
IDs from [example.json](../../examples/projects/login/example.json).
Start with a new independent copy; preserve existing saved projects and drafts.
Use the current full runtime. This route does not depend on the minimal WASM profile.

## Ordered checkpoints

Open Studio at its deployed base (`/studio/` on the demo site, `/` on the
standalone server). Append `?collection=amsterdam&sample=login&file=readme`.
The URL selects the predefined sample; create an independent copy before editing.
Saved copies use their own project selection URL, not `sample=login`.

| Step | Resource ID / configuration ID | Action and expected result |
| --- | --- | --- |
| Read | `readme`, then `login-cem` / `read-edit` | Read the purpose and essential files; Parse presents native CEM-ML structure. |
| Edit | `login-cem` / `validate` | Change the first heading from `Sign in` to `Bienvenue 🍒`. Run before saving to observe the persisted version; save and validate to commit the edit. Zero errors; the three known unresolved-reference warnings remain. |
| Fail | `login-invalid` / `validation-repair` | Validate the syntax-valid source missing its form method. Expect `demo.login.method_required` and three unresolved-reference warnings. Select the diagnostic to locate the original form. |
| Repair | `login-invalid` / `validation-repair` | Add `@method=post` beside `@cem:form="sign-in"`. Save, validate again: zero errors, three existing warnings. Unsaved repair does not change persisted execution. |
| Inspect and format | `login-cem` / `inspect`, `format-pretty`, `format-tabular`, `source-colors` | Inspect original source coordinates; export plain pretty/tabular CEM-ML or inert source-color HTML. Formatting never overwrites the saved source. Restore and save the heading for baseline comparisons. |
| Compare | `login-html`, `login-cem` / `semantics-html`, `semantics-cem`, `semantics-html-cemql`, `semantics-cem-cemql` | All four return the 21 named string values in `login-semantic-contract`. Run `html-conversion` separately to export HTML as CEM-ML. |
| Evaluate | `login-expression` / `expression-html`, `expression-cem` | Both return `title=Sign in` and `required=2`. Edit the expression, save, rerun; restore it afterward. |
| Transform | `login-view-template`, `login-view-graph` / `render-html`, `render-cem`, `render-graph` | Rendered HTML matches `login-view-expected`. Graph output is `generated/login.html`; follow its provenance to the original template. Edit the instruction, save and rebuild to demonstrate the change. |
| Interact | `login-live-view` / `live-view` | Parse the element-template and show shared text fields, action and alert in a script-free light-DOM preview. Edit its action label, save and rerun. Its data input is `login-html`, independently of the CEM graph source. |

To jump directly to a predefined checkpoint, append, for example,
`?collection=amsterdam&sample=login&file=login-view-graph&build=render-graph`.
Resource IDs drive selection; displayed paths are not resolver identities.
Configuration arguments and all declared dependencies are recorded in the
linked manifest; Studio runs the saved project revisions.

## Boundaries to explain

The semantic comparison covers the agreed 21 fields, with explicit namespace
and HTML annotation mirrors. It does not establish byte equality or universal
HTML/CEM interchangeability. Generated HTML has exact expected bytes, including
its trailing spaces. Graph provenance identifies authored instructions and copied
input ranges; it does not invent per-character lineage for computed text.
Parse/Inspect's printed native coordinates alone are not owned provenance links.
Source-color HTML is an export; the sandboxed document preview blocks its styles.

The live view uses `application/vnd.cem.element-template+cem` and
`https://cem.dev/ns/template/cem-element/1`, with 15 advisory open-content
attribute warnings. Existing saved copies retain their own source identities.
The project README records the remaining CLI/worker undeclared-variable
validation difference; worker success alone is not strict binding conformance.

Use only disposable credentials. Entered passwords and one-time codes block
recovery until explicitly cleared. Never export credentials in checkpoints or
project bundles. Automatic whole-Studio recovery remains disabled; retain the
shared 4 MiB aggregate budget and five-second handoff deadline.

## Acceptance record

Source baseline: `1a7cd833ab4d2e85157afceb9cce930e9d24220a` plus the README
identity correction and this walkthrough. Browser command (from repository root):

```sh
NX_DAEMON=false yarn nx run @epa-wg/cem-studio:test-ci--src/feature-tour.browser.spec.js --excludeTaskDependencies --skipNxCache --args='--project=browser --testNamePattern="Login|teaching schema"'
```

This exercises real command-service worker operations for Login queries,
expressions, formatting, element-template parsing, graph output/provenance and
teaching-schema failure/repair. It does not by itself verify manual UI navigation,
offline participant installation, or the complete live interaction route.
Record those separately before declaring the whole presentation route frozen.

The focused Login command-service run on 2026-11-01 passed all 10 selected
browser checks (54 unrelated cases skipped). The standalone navigation actions
check also passed with the requested `0.5rem` gap. These checks use disposable
test state and do not clear existing saved projects or drafts.

Live recovery verification also passed both worker and fallback cases:

```sh
NX_DAEMON=false yarn nx run @epa-wg/cem-studio:test-ci--src/recovery.browser.spec.js --excludeTaskDependencies --skipNxCache --args='--project=browser --testNamePattern="complete live Login"'
```

Each case checks two credential refusals, three successful recoveries after
clearing, twelve edits, and both checkpoint and group sizes within 4 MiB.
These tests explicitly enable recovery in isolated disposable realms; they do
not enable automatic recovery in the application or an existing user session.

## Deployed navigation and persistence acceptance

The repeatable deployment check is:

```sh
NX_DAEMON=false yarn nx run @epa-wg/cem-studio:verify:login-navigation
```

Its dependencies build and verify the static deployment. For an already built
and output-verified deployment, use `--excludeTaskDependencies --skipNxCache`.
The check runs in a new Playwright context with a service-worker-controlled page;
it never connects to the user's browser or clears their storage. It covers the
20 Login configurations and the three companion Supplier configurations offline.

The Login route selects the original schema diagnostic in the shared Diagnostics
list, verifies editor focus and the selected `{form` source span, saves the repair,
and checks the remaining three warnings. It creates independent saved copies,
checks their original source provenance and separate resolver roots, and runs
edited templates and sources against the unmodified second copy.

Additional acceptance opens the saved `login-cem` file with `build=render-graph`,
checks the URL against the Structure tree and window title, edits an unsaved
Unicode draft, then reloads offline. The exact draft and dirty state must return
while both copies' persisted source bytes remain unchanged. Selecting the `data`
folder must open the shared draft guard; cancelling preserves the file route,
editor contents and both copies. This checks ordinary resource persistence,
without placing entered Login credentials in draft storage.

The first deployed run exposed a missing shared runtime dependency:
`compiled-declaration-assets.js` returned 404 before Studio initialized. Both
module-map inventories now include the asset, and the output verifier requires
it beside the shared runtime facade. The startup verifier also reports failed
HTTP requests on initial readiness failure. The rebuilt output check passed
with 219 declared assets and 334 files.

On 2026-11-01, the rebuilt deployment passed `verify:login-navigation`,
including all final URL/tree, offline draft and unchanged-copy assertions.
Studio build, output verification and lint also passed. The initial failed run
was the missing asset described above; no timing threshold or credential
restriction was relaxed. Participant installation/access and timed rehearsal
remain separate acceptance work.
