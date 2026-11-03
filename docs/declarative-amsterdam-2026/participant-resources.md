# Participant resources

Use [Resources and updates — discussion #24](https://github.com/EPA-WG/cem/discussions/24)
as the stable bookmark and QR destination. The links below were read successfully
on 2026-11-01 and pin the verified source/reference revision `712ff2e4`.

| Resource | Public reference | Local copy |
| --- | --- | --- |
| Start Studio and preserve your work | [Participant setup](https://github.com/EPA-WG/cem/blob/712ff2e45b89d159abba7b8447ab53e5efb8ae68/docs/declarative-amsterdam-2026/participant-setup.md) | [Handout](participant-setup.md) |
| Required Login exercise | [Login inputs and expected results](https://github.com/EPA-WG/cem/blob/712ff2e45b89d159abba7b8447ab53e5efb8ae68/docs/declarative-amsterdam-2026/login-presenter/README.md) | [Frozen Login](login-presenter/README.md) |
| Supplier companion | [Supplier inputs and expected results](https://github.com/EPA-WG/cem/blob/712ff2e45b89d159abba7b8447ab53e5efb8ae68/docs/declarative-amsterdam-2026/supplier-presenter/README.md) | [Frozen Supplier](supplier-presenter/README.md) |

## Before the session

Obtain the supplied `cem-studio-login-kit.tgz` from the presenter or organizer.
The tested distribution is a local transfer; there is no advertised public kit
download or participant Studio URL. The GitHub links above are documentation and
reference data, not an app installer. The local kit includes its own handout and
both frozen projects, so those public pages are optional during the exercise.

Unpack the kit, enter its directory and run `node start.mjs`. Open the printed
loopback origin and keep using that exact hostname/port/profile for your saved
projects and drafts. Initial Node/browser installation and first cache population
happen before the timed session. The verified host is Linux x64, Node 24.16.0,
Chromium 148.0.7778.96; other platforms and public CLI installation remain separate
verification work. The [dated kit record](participant-kit-verification.md) identifies
the 417-file artifact by checksum and records the offline acceptance scope.

The [Node/Linux preparation guide](node-linux-setup.md) records the exact
downloaded Node archive, tested host and public CLI availability/compatibility
checks. It keeps the verified local Studio route separate from CLI installation.

On the kit's printed origin, open:

- `/?collection=amsterdam-2026&sample=login&file=readme`
- `/?collection=amsterdam-2026&sample=supplier-pipeline&file=readme`

These are origin-relative paths. In Projects, select the example and **Clone
selected** before editing. Saved copies have independent project URLs. Keep the
predefined examples as baselines, preserve unsaved drafts, and save before running
a command that should use an edit. Do not clear site data to resolve a problem.

## During and after the exercise

Follow the [Login walkthrough](login-walkthrough.md). Compare the saved native
results if a live operation cannot complete. The 21-field HTML/CEM comparison
has explicit namespace/mirror limits; it does not establish byte equality or
universal interchangeability. The Supplier outputs show one domain result in
three formats; its stock failure is distinct from Login's schema failure.

The live Login view demonstrates local interaction, not authentication. Use
disposable demo values only. Entered passwords/OTP block recovery until explicitly
cleared. Automatic whole-Studio recovery remains disabled, with a 4 MiB aggregate
checkpoint budget and a five-second handoff deadline. Do not put credentials in
sources, project bundles, checkpoint exports or screenshots.

For maintainers, the [presenter notes](presenter-notes.md) contain the timed cues,
fallbacks and exact configuration IDs. The [prepared discussion block](discussion-24-resources.md)
contains only the public links above and the local-transfer boundary. It is ready
for review; preparing it does not post a message or publish an artifact.
