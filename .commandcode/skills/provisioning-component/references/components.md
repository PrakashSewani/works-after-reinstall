# The four components as they exist today

Copy the closest shape. Every path below is relative to the repository root.

| | `apps` | `windows` | `environment` | `files` |
| --- | --- | --- | --- | --- |
| Manifest type | `Manifest.apps: Vec<App>` (untagged enum: `App::Id` / `App::Detailed`) | `WindowsSettings` (all `Option<bool>`) | `Environment { variables: BTreeMap, path: Vec<String> }` | `Files { copy: Vec<FileCopy> }` |
| Capture | `winget::export()` -> `parse_apps` | `settings::capture()` -> `read_dword` | `envvars::capture()` -> `decode` | `files::capture(bundle, include)` -> `KNOWN` |
| Apply | `winget::apply(apps, dry_run, console)` | `settings::apply(settings, dry_run, console)` | `envvars::apply(variables, additions, dry_run, console)` | `files::apply(bundle, copies, dry_run, console)` |
| State touched | temp import file, `winget import` | `HKCU`/`HKLM` DWORDs | `HKCU\Environment` values | files under the bundle and on disk |
| Idempotency | `--no-upgrade`, winget reports "already installed" | compare value, print "unchanged" | compare value, print "already set" | compare contents, print "unchanged" |
| Broadcast | - | `broadcast_setting_change(ADVANCED)` | `broadcast_setting_change("Environment")` | - |
| Tests | `winget::tests` (parsing, import file shape) | `settings::tests` (summary formatting) | `envvars::tests` (encode/decode, filters, merge) | `files::tests` (destinations, collisions) |

## Details worth copying

**Capture is allowed to be partial.** `settings::capture` returns `Option<bool>` per key and
`read_dword` returns `None` for a missing value; `files::capture` skips known files that are
not present. Nothing invents state.

**Apply reads before it writes.** `envvars::apply` calls `get_raw_value` and compares before
setting; `files::apply` compares contents before copying. That is what makes a second run
print "unchanged" instead of rewriting.

**Value types are preserved deliberately.** `envvars::encode_as` keeps `REG_EXPAND_SZ` for
values containing `%VARS%` and reuses the type found on the existing PATH value, because
turning an expandable value into a literal one breaks portability.

**Merging beats replacing.** `envvars::apply` appends missing PATH entries and leaves the
rest in order; the default `WindowsApps` entry is filtered out of a capture because every
fresh install already has it.

**Errors are collected, not fatal.** `commands::apply_manifest` pushes `{component}: {error}`
into `failures` and keeps going. Anything you add must do the same.
