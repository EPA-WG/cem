# Login exercise: local Studio participant kit

Use this handout with the supplied `cem-studio-login-kit.tgz` archive. It contains
Studio's complete static deployment, the ordinary Login example, its frozen
presenter references and a local Node HTTP launcher. The archive is prepared
from the verified repository build; this is a local transfer route, not an npm
installation instruction or a claim that a public Studio URL is available.

## Start locally

1. Unpack the supplied archive into a new folder. Keep the whole directory tree.
2. In a terminal, enter the `cem-studio-login-kit` folder and run `node start.mjs`.
3. Open the printed loopback URL, normally `http://127.0.0.1:8080/`.
4. Open `/?collection=amsterdam&sample=login&file=readme` on that same origin.
   In Projects, select Login and use **Clone selected** to create your own copy.
5. Read the README, select `data/login.cem` in Structure, and follow the exercise below.

The launcher uses Node built-ins only: no `npm install`, Rust build, workspace,
package registry or application dependencies are needed after extraction. The
repository's Node target is `^22.12.0 || ^24.0.0`; use the actual tested version
from the verification record when preparing machines. This does not establish
that Node installation itself works offline or that other operating systems have
been tested. Use an up-to-date Chromium-family browser for the tested route.

If the port is occupied on Linux, run
`CEM_STUDIO_KIT_PORT=8090 node start.mjs`. Use that printed URL consistently.
Browser projects and drafts belong to the origin, including the port; changing
ports opens different storage. Keep the same hostname, port and browser profile
when returning to your work. Stop only this kit's server with Ctrl+C.

The launcher checks the complete file inventory and hashes before listening.
If it reports changed or missing files, unpack a fresh copy. Keep authored edits
inside your cloned Studio project; changing the kit's static files invalidates
the inventory. The inventory detects drift; it is not a signed release artifact.

## Exercise

| Step | Action | Expected result |
| --- | --- | --- |
| Edit | Change the first Login heading to `Bienvenue 🍒`, then **Save and validate**. | Zero errors, three existing unresolved-reference warnings. |
| Diagnose | Select **Validate form method**, validate the saved source, and select `demo.login.method_required` in Diagnostics. | The editor selects the original `{form` span. |
| Repair | Restore `@method=post` beside `@cem:form="sign-in"`, save, and validate. | The error disappears; the same three warnings remain. |
| Inspect | Run **Inspect login**, **Format Login (pretty)** and **Format Login (tabular)**. | Native structure and formatted source; formatting does not overwrite the saved source. |
| Compare | Restore and save the baseline heading, then run the HTML and CEM-ML semantics configurations. | The same 21 named values. This comparison covers the stated fields, not byte equality. |
| Transform | Run **Render Login graph**. | `generated/login.html`; source provenance points to original inputs/template instructions. |
| Interact | Run **Login live view**, change its local action label, save and rerun. | Shared light-DOM controls display the saved edit. The view reads `data/login.html`; the graph separately reads `data/login.cem`. |

The complete resource/configuration IDs and commands are in
[the frozen project](docs/declarative-amsterdam-2026/login-presenter/README.md).
Saved HTML/query/diagnostic checkpoints remain readable there if a live step
cannot be completed. Native replay commands in those developer reference notes
require the repository/toolchain; they are separate from this kit's Node launcher.

## Reopen offline and preserve a draft

Allow the first page to finish loading and the service worker to prepare its
offline cache before disconnecting. Run the Login exercise once while connected
to the local server. The initial kit transfer, Node/browser installation and first
cache population are preparation steps; an empty browser cache is not an offline
installation route.

In a saved project, leave an unsaved Unicode source edit, reload on the same
origin and check that the draft returns. Select another folder and cancel the
draft-resolution dialog to keep it. An unsaved draft is distinct from saved
execution inputs: commands still run the persisted revision. Reopen a second
independent copy to confirm it retains the original source. Keep the local
server running for normal use; warmed service-worker offline reopening is also
covered by the deployment verifier. Do not clear site data, saved projects or
draft storage as part of setup or recovery.

## Credentials

The example does not authenticate an account or submit credentials to an identity
provider. Use disposable demo values only. Entered passwords and one-time codes
block recovery until explicitly cleared. Do not save credentials into sources,
archives or screenshots. Automatic whole-Studio recovery stays disabled, with
the existing 4 MiB aggregate checkpoint budget and five-second handoff deadline.

## Verification scope

The verification record is produced alongside the kit as
`dist/reports/cem-studio/participant-kit.json` in the preparation repository.
It records the source revisions, archive hash, actual Node/OS/CPU/browser and
the extracted local kit's acceptance result. Until that check passes, treat this
handout as preparation instructions. Public hosting, public registry packages,
fresh OS installation, and other platforms require their own verification before
being advertised. No public resource links are published by the kit tooling.
