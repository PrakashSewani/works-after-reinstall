# Architecture

Works After Reinstall is a Windows-only Rust binary. It has two jobs that share one apply
engine: capture a machine into a bundle, and make a machine look like a bundle. A third
flow, *start fresh*, skips the bundle and applies a built-in catalogue instead.

Constraints that shape everything below:

- **One file.** The exe has to run on a machine that has nothing else installed, so there is
  no runtime dependency, no installer, no config directory. `cargo build --release`
  produces a ~1.7 MB self-contained binary.
- **It runs on someone else's fresh install.** Anything that needs a package manager, a
  shell profile or a helper script is out. Direct Windows APIs and `winget` are in.
- **Its output is the only feedback.** A half-applied machine is worse than an unchanged
  one, so every write is idempotent, every write path has a dry run, and one failing
  component never aborts the others.

## Flows

```text
export                                  setup
──────                                  ─────
capture apps      (winget export)       load + validate manifest
capture settings  (HKCU/HKLM reads)     [windows]     registry writes + broadcast
capture env       (HKCU\Environment)    [environment] variables, PATH merge + broadcast
capture files     (known list)          [files]       compare, then copy
write YAML + files/                     [apps]        winget import (long run, last)

starter (fresh machine, no bundle)
──────────────────────────────────
catalogue pick (in memory) -> same apply engine as setup
```

Entry points are `commands::export`, `commands::setup` and `commands::apply_manifest`. The
TUI and the CLI both call those three functions; there is no second implementation.

## Module map

| Module | Key symbols | Notes |
| --- | --- | --- |
| `main` | `run_default`, `run_export`, `run_setup`, `run_starter` | Parses args, picks stdout console, exits non-zero on error |
| `cli` | `Cli`, `Command`, `Component` | `Component::ALL` drives every UI that lists components |
| `commands` | `export`, `setup`, `apply_manifest` | Owns apply order and failure collection |
| `console` | `Console::{stdout, channel, line, blank, section}` | Cloneable; `Sender<String>` behind `Channel` |
| `manifest` | `Manifest`, `WindowsSettings`, `Environment`, `Files`, `FileCopy`, `App`, `AppDetail` | `load`/`save`/`validate` |
| `settings` | `capture`, `apply`, `summary` | Registry values described in `docs/manifest.md` |
| `envvars` | `capture`, `apply` | Registry encode/decode helpers live here |
| `files` | `capture`, `apply`, `destination_for` | Known-file list is `const KNOWN` |
| `winget` | `export`, `apply`, `parse_apps`, `import_json` | Source-aware import file builder |
| `catalog` | `APPS`, `CATEGORIES`, `StarterChoices`, `starter_manifest` | Static data, no I/O |
| `starter` | `scaffold`, `listing` | Writes the editable starter bundle |
| `discovery` | `find_bundles`, `resolve_requested_bundle`, `BundleChoice` | Search order: exe dir, cwd, Desktop, Downloads (+ one level) |
| `tui` | `run`, `App`, `Screen`, `Running`, `MouseCapture` | ratatui + crossterm |
| `prompt` | `is_interactive`, `confirm`, `choose` | Only used by the CLI paths |
| `paths` | `expand`, `expand_vars`, `same_path`, `split_path_entries`, `resolve_bundle` | Pure helpers, heavily unit tested |
| `win` | `is_admin`, `broadcast_setting_change` | Token-elevation check via winapi |
| `util` | `now_iso8601` | Local calendar maths, no chrono dependency |

## The Console indirection

Every user-visible line goes through `Console`:

```rust
pub enum Console {
    Stdout,                  // println!
    Channel(Sender<String>), // the TUI drains this into its log view
}
```

Component functions take `&Console` rather than printing directly. That is what lets the TUI
show live progress from code that was written for the command line - including `winget`,
whose piped output is pumped line by line into the same channel
(`winget::pump` splits on `\r` and `\n` and forwards each line).

**The rule:** a new component that prints with `println!` works in the CLI and silently
disappears in the TUI. Always take a `&Console`.

## The interactive interface

`tui::run` uses `ratatui::run`, which owns raw mode, the alternate screen and a panic hook
that restores the terminal. A `MouseCapture` guard enables wheel events and disables them on
drop, so a panic cannot leave the mouse captured.

State lives in one `App` struct: the current `Screen`, a focus index per screen, the bundle
list, per-component checkboxes, the log buffer, and the scroll offsets. Screens are
`Home`, `Starter`, `Export`, `Setup`, `Running`.

- **Event loop** - `App::tick(100ms)` polls for a key or wheel event, then drains the worker
  channels. Long work never runs on this thread.
- **Long work** - `App::run_job` spawns a thread with `Console::channel(..)`, stores the
  receivers in `Running`, and returns immediately. The job's result arrives as a
  `Result<(), String>` on a second channel.
- **Component lists** - `EXPORT_ROWS` / `SETUP_ROWS` are the number of focusable rows; when
  a row is added or removed, those constants and the `component_row` index mapping change
  together.
- **Scrolling** - `view_height` is a `Cell<usize>` filled in during `draw`, so key handling
  knows how much is visible. The flow log scrolls *bottom-relative* (`log_scroll` = lines
  back from the newest); new output arriving while you are reading back increments it so the
  line you were reading stays put. The starter list scrolls *top-relative*
  (`starter_scroll` + `starter_reveal`) and keeps the focused row visible.
- **The wheel means opposite things in the two views** - back through a log, down a list.
  `on_mouse` is where that asymmetry lives.

## Windows integration

- **Registry** through `winreg` (its 0.56 API: `RegValue<'a>` with `Cow<[u8]>` and the
  `RegType` enum). Values are encoded/decoded by hand in `envvars` because environment
  variables are `REG_SZ` / `REG_EXPAND_SZ` and PATH merging has to preserve which one it
  found.
- **`WM_SETTINGCHANGE`** is broadcast after environment or Explorer changes via
  `SendMessageTimeoutW(HWND_BROADCAST, ..)` so already-running shells and Explorer pick the
  change up. `is_admin` reads the process token's elevation state, which is only used to
  make a permission error explain itself.
- **winget** is driven as a child process. `winget export` writes a package file that is
  parsed into the manifest; import builds an equivalent file, grouped by source with
  `InitialOverrideArguments` preserved, and pipes output into the console while running.

## Decisions worth knowing

| Decision | Why |
| --- | --- |
| Direct registry APIs instead of PowerShell | No dependency on a shell being present or a policy allowing it, and errors surface as `io::Error` we can map to a useful message |
| `winget export`/`import` instead of `winget list` parsing | `list` output is a formatted table that breaks between versions; export is a schema we can also feed back to `import` |
| Keep per-package source and installer arguments | A restored `Microsoft.VisualStudio.2022.BuildTools` without its workload arguments is not the machine you had |
| YAML, not JSON, for the manifest | It is meant to be read and hand-edited; comments and the starter template depend on it |
| Bundle discovery instead of a required path | The post-reinstall case is "run the exe next to the bundle" - making people type the path is friction, and a missing bundle should suggest the next step, not exit |
| Starter catalogue with verified ids | The common case is a fresh machine with nothing exported; the catalogue makes the tool useful before any bundle exists |
| `deny_unknown_fields` on the manifest | A typo in a hand-edited config should be an error, not silently ignored |

## Invariants

See `AGENTS.md` for the list that must hold in every change. The short version: output goes
through `Console`, writes honour `dry_run`, applies are idempotent and merge rather than
replace, and tests never touch real machine state.
