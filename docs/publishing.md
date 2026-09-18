# Publishing to winget

**Status: planned, not started.** This is the backlog entry for getting
`winget install PrakashSewani.WorksAfterReinstall` to work. Nothing here is implemented yet:

| | Item | Blocked on |
| --- | --- | --- |
| P1 | Application icon embedded in the exe | - |
| P2 | `repository` / `homepage` in `Cargo.toml` | - |
| P3 | Tag-triggered release workflow | - |
| P4 | Manifests generated and checked in | P1-P3 |
| P5 | Local validation | P4 |
| P6 | First pull request to `microsoft/winget-pkgs` | P5 + CLA |
| P7 | Automated updates per release | P6 |

The identifier `PrakashSewani.WorksAfterReinstall` was checked against the live source on
2026-09-19 and is free.

## What winget requires

- **One package version per PR**, and **manifest files only** - no README, docs, tooling or
  spelling changes in the same pull request.
- A **multi-file manifest set** (version + installer + defaultLocale); singleton manifests are
  not accepted in the community repository.
- The `# yaml-language-server: $schema=...` header on **every** manifest file, a supported
  `ManifestVersion`, and directory/file names that match `PackageIdentifier` and
  `PackageVersion` **case-sensitively**.
- An `InstallerUrl` that is HTTPS, publicly reachable, **version-pinned** (no "latest"
  vanity URLs - the hash would move under you), and from the publisher's own release
  location - no mirrors, aggregators or shorteners.
- An installer that runs **unattended as a non-elevated user**: no dialogs, no UAC prompt.
- A binary that survives antivirus and PUA scanning, and an **extractable application icon**.
- A signed **Microsoft CLA** for the first contribution, or the PR carries `Needs-CLA` and
  cannot merge.

## The manifests

Layout in `microsoft/winget-pkgs` (the first path segment is the lowercase first letter of
the publisher):

```text
manifests/p/PrakashSewani/WorksAfterReinstall/0.1.0/
├── PrakashSewani.WorksAfterReinstall.yaml                  # version
├── PrakashSewani.WorksAfterReinstall.installer.yaml        # installer
└── PrakashSewani.WorksAfterReinstall.locale.en-US.yaml     # defaultLocale
```

`ManifestVersion` must be a version the repository currently supports - the list in
`doc/manifest/README.md` is the source of truth (1.28.0, 1.12.0 and 1.10.0 were the
supported entries when this was written) and the PR template names the recommended one.
`wingetcreate` fills in the right value for you; do not guess it.

**version**

```yaml
# yaml-language-server: $schema=https://aka.ms/winget-manifest.version.<schema>.schema.json
PackageIdentifier: PrakashSewani.WorksAfterReinstall
PackageVersion: 0.1.0
DefaultLocale: en-US
ManifestType: version
ManifestVersion: <schema>
```

**installer**

```yaml
# yaml-language-server: $schema=https://aka.ms/winget-manifest.installer.<schema>.schema.json
PackageIdentifier: PrakashSewani.WorksAfterReinstall
PackageVersion: 0.1.0
InstallerType: portable
Commands:
  - works-after-reinstall
Installers:
  - Architecture: x64
    InstallerUrl: https://github.com/PrakashSewani/works-after-reinstall/releases/download/v0.1.0/works-after-reinstall.exe
    InstallerSha256: <sha256 of the release asset>
ManifestType: installer
ManifestVersion: <schema>
```

**defaultLocale**

```yaml
# yaml-language-server: $schema=https://aka.ms/winget-manifest.defaultLocale.<schema>.schema.json
PackageIdentifier: PrakashSewani.WorksAfterReinstall
PackageVersion: 0.1.0
PackageLocale: en-US
Publisher: Prakash Sewani
PublisherUrl: https://github.com/PrakashSewani
PackageName: Works After Reinstall
PackageUrl: https://github.com/PrakashSewani/works-after-reinstall
License: MIT
LicenseUrl: https://github.com/PrakashSewani/works-after-reinstall/blob/dev/LICENSE
ShortDescription: Capture this Windows machine, then rebuild it after a reinstall.
Description: >
  Works After Reinstall captures your apps, Windows preferences, environment variables and
  configuration files into a portable bundle, and rebuilds them after a fresh Windows
  install. It also sets up a new machine from a built-in catalogue when there is no bundle.
Tags:
  - windows
  - provisioning
  - setup
  - developer-tools
ManifestType: defaultLocale
ManifestVersion: <schema>
```

## Work items

### P1 - Embed an application icon

