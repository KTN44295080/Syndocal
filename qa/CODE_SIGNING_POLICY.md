# Code signing policy

Syndocal is maintained by [Seraf() / KTN](https://github.com/KTN44295080).
SignPath Foundation enrollment is pending. This policy does not claim that any
existing download has an Authenticode signature or that Foundation has approved
the project. For enrolled, successfully signed releases: Free code signing
provided by [SignPath.io](https://signpath.io), certificate by
[SignPath Foundation](https://signpath.org).

## Scope and release states

The 1.2.0 public Windows format is the x64 NSIS installer. The signing set is
Syndocal's application executable, its generated NSIS uninstaller and the final
NSIS installer. The normal workflow does not distribute the optional ASIO bridge
or NDI. A future ASIO profile requires its own licensing, source, exact component
inventory, Foundation eligibility review and signing configuration before use.
MSI, portable archives, unrelated products and arbitrary uploaded binaries are
outside this policy.

There are exactly two accepted Authenticode states:

- `signpath-signed`: all project-owned executable components and the outer
  installer have valid timestamped signatures from the configured Foundation
  certificate. The manifest records the successful SignPath request IDs and final
  component SHA-256 values.
- `declared-unsigned`: no project-owned executable component has an Authenticode
  signature. The manifest and release notes explicitly declare that state and
  its reason. This is the owner's approved no-cost fallback when enrollment or
  signing is unavailable at candidate freeze.

Unsigned upstream OSS components, including the seven FFmpeg DLLs and recording
`ffmpeg.exe`, are an explicit exception to the project signing set: their exact
bytes must match the approved [runtime inventory](FFMPEG_WINDOWS_RUNTIME_INVENTORY.json).
They must never be signed using Syndocal's subscription. Unknown, changed or
unlisted executable components, mixed project signatures, untrusted signatures,
missing declarations and hashes changed after signing are rejected. This
upstream exception follows [Foundation's published conditions](https://signpath.org/terms).

## Roles and approval

- Author/committer: [KTN44295080](https://github.com/KTN44295080), responsible for
  source, dependency pins and build scripts.
- Reviewer: [KTN44295080](https://github.com/KTN44295080), responsible for reviewing
  contributions from people without commit access, release changes and evidence.
  Automated assistants provide implementation/review evidence; they are not
  signing approvers.
- Signing approver and release owner: [KTN44295080](https://github.com/KTN44295080).
  The single-maintainer role combination must be disclosed in the application;
  Foundation acceptance of that arrangement is still required.

All human accounts participating in signing must use MFA for both GitHub and
SignPath. Each payload and installer request needs explicit human approval in
SignPath. A CI API token may submit requests but must not approve them. The
approver checks the protected ref, source commit, hosted workflow run, dependency
and artifact identities, checks and certificate before approval. Missing approval,
rejected requests and failed signature verification stop the signed path; they
do not silently downgrade a partly signed candidate.

## Build provenance and key custody

[Windows release candidate CI](../.github/workflows/windows-release-signpath.yml)
is manually dispatched from protected `main` or a protected `v*` tag in this
repository. All jobs preceding signing use GitHub-hosted Windows runners. The
workflow builds checked-out source using the lockfiles, the pinned FFmpeg SDK,
and exact MSVC 14.44.35207 with the existing PATH-first linker gate. It accepts no
local binary, arbitrary artifact URL or alternate source-ref input. SignPath's
GitHub trusted build integration verifies the GitHub artifact origin.

The first SignPath request signs the application and generated uninstaller.
The packaging helper inserts those exact signed bytes without rebuilding Rust
or frontend code. The second request signs the final NSIS installer. Its final
hash is frozen before the independent Tauri updater signature is generated.
The workflow retains source/run/artifact/request identities, component inventory,
signing-state declaration and `SHA256SUMS`. Any byte change creates a new
candidate requiring affected acceptance again.

Foundation retains the Authenticode private key. GitHub receives only a scoped
submitter token in the protected environment. The owner separately manages the
production Tauri updater key; fixture keys are forbidden in production. Missing
updater credentials produce a marked, non-publishable candidate without updater
acceptance. Authenticode unsigned never relaxes cryptographic updater checking.
No workflow in this path publishes a release or creates a tag.

## Network use and privacy

Operators request network features such as show-control connections, remote
control, configured media URLs and update checks. Operators choose those peers
or the configured release update service. Release documentation must name the
actual update endpoint and describe the information sent before public release;
no production endpoint is configured by this task. Signing CI sends build
artifacts and source/run metadata to GitHub and SignPath, not user projects.

Windows installation can obtain Microsoft WebView2; its behavior is covered by
[Microsoft's privacy statement](https://privacy.microsoft.com/privacystatement).
GitHub downloads are subject to [GitHub's privacy statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement).
SignPath's service is subject to its [privacy policy](https://signpath.io/privacy-policy).
The release owner must complete the component/network disclosure review and any
required installer consent/disable controls before claiming Foundation privacy
compliance. This policy does not certify an unperformed privacy audit.

## Candidate and publication boundary

The unsigned decision does not close license/source, clean-machine install,
actual SmartScreen/Smart App Control behavior, production update or publication
gates. The release owner freezes the selected state by November 1, 2026 for the
planned November 12 release. Late signing creates a new candidate, never an
in-place replacement of already accepted or published bytes. See the
[unsigned download note](RELEASE_NOTES_SIGNING.md) and
[owner setup instructions](../tools/distribution/signpath/README.md).
