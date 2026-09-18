---
name: provisioning-component
description: Add or extend a managed component - applications, Windows settings, environment variables or files - across the manifest, CLI, TUI, docs and tests. Use when asked to capture or restore a new kind of thing, add a Windows setting, or when editing src/settings.rs, src/envvars.rs, src/files.rs, src/winget.rs or src/manifest.rs.
---

# Adding a component

A component is one of the four bundle sections: `apps`, `windows`, `environment`, `files`.
Decide which job you are doing before touching anything:

- **A new key inside an existing component** (another Windows preference, another variable
  rule): steps 1, 2, 6, 7.
- **A whole new component**: every step.

## 1. The manifest (src/manifest.rs)

- Adding a key: put it on the section struct (`WindowsSettings`, `Environment`, `Files`) with
  `#[serde(default, skip_serializing_if = "...")]`, so an unset key is never written.
- A key that is an `Option<T>` uses `skip_serializing_if = "Option::is_none"`; a collection
  uses the matching `is_empty`.
- Update that struct's `is_empty()` - if the new field is missing from it, an untouched
  section starts serialising as an empty block.
- `deny_unknown_fields` stays on every struct. Do not soften it to make parsing easier; a
  typo in a hand-edited manifest must fail loudly.
- A new component needs a field on `Manifest`, and the README example test must still parse.

## 2. Capture and apply

Work in the module that owns the area, following the functions already there:

- `capture()` reads only, and never fails hard on a missing value - return `None` and skip,
  so a machine that never had the setting does not invent one.
- `apply(.., dry_run, console)` follows this shape, in order:
  1. return early when `dry_run`, after printing what would change;
  2. read the current value, compare, print "unchanged" / "already set" and skip when equal;
  3. mutate only what differs, merging rather than replacing (append and de-duplicate lists,
     leave unknown values untouched);
  4. broadcast with `win::broadcast_setting_change(..)` when something changed and the area
     is one other processes cache.
- Progress goes through the `&Console`, never `println!` - a `println!` is invisible in the
  TUI and the change will look broken there.
- Map permission failures to an actionable message: `... needs administrator rights - run
  this from a terminal opened as Administrator`.

## 3. The component list (src/cli.rs)

`Component::ALL` is what the CLI, the TUI and the apply loop all iterate. A new component
needs a variant, its position in `ALL` matching apply order, `label()` (the lowercase CLI
value used by `--only` and `--skip`) and `title()` (the human name shown in the TUI).

## 4. Apply order (src/commands.rs)

`apply_manifest` runs windows -> environment -> files -> apps. Put anything new before
`apps`: the winget run takes minutes and every cheap, local change should land first. Keep
the failure-collection structure - a component that fails is recorded in `failures` and the
remaining components still run.

## 5. The interface (src/tui.rs)

- `Component::ALL` gives the export and setup screens one row per component, but
  `EXPORT_ROWS` / `SETUP_ROWS` count focusable rows and `component_row(row)` maps a row index
  back to a component index. Change them together or the highlight lands on the wrong row.
- Toggle state lives in `export_enabled` / `setup_enabled`, typed `[bool; 4]` to the size of
  `Component::ALL`. Bump the size.
- A new component also needs a line in the catalog/starter rows only if it is offered there.

## 6. Tests

- Unit tests in the module you changed, for the pure helpers and for the "already correct"
  path (that is the idempotency guarantee).
- A round-trip test proving the new manifest field survives `serde_yaml` both ways.
- Nothing may touch the real registry, environment or installed apps - see the
  `windows-verify` skill for how to check a write path safely.
- `cargo fmt && cargo clippy --all-targets && cargo test` all clean.

## 7. Docs

- `README.md` - the "What it captures" bullet, and the manifest example if the schema moved.
- `docs/manifest.md` - the new key: what it maps to, what "unchanged" means, dry-run
  behaviour.
- `docs/architecture.md` if the module map or a design decision changed.
- `CHANGELOG.md` under `## [Unreleased]`.

## Worked example

[references/components.md](references/components.md) maps every existing component to its
symbols, so you can copy the shape of whichever one is closest to what you are adding.
