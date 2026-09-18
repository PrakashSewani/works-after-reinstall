# Contributing

## Before you start

- Windows 10 or 11 and a Rust stable toolchain with the MSVC target
  (`rustup default stable-x86_64-pc-windows-msvc`).
- `winget` for anything touching the `apps` component. Windows 11 has it; on Windows 10 you
  may need App Installer from the Store.
- An elevated terminal only if you plan to test `developer_mode`, which lives in `HKLM`.

## Dev loop

```powershell
cargo fmt
cargo clippy --all-targets
cargo test
cargo build --release
```

All four should be clean before you call something done. `cargo clippy --all-targets` covers
the test code too, which is where most of the style slips happen.

**The exe locks itself.** A running `works-after-reinstall.exe` holds
`target\debug\works-after-reinstall.exe` open, and the next build fails with
`Access is denied`. Close the instance, or build somewhere else:

```powershell
cargo build --target-dir $env:TEMP\war-target
```

If you are unsure whether a stale instance is the problem:

```powershell
Get-Process works-after-reinstall -ErrorAction SilentlyContinue
```

## Where to make a change

Agent skills in `.commandcode/skills/` are the detailed playbooks and are worth reading even
if you are not an agent:

| Change | Skill |
| --- | --- |
| A new kind of thing to capture or restore | `provisioning-component` |
| Adding or fixing a package in the Start fresh catalogue | `starter-catalog` |
| Adding a screen, key binding or scrolling behaviour | `tui-screen` |
| Verifying something against a real machine | `windows-verify` |

`docs/architecture.md` explains how the pieces fit; `docs/manifest.md` is the schema
contract.

## Testing

- Pure logic gets unit tests next to it (`paths`, `manifest`, `envvars`, `files`, `catalog`,
  `starter`, `winget` parsing). These run anywhere.
- TUI work is tested by rendering a screen to ratatui's `TestBackend` and asserting on the
  buffer, plus key-driven state transitions. See the tests at the bottom of `src/tui.rs`.
- **Tests must never mutate real machine state.** Registry writes, environment changes and
  file copies in tests are a review blocker. The ladder in the `windows-verify` skill covers
  how to verify a write path for real without leaving anything behind.
- Apply paths are expected to be exercised at least once through `--dry-run`, which is the
  only way to see the full plan without touching the machine.

## Documentation to update

| If you change | Update |
| --- | --- |
| Anything a user can observe | `CHANGELOG.md` under `## [Unreleased]` |
| The manifest schema | `docs/manifest.md` and the example in `README.md` (a test parses it) |
| CLI flags or commands | the command tables in `README.md` |
| Module boundaries or a design decision | `docs/architecture.md` |
| What is planned | `docs/roadmap.md` |

## Commits and releases

- Commit subjects are short and imperative ("add machine-scope environment capture"), with a
  body when the *why* is not obvious from the change.
- Keep a change and its docs in the same commit; a behaviour change with a stale README is
  worse than either alone.
- Release checklist: bump `version` in `Cargo.toml`, move the `Unreleased` section in
  `CHANGELOG.md` under the new version with a date, build `--release`, tag, and attach the
  exe. There is no release automation yet; the plan for the workflow and for the winget
  package is [publishing.md](publishing.md).

## Style notes

- Console and TUI output is ASCII only: legacy consoles run codepage 437 and box-drawing or
  accented characters come out as mojibake.
- Error messages say what to do next ("run this from a terminal opened as Administrator"),
  not just what went wrong.
- Prefer merging over replacing: if a change could drop something the user had, it needs a
  story for why it cannot.
