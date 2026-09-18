# Works After Reinstall

> Your Windows setup, rebuilt after every reinstall.

Because reinstalling Windows is easy. Recreating your environment isn't.

**Works After Reinstall** is a Windows machine provisioning tool that lets you capture your preferred applications, Windows settings, environment variables, and configuration files from the machine you have — then rebuild that setup after a fresh Windows installation.

```text
Your machine today                     Fresh Windows
     ↓                                      ↓
works-after-reinstall export      →    works-after-reinstall setup
     ↓                                      ↓
┌─────────────────────────┐            ┌─────────────────────────┐
│ Applications            │            │ Applications            │
│ Windows Settings        │            │ Windows Settings        │
│ Environment Variables   │            │ Environment Variables   │
│ Configuration Files     │            │ Configuration Files     │
└─────────────────────────┘            └─────────────────────────┘
```

## Two ways to use it

Run the exe with no arguments and you get an interactive interface — arrow keys, checkboxes, live progress, nothing to remember:

```powershell
works-after-reinstall
```

Everything is also available as plain commands, for scripting or for machines where you would rather not run a full-screen interface:

```powershell
works-after-reinstall export
works-after-reinstall setup D:\bundle
```

## Usage

### On a fresh Windows install

Nothing needs to have been exported first. Run the exe and pick what you want:

```text
 > Start fresh - pick what to install
   Setup from a bundle
   Export this machine
   Quit
```

The first option is a catalogue of common apps grouped by category. Tick the ones you want, decide whether to also apply a few sensible Windows preferences and environment variables, and it installs them — no bundle, no config file, nothing to write first. `works-after-reinstall starter --list` prints the same catalogue if you would rather work from the command line.

If you prefer a file you can read and edit, `works-after-reinstall starter` writes a starter bundle: a commented manifest plus example `.gitconfig`, VS Code settings and PowerShell profile in `files/`. Fill in your details, uncomment what you want, and apply it with `setup`.

### Before reinstalling Windows

Capture this machine:

```powershell
works-after-reinstall export
```

That writes a portable bundle you can carry to the new install:

```text
works-after-reinstall-bundle/
├── works-after-reinstall.yaml   # what to restore, editable by hand
└── files/                       # copies of your configuration files
```

Keep it somewhere you can reach from a fresh install — USB drive, cloud storage, another machine.

After reinstalling Windows, put the exe next to the bundle and run:

```powershell
works-after-reinstall setup .\works-after-reinstall-bundle
```

Run `setup --dry-run` first if you want to see what it would change without touching anything.

## Commands

### `export`

| Option | Meaning |
| --- | --- |
| `-o, --output <DIR>` | Where to write the bundle (default `works-after-reinstall-bundle`) |
| `-i, --include <PATH>` | Extra file to capture (repeatable) |
| `--skip <COMPONENT>` | Leave out `apps`, `windows`, `environment` or `files` (repeatable) |

### `setup`

| Option | Meaning |
| --- | --- |
| `<BUNDLE>` | Bundle directory or manifest file to apply; when omitted, nearby bundles are searched for |
| `--dry-run` | Show what would change, change nothing |
| `--only <COMPONENT>` | Apply only `apps`, `windows`, `environment` or `files` (repeatable) |

No bundle is not a dead end. `setup` looks for one next to the exe, in the current folder, on your Desktop and in Downloads. If it finds exactly one, it uses it; if it finds several, it asks which; and if it finds none on an interactive terminal, it offers to capture this machine right then instead of exiting with an error.

### `starter`

| Option | Meaning |
| --- | --- |
| `<DIR>` | Folder to write the starter bundle into (default `works-after-reinstall-bundle`) |
| `--force` | Overwrite an existing bundle in that folder |
| `--list` | Print the app catalogue instead of writing a bundle |

The bundle it writes is meant to be read: a starter set of apps, a few preferences, and three example files under `files/` waiting for your details before you uncomment the matching entries.

### `ui`

