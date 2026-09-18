# Roadmap

What exists, what is next and why. Anything here is open to argument - the point is that the
reasoning is written down, not that the order is fixed.

## Shipped

| Area | What works today |
| --- | --- |
| Capture | `export` writes a bundle: WinGet packages (with source and installer arguments), Windows preferences, user environment variables and PATH, known config files plus `--include` |
| Restore | `setup` applies a bundle: bundle discovery when no path is given, `--dry-run`, `--only`, idempotent applies, failures collected per component |
| Start fresh | `starter` catalogue in the TUI (48 verified packages across 7 categories) plus `starter` on the command line, which scaffolds an editable bundle with example configs |
| Interface | Interactive TUI with a live log, scrolling and mouse support; every flow also a plain command for scripting |
| Quality | 92 unit tests, clippy clean, single self-contained ~1.7 MB exe |

## Next

Ordered by how much they hurt without them.

1. **Resume after a reboot.** Installing packages and changing Explorer settings sometimes
   needs a restart; right now the run ends with "some changes may need a restart" and you
   have to work out what is left. Plan: write a small state file next to the manifest as
   each component completes, and have `setup` skip finished work on the next run. A
   `RunOnce` entry would let Windows continue by itself after the reboot.
2. **Machine-scope environment.** Only user variables and user PATH are captured. Machine
   PATH is where system-wide tool installs land, so a restored machine can be missing
   entries. It needs elevation and a merge that provably never drops a missing system
   entry - which is why it was not rushed in.
3. **Single-file bundle.** A folder is awkward to email or drop in cloud storage; `export
   --zip` and `setup bundle.zip` (unpacked to a temp folder) would make the bundle one file.
   Needs a zip dependency and a decision about where to unpack.
4. **`--json` output.** A machine-readable plan/result so the tool can be driven by scripts
   and CI, not just by people reading console text.
5. **More Windows settings.** Taskbar alignment, Explorer tweaks, terminal defaults,
   per-monitor scaling. File associations are the popular request and the riskiest - they
   change system state other apps rely on, so they need their own design.
6. **Publish to winget.** Two halves: a tag-triggered release workflow that attaches the exe
   and a SHA256 file, and the manifest submission that makes
   `winget install PrakashSewani.WorksAfterReinstall` work - which also needs an icon in the
   exe. The runbook, including what the validation pipeline checks and the traps, is in
   [publishing.md](publishing.md). Code signing later, once there is a certificate story.
7. **Verify mode.** `setup --check` reporting how far the machine has drifted from the
   manifest, changing nothing. Useful before a reinstall ("is my bundle still accurate?")
   and after one ("did everything land?").

## Later

- Driver-style installs (NVIDIA and friends are not reliably in WinGet).
- WSL distro install and import, which is its own capture/restore problem.
- Unattended setup for lab machines: an answer file that picks catalogue items and skips
  prompts entirely.
- Post-install hooks per package - powerful, and needs a story about running arbitrary
  scripts safely before it can be offered.
- A richer set of starter config templates beyond the three that ship today.

## Not planned

- **Storing secrets, tokens or licences.** The bundle is plain YAML on a USB stick; sign-in
  state and licences belong to the apps that own them.
- **Replacing WinGet.** Sources, versions and installers are WinGet's job; this tool decides
  *what* should be installed and hands the list over.
- **Non-Windows platforms.** The value here is registry, Explorer and WinGet semantics.
- **Cloning app state.** Preferences inside third-party apps are only capturable where the
  app stores them in a file we can copy.

## Known gaps

- `developer_mode` needs an elevated terminal; the failure message says so, but the tool
  will not elevate itself.
- Store package ids only come from an export - the catalogue is WinGet packages only.
- Bundles are unencrypted and unversioned beyond the `version` field in the manifest.
