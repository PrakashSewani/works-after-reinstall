# Works After Reinstall

A single-exe Windows provisioning tool. `export` captures this machine into a portable
bundle; `setup` re-applies a bundle; `starter` sets a fresh machine up from a built-in
catalogue with no bundle at all. Rust, Windows-only, nothing to install alongside the exe.

Running the exe with no arguments opens the interactive interface; every flow is also a
plain command.

## Layout

| File | Responsibility |
| --- | --- |
| `src/main.rs` | Argument dispatch and module declarations; no logic |
| `src/cli.rs` | The `clap` surface, including the `Component` enum (`label`, `title`, `ALL`) |
| `src/commands.rs` | The flows: `export`, `setup`, `apply_manifest` |
| `src/console.rs` | Where progress output goes - stdout for the CLI, a channel for the TUI |
| `src/manifest.rs` | The YAML contract: `Manifest`, `WindowsSettings`, `Environment`, `Files`, `App` |
| `src/settings.rs` | Windows preferences via HKCU/HKLM, plus the settings-change broadcast |
| `src/envvars.rs` | `HKCU\Environment` variables and PATH merging |
| `src/files.rs` | Known dotfiles plus `--include`, copied in and out of the bundle |
| `src/winget.rs` | `winget export`/`import`, preserving source and installer arguments |
| `src/catalog.rs` | The starter catalogue (categories, verified ids) and `starter_manifest` |
| `src/starter.rs` | Scaffolds an editable starter bundle |
| `src/discovery.rs` | Finds bundles nearby and resolves what `setup` should apply |
| `src/tui.rs` | The interactive interface: screens, key handling, rendering |
| `src/prompt.rs` | Interactive yes/no and numbered-choice prompts |
| `src/paths.rs` | `%VAR%` / `~` expansion, PATH entry helpers, bundle resolution |
| `src/win.rs` | Win32 bits: elevation check, `WM_SETTINGCHANGE` broadcast |
| `src/util.rs` | Timestamps |

## Invariants

- **Output goes through `Console`.** A `println!` inside a component is invisible in the TUI.
  Component functions take `&Console` and call `console.line(..)` / `console.section(..)`.
- **Every write path honours `dry_run`** and returns before mutating anything.
- **Apply is idempotent.** Compare first, skip when the value or file already matches;
  running `setup` twice must report "unchanged", not rewrite.
- **Never replace, always merge.** PATH entries are appended and de-duplicated, unknown
  registry values are left alone, files are only copied when the contents differ.
- **Apply order is windows -> environment -> files -> apps** - fast local things first, the
  long winget run last. A component that fails is collected and reported; the rest still run.
- **The manifest is a user-facing contract.** `deny_unknown_fields` stays on so typos fail
  loudly, new fields need `#[serde(default, skip_serializing_if = ...)]`, and the README
  example must keep parsing (there is a test for exactly that).
- **ASCII only in console and TUI text** - legacy consoles run codepage 437.
- **Tests never mutate real machine state.** The `windows-verify` skill has the ladder.

## Commands

```powershell
cargo fmt
cargo clippy --all-targets
cargo test
cargo build --release      # target\release\works-after-reinstall.exe, about 1.7 MB
```

A running `works-after-reinstall.exe` locks `target\debug\works-after-reinstall.exe` and
breaks the next build. Close it, or build somewhere else:
`cargo build --target-dir $env:TEMP\war-target`.

## Skills

- `provisioning-component` - adding a new kind of thing to capture or restore.
- `starter-catalog` - adding or fixing apps in the Start fresh catalogue.
- `tui-screen` - any change to the interactive interface.
- `windows-verify` - verifying changes that touch the registry, PATH, winget or files.

## Long-form docs

Read the matching doc before changing the area it covers:

- `docs/architecture.md` - module boundaries, flows, design trade-offs
- `docs/manifest.md` - the manifest schema, which is a user-facing contract
- `docs/roadmap.md` - what is planned next and why
- `docs/publishing.md` - the winget publishing plan and release runbook
- `docs/contributing.md` - dev loop, release, which docs to update with a change
- `CHANGELOG.md` - add an entry whenever behaviour changes