Opens the interactive interface explicitly. Running the exe with no arguments does the same thing when there is a terminal available.

## What it captures

* 📦 **Applications** — `winget export`, including Microsoft Store packages and the installer arguments a package was originally installed with
* ⚙️ **Windows Settings** — dark mode, file extensions, hidden files, developer mode
* 🌱 **Environment** — user-scope variables and PATH entries, with Windows' own defaults filtered out
* 📁 **Files & Config** — known dotfiles (`.gitconfig`, `.ssh/config`, `.wslconfig`, PowerShell profiles, VS Code settings, …) plus anything you pass to `--include`

Setup is idempotent: running it again leaves already-correct settings, PATH entries and files alone.

## The manifest

`works-after-reinstall.yaml` is a plain, hand-editable file. Anything you delete from it simply isn't applied.

```yaml
version: 1
exported_at: 2026-09-18T18:54:27Z
source_machine: WORKSTATION

apps:
  - Google.Chrome
  - Microsoft.VisualStudioCode
  - id: Microsoft.VisualStudio.2022.BuildTools
    arguments: "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools"
  - id: XP89DCGQ3K6VLD
    source: msstore

windows:
  dark_mode: true
  show_file_extensions: true
  show_hidden_files: true
  developer_mode: true

environment:
  variables:
    EDITOR: code
    NODE_ENV: development
  path:
    - "%USERPROFILE%\\.cargo\\bin"

files:
  copy:
    - source: "files/gitconfig"
      destination: "%USERPROFILE%\\.gitconfig"
```

An app is either a bare package id or a mapping when it needs extra detail:

| Field | Meaning |
| --- | --- |
| `id` | WinGet package identifier |
| `source` | WinGet source to resolve it from, e.g. `msstore` (defaults to `winget`) |
| `arguments` | Installer arguments to use when installing |

Paths accept `%VARIABLES%` and a leading `~/`, and file `source` entries are relative to the bundle.

## How it works

- **No PowerShell, no scripts.** Settings go straight to the registry through the Windows API, so nothing depends on a shell being present or a policy allowing it.
- **WinGet does the installing.** `export` reads WinGet's package list and `setup` feeds an equivalent file back to `winget import` — Microsoft Store packages and the installer arguments an app was originally installed with included.
- **One file, no runtime.** A release build is a single ~1.7 MB exe with no installer and no dependencies, which is what makes it usable on a machine that has nothing on it yet.
- **Idempotent by design.** Every apply compares before it writes, merges instead of replacing, and tells you what it left alone.

The module map, flows and the reasoning behind those choices are in [docs/architecture.md](docs/architecture.md).

## Documentation

| Document | What it covers |
| --- | --- |
| [docs/architecture.md](docs/architecture.md) | Module map, the two flows, design trade-offs |
| [docs/manifest.md](docs/manifest.md) | Every manifest field, path expansion, dry-run behaviour |
| [docs/roadmap.md](docs/roadmap.md) | What is planned next and why |
| [docs/contributing.md](docs/contributing.md) | Dev loop, testing expectations, release checklist |
| [docs/publishing.md](docs/publishing.md) | How the tool gets published, including the winget package plan |
| [CHANGELOG.md](CHANGELOG.md) | What changed |

## Requirements

* Windows 10 or 11
* `winget` (App Installer) for the `apps` component — present by default on Windows 11
* Administrator rights for `developer_mode`, which lives in `HKLM`
* A real terminal (Windows Terminal, PowerShell) for the interactive interface; the commands themselves also work non-interactively

## Build

```powershell
cargo build --release
```

The result is a single self-contained executable at `target\release\works-after-reinstall.exe` — no runtime to install, which is the point when the machine in front of you is fresh.

## Status

🚧 **Early development**

The goal is simple:

**Reinstall Windows. Run one command. Get your machine back.**

What is coming next — reboot resume, machine-scope environment, single-file bundles — is in [docs/roadmap.md](docs/roadmap.md).

## License

MIT