winget's metadata step extracts an application icon for the catalogue; a bare Rust exe has
none. Add an `.ico` (256x256 and smaller sizes) and a `build.rs` using a resource compiler
crate such as `winresource` to embed it.

*Done when:* the built exe shows the icon in Explorer and `winget validate` passes against
a locally built manifest.

### P2 - Package metadata

Add `repository = "https://github.com/PrakashSewani/works-after-reinstall"` (and
`homepage` if it ever differs) to `[package]` in `Cargo.toml`, so the crate and the winget
manifest agree about where the project lives.

### P3 - Tag-triggered release

`.github/workflows/release.yml`, triggered on `v*` tags:

1. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`;
2. `cargo build --release`;
3. write `SHA256SUMS.txt` for the exe;
4. create the GitHub Release with both files attached.

*Done when:* tagging `v0.1.0` produces a release whose asset URL is exactly what the
manifest's `InstallerUrl` points at.

### P4 - Generate the manifests

`wingetcreate new <release-url>` scaffolds the three files (or `wingetcreate submit` opens
the PR in one step). Review the output, then check it into this repository under
`packaging/winget/<version>/` so version bumps are diffable in review instead of appearing
only in an external PR.

### P5 - Validate locally

```powershell
winget validate --manifest packaging\winget\0.1.0

# needs an elevated terminal; policy can block this on managed devices
winget settings --enable LocalManifestFiles
winget install --manifest packaging\winget\0.1.0
winget uninstall --id PrakashSewani.WorksAfterReinstall
```

If Microsoft Windows Sandbox is available, `Tools\SandboxTest.ps1` from `winget-pkgs` is the
preferred test: it proves the package needs no pre-existing dependencies.

### P6 - First submission

Sign the CLA, then open a PR to `microsoft/winget-pkgs` with the three files in the path
above. Keep it to one package version and manifest files only.

### P7 - Updates per release

For each new version: bump `Cargo.toml`, add the `CHANGELOG.md` entry, tag, let P3 publish,
then regenerate the manifests (`wingetcreate update`) and open the PR. This can be automated
with the community `winget-releaser` GitHub Action once the first PR has been merged.

## Gotchas

| Trap | Consequence |
| --- | --- |
| `Cargo.toml` version, git tag and `PackageVersion` drifting apart | Version-check failures and a package that installs the wrong label |
| Pointing `InstallerUrl` at a "latest" URL | SHA256 mismatch the moment a new release lands |
| Forgetting to recompute `InstallerSha256` per tag | `Error-Hash-Mismatch` |
| Using a generic `InstallerType: exe` | winget passes the wrong silent switches; use `portable` for a single exe |
| Declaring `ElevationRequirement` | Not needed: the tool installs per-user and only touches `HKLM` when the user asks it to |
| Looking for `AppsAndFeaturesEntries` | Portable installs create no Add/Remove entry, so there is nothing to align |
| Assuming the binary can be run by the pipeline | Our no-terminal guard prints help and exits 0 instead of blocking on a TUI - keep that behaviour |
| Unsigned exe | SmartScreen warns on download; signing options are tracked in `docs/roadmap.md` |

## What the validation pipeline does

Ten automated steps run on the PR, each reported as a check: PR shape (one version, manifest
files only) -> manifest schema -> URL reachability and HTTPS -> URL domain is an official
source -> content policy -> catalog consistency (version-range overlaps, dependencies) ->
binary scan (SHA256 + antivirus/PUA) -> unattended install as a standard user -> installer
metadata and icon -> summary. Labels to watch when it fails:
`Manifest-Validation-Error`, `URL-Validation-Error`, `Validation-Domain`,
`Validation-Unattended-Failed`, `Validation-Defender-Error`, `Error-Hash-Mismatch`.

Validation takes minutes; a human moderator then reviews, so budget days rather than hours.
After approval the PR merges automatically and, once published, the package appears in the
winget source within about an hour.

## References

- [Submit packages to Windows Package Manager](https://learn.microsoft.com/en-us/windows/package-manager/package/)
- [Repository policies](https://learn.microsoft.com/en-us/windows/package-manager/package/windows-package-manager-policies)
- [winget-pkgs contributor guide](https://github.com/microsoft/winget-pkgs/blob/master/CONTRIBUTING.md)
- [First-time contributor checklist](https://github.com/microsoft/winget-pkgs/blob/master/doc/FirstContribution.md)
- [Validation pipeline](https://github.com/microsoft/winget-pkgs/blob/master/doc/Validation.md)
- [Supported manifest schema versions](https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/README.md)
