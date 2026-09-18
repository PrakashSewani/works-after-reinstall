# Command snippets

Replace `<bundle>` with the bundle folder (or with `%COMMANDCODE_SCRATCHPAD%\bundle` while
testing). Run from the repository root.

## Build and test

```powershell
cargo fmt
cargo clippy --all-targets
cargo test
cargo build --release

# when a running exe holds target\debug\works-after-reinstall.exe
cargo build --target-dir $env:TEMP\war-target
```

## Is a stale instance running?

```powershell
Get-Process works-after-reinstall -ErrorAction SilentlyContinue | Select-Object Id, StartTime
```

## Dry run, twice

```powershell
target\release\works-after-reinstall.exe setup <bundle> --dry-run
target\release\works-after-reinstall.exe setup <bundle> --dry-run   # everything "unchanged"
```

## Scope a run

```powershell
target\release\works-after-reinstall.exe setup <bundle> --only files
target\release\works-after-reinstall.exe setup <bundle> --only environment
target\release\works-after-reinstall.exe setup <bundle> --only apps --dry-run
```

## Export without touching the user's folder

```powershell
target\release\works-after-reinstall.exe export -o "%COMMANDCODE_SCRATCHPAD%\bundle"
```

## A throwaway env var, then cleanup

```powershell
reg query "HKCU\Environment" /v WAR_IMPORT_TEST
reg delete "HKCU\Environment" /v WAR_IMPORT_TEST /f
```

## Winget sanity checks

```powershell
winget --version
winget search --id Git.Git -e --accept-source-agreements --disable-interactivity
```

## Registry spot checks (read-only)

```powershell
reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced" /v HideFileExt
reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize" /v AppsUseLightTheme
reg query "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock" /v AllowDevelopmentWithoutDevLicense
```

## Scratchpad

Every shell has the session scratchpad path in `COMMANDCODE_SCRATCHPAD`
(`%COMMANDCODE_SCRATCHPAD%` in cmd, `$env:COMMANDCODE_SCRATCHPAD` in PowerShell). Use it for
throwaway bundles, test destinations and export output so nothing lands in the repository or
in the user's profile.
