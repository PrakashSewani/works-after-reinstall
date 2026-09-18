---
name: windows-verify
description: Verify a change against a real Windows machine without leaving anything behind. Use when a change touches the registry, environment variables, PATH, WinGet or file copies, or before claiming that such a change works.
---

# Verifying against a real machine

The unit tests prove logic. They cannot prove that a registry write lands, that Explorer
picks a setting up, or that a copied file is where a user expects it. This is the ladder for
checking those things for real - in order, stopping as soon as the level below already
settles the question.

## 1. Unit tests

```powershell
cargo test
```

Pure helpers, manifest round-trips, parsing and the "already correct" paths are covered here.
If the change is only logic, this level is enough.

## 2. Dry run

```powershell
cargo build --release
target\release\works-after-reinstall.exe setup <bundle> --dry-run
```

Read the plan line by line: it is what the real run will do. Then run it a second time - a
correct implementation reports "unchanged" / "already set" for everything it would have
considered done. Dry runs must leave the machine untouched, so anything you see change is a
bug in the dry-run guards.

## 3. Contained real writes

For file copies, point the destination somewhere disposable - the session scratchpad
(`%COMMANDCODE_SCRATCHPAD%` in cmd, `$env:COMMANDCODE_SCRATCHPAD` in PowerShell) - never at
the user's real dotfiles:

```yaml
files:
  copy:
    - source: files/hello.txt
      destination: "%COMMANDCODE_SCRATCHPAD%\\deep\\nested\\hello.txt"
```

Apply with `--only files`, check the contents, run it again and confirm it reports
"unchanged".

## 4. Device state: add, verify, remove

For an environment variable, use a throwaway name and clean up in the same session:

```powershell
# 1. a bundle with only a test variable
target\release\works-after-reinstall.exe setup <bundle> --only environment

# 2. it really landed
reg query "HKCU\Environment" /v WAR_IMPORT_TEST

# 3. remove it again
reg delete "HKCU\Environment" /v WAR_IMPORT_TEST /f

# 4. confirm it is gone (this query is *expected* to fail)
reg query "HKCU\Environment" /v WAR_IMPORT_TEST
```

Say in your summary that you did this and that the value was removed.

## 5. Never for real during verification

| Area | Why not |
| --- | --- |
| `HKCU` theme and Explorer settings (dark mode, file extensions, hidden files) | They change the user's desktop immediately |
| `HKLM` (developer mode) | Needs elevation and changes machine-wide state; verify with `--dry-run` only |
| Real WinGet installs | Use a bundle of packages that are already installed - with `--no-upgrade` the import is a no-op that still exercises the whole path |
| Machine-scope environment | Not captured yet, and it is machine-wide when it is |
| The user's real config files | Use scratchpad destinations |

## 6. Clean up and report

Remove the temporary bundles you created, delete any throwaway registry values, and close any
process you started. Then state plainly: what you ran, what it wrote, and what you removed.
"Verified" without that list is not verification.

## Traps

- **A running exe locks the build.** `Get-Process works-after-reinstall -ErrorAction
  SilentlyContinue` shows it; either close the instance or build elsewhere with
  `cargo build --target-dir $env:TEMP\war-target`.
- **`winget --version` is the cheap liveness check** for the apps component; a missing winget
  is a real failure on a fresh machine and the error message should say so.
- **The TUI needs a terminal.** Piped output makes `ui` refuse on purpose, so TUI paths are
  checked with the `TestBackend` tests rather than by running the exe.
- **Explorer caches.** File-extension and hidden-file changes are broadcast but Explorer may
  need a restart; do not read a stale icon as a failed write.

Exact command snippets for each level: [references/commands.md](references/commands.md).
