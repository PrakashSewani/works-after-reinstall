# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Nothing has been tagged yet; everything below is the work that will become 0.1.0.

### Added

- `export` captures a machine into a portable bundle: WinGet packages (with Microsoft Store
  packages and custom installer arguments preserved), Windows preferences (dark mode, file
  extensions, hidden files, developer mode), user environment variables and PATH entries,
  and known configuration files, plus anything passed to `--include`.
- `setup` applies a bundle: `--dry-run` to preview, `--only` to scope, idempotent applies,
  per-component failure reporting, and bundle discovery so a missing path is not a dead end.
- `starter` sets up a fresh machine with no bundle: an interactive catalogue of 48 verified
  packages in 7 categories, with optional sensible Windows preferences and environment
  defaults, plus a `starter` command that scaffolds an editable bundle with example
  configuration files.
- Interactive interface (default when the exe runs with no arguments): home, catalogue,
  export, setup and live-log screens, with scrolling and mouse support.
- `docs/` with architecture, manifest reference, roadmap, contributing and publishing guides.

[Unreleased]: https://github.com/PrakashSewani/works-after-reinstall/commits/dev
