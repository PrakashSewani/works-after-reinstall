# The manifest

`works-after-reinstall.yaml` is the bundle's only configuration file and a user-facing
contract: it is meant to be read, hand-edited and trimmed. This document is the reference
for every field.

A bundle is a folder:

```text
works-after-reinstall-bundle/
├── works-after-reinstall.yaml
└── files/                      # copies of captured config files
```

`setup` accepts either the folder or the YAML file directly.

## Top level

```yaml
version: 1
exported_at: 2026-09-18T18:54:27Z
source_machine: WORKSTATION

apps: [...]
windows: {...}
environment: {...}
files: {...}
```

| Field | Required | Meaning |
| --- | --- | --- |
| `version` | no (defaults to 1) | Manifest format version. A manifest newer than the binary understands is rejected with a clear error rather than misread |
| `exported_at` | no | Timestamp written by `export`; informational |
| `source_machine` | no | Computer name the bundle came from; informational |
| `apps` | no | Packages to install |
| `windows` | no | Windows preferences to reapply |
| `environment` | no | Environment variables and PATH entries to restore |
| `files` | no | Files to copy into place |

Unknown top-level keys are an error (`deny_unknown_fields`), so a typo fails loudly instead
of being ignored. Empty sections are omitted when writing, and any section you delete from
the file is simply not applied.

## `apps`

Each entry is either a bare WinGet package id, or a mapping when the package needs more
detail:

```yaml
apps:
  - Google.Chrome
  - Microsoft.VisualStudioCode
  - id: Microsoft.VisualStudio.2022.BuildTools
    arguments: "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools"
  - id: XP89DCGQ3K6VLD
    source: msstore
```

| Field | Meaning |
| --- | --- |
| `id` | WinGet package identifier (`Publisher.Package`, or a Store product id) |
| `source` | Source to resolve it from; `winget` (the default) or `msstore` |
| `arguments` | Installer arguments to pass when installing |

How it is applied: entries are grouped by source, written to a temporary import file in
WinGet's own packages schema, and installed with a single `winget import` call
(`--ignore-unavailable --no-upgrade --disable-interactivity`, agreements pre-accepted).
Packages that are already installed are reported and skipped.

`export` fills this section from `winget export`, which is why Store packages and custom
installer arguments survive a round trip: the source details and
`InitialOverrideArguments` in WinGet's output are preserved in the YAML.

## `windows`

```yaml
windows:
  dark_mode: true
  show_file_extensions: true
  show_hidden_files: false
  developer_mode: true
```

| Setting | Registry value | Meaning |
| --- | --- | --- |
| `dark_mode` | `HKCU\...\Themes\Personalize` `AppsUseLightTheme` (and `SystemUsesLightTheme`) | Both are written together so apps and shell agree |
| `show_file_extensions` | `HKCU\...\Explorer\Advanced` `HideFileExt` | Inverted: the registry stores "hide", the manifest stores what you see |
| `show_hidden_files` | `HKCU\...\Explorer\Advanced` `Hidden` | `1` shows, `2` hides |
| `developer_mode` | `HKLM\...\AppModelUnlock` `AllowDevelopmentWithoutDevLicense` | Needs administrator rights; without elevation the error says so |

Any key you omit is left alone - `export` only writes the settings it could actually read,
so a machine that has never had developer mode set produces no `developer_mode` line rather
than an assumption. After applying, the change is broadcast so Explorer and running shells
notice; Explorer may still need a restart before icons update.

## `environment`

```yaml
environment:
  variables:
    EDITOR: code
    DOTNET_CLI_TELEMETRY_OPTOUT: "1"
  path:
    - "%USERPROFILE%\.cargo\bin"
```

- `variables` are user-scope variables written to `HKCU\Environment`. A value containing
  `%VARIABLES%` is stored as `REG_EXPAND_SZ` so it stays portable; anything else is
  `REG_SZ`.
- `path` entries are merged into your user `PATH`: entries already present (compared
  case-insensitively, ignoring separator style and expanding variables) are left as they
  are, missing ones are appended. Nothing is ever removed or reordered.
- `export` leaves out variables Windows creates by itself (TEMP, OneDrive, PSModulePath and
  friends) and the default `WindowsApps` PATH entry, because a fresh install already has
  them.
- Machine-scope variables are not captured or applied yet - see `docs/roadmap.md`.

After a change, the tool broadcasts a settings change so new shells pick it up. Terminals
that are already open keep their old environment; open a new one.

## `files`

```yaml
files:
  copy:
    - source: files/gitconfig
      destination: '%USERPROFILE%\.gitconfig'
```

| Field | Meaning |
| --- | --- |
| `source` | Path inside the bundle, or an absolute path |
| `destination` | Where it goes on this machine; `%VARIABLES%` and a leading `~/` are expanded |

`export` captures a known list when the file exists, and copies it into `files/`:

`.gitconfig`, `.gitignore_global`, `.ssh/config`, `.wslconfig`, `.npmrc`, `.vimrc`,
`.editorconfig`, the PowerShell 5.1 and 7 profiles, and VS Code's `settings.json` and
`keybindings.json`. Anything else goes in with `--include <path>`, which infers a
`%USERPROFILE%\...` destination for files under your home directory and keeps absolute paths
otherwise.

Applying copies only when the destination is missing or different, so a second run reports
"unchanged". Parent directories are created as needed.

## Path expansion

Wherever a path is written by hand - `destination`, `environment.path` entries - these are
expanded:

- `%USERPROFILE%`, `%APPDATA%` and any other environment variable, case-insensitively. An
  unknown `%NAME%` is left as literal text rather than blanked out.
- A leading `~/` or `~` becomes your user profile.

File `source` entries are resolved relative to the bundle folder unless they are absolute.

## Validation

`setup` loads and validates before touching anything:

- Unknown fields anywhere in the file are an error naming the field, the line and the keys
  that would have been accepted.
- `version` newer than the binary supports is refused.
- An app entry without an id, or a file entry without a source or destination, is refused.

Run `setup --dry-run` to see exactly what a manifest would do without applying it.

## Dry runs

| Component | `--dry-run` behaviour |
| --- | --- |
| `windows` | Prints the setting transitions, writes nothing, does not broadcast |
| `environment` | Prints the variables to set and PATH entries to add, writes nothing |
| `files` | Prints each copy (or "unchanged"), copies nothing |
| `apps` | Lists the packages it would install, runs no install |
