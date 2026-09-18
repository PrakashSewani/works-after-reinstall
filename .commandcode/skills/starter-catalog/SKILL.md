---
name: starter-catalog
description: Add, verify or fix applications in the Start fresh catalogue, or add a category to it. Use when adding a package to the starter list, fixing a package id that does not resolve, or when editing src/catalog.rs.
---

# Extending the starter catalogue

The catalogue is **data, not policy**: a list of verified WinGet ids grouped by category.
Anything that needs per-app logic (custom installer arguments, a different source) does not
belong here - that is what `App::Detailed` in a hand-written manifest is for.

## 1. Verify the id first

Never add an id from memory. Check it against the live WinGet source:

```powershell
winget search --id <Candidate.Id> -e --accept-source-agreements --disable-interactivity
```

The output must show that exact id. If nothing comes back, find the real one by name:

```powershell
winget search "<app name>" --accept-source-agreements --disable-interactivity
```

Titles are similar across entries; pick the one from the publisher that makes the app. State
in your summary which ids you checked and what the command returned.

## 2. Add the entry (src/catalog.rs)

```rust
CatalogApp {
    category: "Utilities",
    name: "PowerToys",
    id: "Microsoft.PowerToys",
    note: "Microsoft's power user tools",
},
```

- Append inside the right category block: the list renders in file order, so position is the
  order people see.
- `name` is what a person calls the app, not the id. Keep it short - it shares one line with
  the note.
- `note` helps someone decide (what the app is for, or a caveat like "large download"). Use
  `""` when the name is enough. Lowercase, no trailing period.
- Keep the struct order `category, name, id, note` so the file stays scannable.

## 3. Categories

`category` must be one of `CATEGORIES` in the same file. A new category goes into
`CATEGORIES` in the position it should appear in the TUI - headers change once, when the
category changes, so the list stays grouped.

## 4. Tests

```powershell
cargo test catalog
```

The tests enforce what a reviewer would otherwise have to check by eye: unique ids, every
category present in `CATEGORIES` and non-empty, no whitespace in an id, no empty names. If
you add a test for something new, make it a property of the catalogue rather than a fact
about one app.

## 5. Eyeball it

```powershell
cargo run -- starter --list          # the plain-text listing
cargo run                            # the TUI: Start fresh
```

The TUI list is scrollable; check the new row fits on a 90x24 terminal without wrapping.

## 6. Docs

A few additions do not need a changelog line; a category or a meaningful batch does.
`README.md` mentions the catalogue size in places - keep the number honest
(`catalog::APPS.len()`), and `docs/roadmap.md` mentions the catalogue only if the shape
changed.
